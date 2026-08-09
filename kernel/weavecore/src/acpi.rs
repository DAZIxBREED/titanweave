use core::mem::size_of;
use core::ptr;

pub const MAX_CPUS: usize = 64;
const MAX_TABLE_SIZE: usize = 1024 * 1024;

#[derive(Clone, Copy)]
pub struct PlatformInfo {
    pub local_apic_address: u64,
    pub apic_ids: [u32; MAX_CPUS],
    pub cpu_count: usize,
}

impl PlatformInfo {
    const fn empty() -> Self {
        Self {
            local_apic_address: 0,
            apic_ids: [0; MAX_CPUS],
            cpu_count: 0,
        }
    }

    fn push_apic_id(&mut self, apic_id: u32) {
        if self.cpu_count >= MAX_CPUS || self.apic_ids[..self.cpu_count].contains(&apic_id) {
            return;
        }
        self.apic_ids[self.cpu_count] = apic_id;
        self.cpu_count += 1;
    }

    #[must_use]
    pub fn logical_index_for_apic(&self, apic_id: u32) -> Option<usize> {
        self.apic_ids[..self.cpu_count]
            .iter()
            .position(|candidate| *candidate == apic_id)
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RsdpV1 {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RsdpV2 {
    v1: RsdpV1,
    pub length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct SdtHeader {
    pub signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

pub fn discover(rsdp_address: u64, identity_map_limit: u64) -> Result<PlatformInfo, &'static str> {
    if rsdp_address == 0 || rsdp_address >= identity_map_limit {
        return Err("RSDP lies outside the bootstrap identity map");
    }

    let rsdp_v1 = read_unaligned::<RsdpV1>(rsdp_address)?;
    if rsdp_v1.signature != *b"RSD PTR " {
        return Err("RSDP signature mismatch");
    }
    if !checksum_is_zero(rsdp_address, size_of::<RsdpV1>())? {
        return Err("RSDP v1 checksum failed");
    }

    let root_address = if rsdp_v1.revision >= 2 {
        let rsdp_v2 = read_unaligned::<RsdpV2>(rsdp_address)?;
        let length = usize::try_from(rsdp_v2.length).map_err(|_| "RSDP length overflow")?;
        if length < size_of::<RsdpV2>() || length > 4096 {
            return Err("RSDP v2 length invalid");
        }
        if !checksum_is_zero(rsdp_address, length)? {
            return Err("RSDP extended checksum failed");
        }
        if rsdp_v2.xsdt_address != 0 {
            RootTable::Xsdt(rsdp_v2.xsdt_address)
        } else {
            RootTable::Rsdt(rsdp_v1.rsdt_address as u64)
        }
    } else {
        RootTable::Rsdt(rsdp_v1.rsdt_address as u64)
    };

    find_madt(root_address, identity_map_limit)
}

enum RootTable {
    Rsdt(u64),
    Xsdt(u64),
}

fn find_madt(root: RootTable, identity_map_limit: u64) -> Result<PlatformInfo, &'static str> {
    let (address, entry_size, expected_signature) = match root {
        RootTable::Rsdt(address) => (address, 4usize, *b"RSDT"),
        RootTable::Xsdt(address) => (address, 8usize, *b"XSDT"),
    };

    let header = validate_sdt(address, identity_map_limit)?;
    if header.signature != expected_signature {
        return Err("ACPI root table signature mismatch");
    }

    let table_length = header.length as usize;
    let payload = table_length
        .checked_sub(size_of::<SdtHeader>())
        .ok_or("ACPI root table shorter than header")?;
    if payload % entry_size != 0 {
        return Err("ACPI root table entry area is malformed");
    }

    for index in 0..payload / entry_size {
        let entry_address = address
            .checked_add(size_of::<SdtHeader>() as u64)
            .and_then(|value| value.checked_add((index * entry_size) as u64))
            .ok_or("ACPI root entry address overflow")?;
        let table_address = if entry_size == 8 {
            read_unaligned::<u64>(entry_address)?
        } else {
            read_unaligned::<u32>(entry_address)? as u64
        };

        let table_header = validate_sdt(table_address, identity_map_limit)?;
        if table_header.signature == *b"APIC" {
            return parse_madt(table_address, table_header);
        }
    }

    Err("MADT/APIC table not found")
}

fn parse_madt(address: u64, header: SdtHeader) -> Result<PlatformInfo, &'static str> {
    const MADT_FIXED_SIZE: usize = size_of::<SdtHeader>() + 8;
    if (header.length as usize) < MADT_FIXED_SIZE {
        return Err("MADT is shorter than its fixed header");
    }

    let mut platform = PlatformInfo::empty();
    platform.local_apic_address = read_unaligned::<u32>(address + size_of::<SdtHeader>() as u64)? as u64;

    let end = address
        .checked_add(header.length as u64)
        .ok_or("MADT end overflow")?;
    let mut cursor = address + MADT_FIXED_SIZE as u64;

    while cursor < end {
        if cursor + 2 > end {
            return Err("MADT entry header truncated");
        }
        let entry_type = read_unaligned::<u8>(cursor)?;
        let entry_length = read_unaligned::<u8>(cursor + 1)? as u64;
        if entry_length < 2 || cursor + entry_length > end {
            return Err("MADT entry length invalid");
        }

        match entry_type {
            // Processor Local APIC structure.
            0 if entry_length >= 8 => {
                let apic_id = read_unaligned::<u8>(cursor + 3)? as u32;
                let flags = read_unaligned::<u32>(cursor + 4)?;
                if flags & 0b11 != 0 {
                    platform.push_apic_id(apic_id);
                }
            }
            // Local APIC Address Override.
            5 if entry_length >= 12 => {
                platform.local_apic_address = read_unaligned::<u64>(cursor + 4)?;
            }
            // Processor Local x2APIC structure. K3 can start IDs that fit the
            // xAPIC destination field; larger IDs are discovered but skipped.
            9 if entry_length >= 16 => {
                let x2apic_id = read_unaligned::<u32>(cursor + 4)?;
                let flags = read_unaligned::<u32>(cursor + 8)?;
                if flags & 0b11 != 0 && x2apic_id <= u8::MAX as u32 {
                    platform.push_apic_id(x2apic_id);
                }
            }
            _ => {}
        }

        cursor += entry_length;
    }

    if platform.local_apic_address == 0 || platform.cpu_count == 0 {
        return Err("MADT did not describe a usable local APIC topology");
    }
    Ok(platform)
}

fn validate_sdt(address: u64, identity_map_limit: u64) -> Result<SdtHeader, &'static str> {
    if address == 0 || address >= identity_map_limit {
        return Err("ACPI table lies outside the bootstrap identity map");
    }
    let header = read_unaligned::<SdtHeader>(address)?;
    let length = usize::try_from(header.length).map_err(|_| "ACPI table length overflow")?;
    if length < size_of::<SdtHeader>() || length > MAX_TABLE_SIZE {
        return Err("ACPI table length invalid");
    }
    let table_end = address
        .checked_add(length as u64)
        .ok_or("ACPI table end overflow")?;
    if table_end > identity_map_limit {
        return Err("ACPI table crosses the bootstrap identity-map limit");
    }
    if !checksum_is_zero(address, length)? {
        return Err("ACPI table checksum failed");
    }
    Ok(header)
}

fn checksum_is_zero(address: u64, length: usize) -> Result<bool, &'static str> {
    if address == 0 || length == 0 || length > MAX_TABLE_SIZE {
        return Err("Checksum range invalid");
    }
    let mut sum = 0u8;
    for offset in 0..length {
        // SAFETY: Callers validate the identity-mapped range and ACPI length.
        let byte = unsafe { ptr::read_volatile((address as *const u8).add(offset)) };
        sum = sum.wrapping_add(byte);
    }
    Ok(sum == 0)
}

fn read_unaligned<T: Copy>(address: u64) -> Result<T, &'static str> {
    if address == 0 {
        return Err("Null firmware table address");
    }
    // SAFETY: The caller validates that the containing firmware table is mapped.
    Ok(unsafe { ptr::read_unaligned(address as *const T) })
}


#[derive(Clone, Copy, Debug)]
pub struct AcpiTableRef { pub signature:[u8;4], pub address:u64, pub length:u32, pub revision:u8 }

pub const MAX_ACPI_TABLES:usize=96;
pub struct AcpiCatalog { tables:[AcpiTableRef;MAX_ACPI_TABLES], count:usize, identity_map_limit:u64 }
impl AcpiCatalog {
    pub const fn empty()->Self{Self{tables:[AcpiTableRef{signature:[0;4],address:0,length:0,revision:0};MAX_ACPI_TABLES],count:0,identity_map_limit:0}}
    pub fn build(rsdp_address:u64,identity_map_limit:u64)->Result<Self,&'static str>{
        let rsdp_v1=read_unaligned::<RsdpV1>(rsdp_address)?;
        if rsdp_v1.signature!=*b"RSD PTR "||!checksum_is_zero(rsdp_address,size_of::<RsdpV1>())?{return Err("invalid RSDP")}
        let root=if rsdp_v1.revision>=2{let r=read_unaligned::<RsdpV2>(rsdp_address)?;if r.xsdt_address!=0{RootTable::Xsdt(r.xsdt_address)}else{RootTable::Rsdt(rsdp_v1.rsdt_address as u64)}}else{RootTable::Rsdt(rsdp_v1.rsdt_address as u64)};
        let(address,entry_size,signature)=match root{RootTable::Rsdt(a)=>(a,4usize,*b"RSDT"),RootTable::Xsdt(a)=>(a,8usize,*b"XSDT")};
        let h=validate_sdt(address,identity_map_limit)?;if h.signature!=signature{return Err("ACPI root signature mismatch")}
        let payload=h.length as usize-size_of::<SdtHeader>();if payload%entry_size!=0{return Err("malformed ACPI root")}
        let mut catalog=Self::empty();catalog.identity_map_limit=identity_map_limit;
        for i in 0..payload/entry_size{
            let e=address+size_of::<SdtHeader>() as u64+(i*entry_size) as u64;
            let a=if entry_size==8{read_unaligned::<u64>(e)?}else{read_unaligned::<u32>(e)? as u64};
            let th=validate_sdt(a,identity_map_limit)?;
            if catalog.count>=MAX_ACPI_TABLES{return Err("ACPI catalog full")}
            catalog.tables[catalog.count]=AcpiTableRef{signature:th.signature,address:a,length:th.length,revision:th.revision};catalog.count+=1;
        }
        Ok(catalog)
    }
    pub fn find(&self,signature:[u8;4],instance:usize)->Option<AcpiTableRef>{self.tables[..self.count].iter().filter(|t|t.signature==signature).nth(instance).copied()}
    pub fn iter(&self)->impl Iterator<Item=&AcpiTableRef>{self.tables[..self.count].iter()}
    pub fn identity_map_limit(&self)->u64{self.identity_map_limit}
}

pub fn read_table_u8(table:AcpiTableRef,offset:usize)->Result<u8,&'static str>{if offset>=table.length as usize{return Err("ACPI read outside table")}read_unaligned(table.address+offset as u64)}
pub fn read_table_u16(table:AcpiTableRef,offset:usize)->Result<u16,&'static str>{if offset.checked_add(2).ok_or("ACPI offset overflow")?>table.length as usize{return Err("ACPI read outside table")}read_unaligned(table.address+offset as u64)}
pub fn read_table_u32(table:AcpiTableRef,offset:usize)->Result<u32,&'static str>{if offset.checked_add(4).ok_or("ACPI offset overflow")?>table.length as usize{return Err("ACPI read outside table")}read_unaligned(table.address+offset as u64)}
pub fn read_table_u64(table:AcpiTableRef,offset:usize)->Result<u64,&'static str>{if offset.checked_add(8).ok_or("ACPI offset overflow")?>table.length as usize{return Err("ACPI read outside table")}read_unaligned(table.address+offset as u64)}
