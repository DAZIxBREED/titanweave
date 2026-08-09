use crate::acpi::{PlatformInfo, MAX_CPUS};
use crate::arch::x86_64::{apic, gdt, halt_forever, idt, pause};
use crate::memory::FrameAllocator;
use crate::percpu;
use crate::serial;
use core::arch::global_asm;
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};
use titanweave_boot_protocol::{BootInfo, BOOT_INFO_MAGIC, BOOT_PROTOCOL_VERSION};

const AP_STACK_PAGES: u64 = 16;
const AP_STACK_SIZE: u64 = AP_STACK_PAGES * 4096;
const PARAM_OFFSET: u64 = 0x800;
const PARAM_CR3: u64 = 0x00;
const PARAM_STACK_TOP: u64 = 0x08;
const PARAM_BOOT_INFO: u64 = 0x10;
const PARAM_ENTRY: u64 = 0x18;
const PARAM_CPU_INDEX: u64 = 0x20;

static CPU_ONLINE: [AtomicBool; MAX_CPUS] =
    [const { AtomicBool::new(false) }; MAX_CPUS];

pub struct SmpReport {
    pub discovered: usize,
    pub online: usize,
    pub failed: usize,
}

pub fn start_application_processors(
    boot_info: &BootInfo,
    platform: &PlatformInfo,
    bsp_apic_id: u32,
    allocator: &mut FrameAllocator<'_>,
) -> SmpReport {
    copy_trampoline(boot_info);

    let maximum = core::cmp::min(
        platform.cpu_count,
        boot_info.smp.maximum_logical_cpus as usize,
    );
    let bsp_index = platform.apic_ids[..maximum]
        .iter()
        .position(|id| *id == bsp_apic_id)
        .unwrap_or(0);
    CPU_ONLINE[bsp_index].store(true, Ordering::Release);

    let vector = u8::try_from(boot_info.smp.trampoline_physical_base >> 12)
        .expect("AP trampoline must be below 1 MiB");
    let mut online = 1usize;
    let mut failed = 0usize;

    for cpu_index in 0..maximum {
        let apic_id = platform.apic_ids[cpu_index];
        if apic_id == bsp_apic_id {
            continue;
        }
        if apic_id > u8::MAX as u32 {
            serial::println(format_args!(
                "[SMP ] APIC ID {} requires x2APIC startup; skipped in K3",
                apic_id
            ));
            failed += 1;
            continue;
        }

        let Some(stack_base) = allocator.allocate_contiguous(AP_STACK_PAGES) else {
            serial::println(format_args!(
                "[SMP ] Could not allocate stack for APIC {}",
                apic_id
            ));
            failed += 1;
            continue;
        };
        unsafe { ptr::write_bytes(stack_base as *mut u8, 0, AP_STACK_SIZE as usize) };
        let stack_top = (stack_base + AP_STACK_SIZE) & !0x0f;
        CPU_ONLINE[cpu_index].store(false, Ordering::Release);
        write_parameters(
            boot_info,
            stack_top,
            boot_info as *const BootInfo as u64,
            weavecore_ap_entry as usize as u64,
            cpu_index as u64,
        );

        serial::println(format_args!(
            "[SMP ] Starting logical CPU {} (APIC {}) stack={:#x}",
            cpu_index, apic_id, stack_base
        ));
        apic::send_init_sipi(apic_id, vector);

        let mut reached_entry = false;
        for _ in 0..5_000_000 {
            if CPU_ONLINE[cpu_index].load(Ordering::Acquire) {
                reached_entry = true;
                break;
            }
            pause();
        }

        if reached_entry {
            online += 1;
        } else {
            failed += 1;
            serial::println(format_args!(
                "[SMP ] APIC {} did not report online",
                apic_id
            ));
        }
    }

    SmpReport {
        discovered: maximum,
        online,
        failed,
    }
}

fn copy_trampoline(boot_info: &BootInfo) {
    unsafe extern "C" {
        static weave_ap_trampoline_start: u8;
        static weave_ap_trampoline_end: u8;
    }

    let source = unsafe { core::ptr::addr_of!(weave_ap_trampoline_start) as u64 };
    let end = unsafe { core::ptr::addr_of!(weave_ap_trampoline_end) as u64 };
    let size = end.saturating_sub(source);
    assert!(size != 0 && size <= boot_info.smp.trampoline_size);

    unsafe {
        ptr::copy_nonoverlapping(
            source as *const u8,
            boot_info.smp.trampoline_physical_base as *mut u8,
            size as usize,
        );
    }
}

fn write_parameters(
    boot_info: &BootInfo,
    stack_top: u64,
    boot_info_address: u64,
    entry: u64,
    cpu_index: u64,
) {
    let base = boot_info.smp.trampoline_physical_base + PARAM_OFFSET;
    unsafe {
        ptr::write_volatile((base + PARAM_CR3) as *mut u64, boot_info.bootstrap.page_table_root);
        ptr::write_volatile((base + PARAM_STACK_TOP) as *mut u64, stack_top);
        ptr::write_volatile((base + PARAM_BOOT_INFO) as *mut u64, boot_info_address);
        ptr::write_volatile((base + PARAM_ENTRY) as *mut u64, entry);
        ptr::write_volatile((base + PARAM_CPU_INDEX) as *mut u64, cpu_index);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn weavecore_ap_entry(boot_info_address: u64, cpu_index: u64) -> ! {
    let boot_info = unsafe { &*(boot_info_address as *const BootInfo) };
    if boot_info.magic != BOOT_INFO_MAGIC
        || boot_info.protocol_version != BOOT_PROTOCOL_VERSION
        || cpu_index as usize >= MAX_CPUS
    {
        halt_forever();
    }

    let _ = crate::arch::x86_64::initialize_execution_state();
    gdt::load_for_cpu(cpu_index as usize);
    idt::load();
    let apic_id = apic::initialize_application_processor();
    percpu::initialize(cpu_index as usize, apic_id, false);
    CPU_ONLINE[cpu_index as usize].store(true, Ordering::Release);
    serial::println(format_args!(
        "[SMP ] Logical CPU {} online (APIC {})",
        cpu_index, apic_id
    ));
    halt_forever();
}

global_asm!(include_str!("ap_trampoline.S"));
