use crate::acpi::MAX_CPUS;
use core::arch::asm;
use core::mem::size_of;
use core::ptr;

pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;
pub const USER_DATA_SELECTOR: u16 = 0x1b;
pub const USER_CODE_SELECTOR: u16 = 0x23;
const TSS_SELECTOR: u16 = 0x28;

const IST_STACK_SIZE: usize = 8 * 1024;
const DOUBLE_FAULT_IST: usize = 0;
const NMI_IST: usize = 1;
const MACHINE_CHECK_IST: usize = 2;
const FATAL_FAULT_IST: usize = 3;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct TaskStateSegment {
    reserved_0: u32,
    privilege_stacks: [u64; 3],
    reserved_1: u64,
    interrupt_stacks: [u64; 7],
    reserved_2: u64,
    reserved_3: u16,
    io_map_base: u16,
}

const _: [(); 104] = [(); size_of::<TaskStateSegment>()];

impl TaskStateSegment {
    const EMPTY: Self = Self {
        reserved_0: 0,
        privilege_stacks: [0; 3],
        reserved_1: 0,
        interrupt_stacks: [0; 7],
        reserved_2: 0,
        reserved_3: 0,
        io_map_base: 0,
    };
}

#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Gdt {
    entries: [u64; 7],
}

impl Gdt {
    const EMPTY: Self = Self { entries: [0; 7] };
}

#[repr(C, align(16))]
struct IstBank([[u8; IST_STACK_SIZE]; MAX_CPUS]);

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: u64,
}

static mut CPU_TSS: [TaskStateSegment; MAX_CPUS] = [TaskStateSegment::EMPTY; MAX_CPUS];
static mut CPU_GDTS: [Gdt; MAX_CPUS] = [Gdt::EMPTY; MAX_CPUS];
static mut DOUBLE_FAULT_STACKS: IstBank = IstBank([[0; IST_STACK_SIZE]; MAX_CPUS]);
static mut NMI_STACKS: IstBank = IstBank([[0; IST_STACK_SIZE]; MAX_CPUS]);
static mut MACHINE_CHECK_STACKS: IstBank = IstBank([[0; IST_STACK_SIZE]; MAX_CPUS]);
static mut FATAL_FAULT_STACKS: IstBank = IstBank([[0; IST_STACK_SIZE]; MAX_CPUS]);

/// Install the per-CPU GDT and TSS. K6 retains ring-3 code/data descriptors and
/// retains the K3 emergency IST stacks.
pub fn load_for_cpu(cpu_index: usize) {
    assert!(cpu_index < MAX_CPUS, "GDT CPU index outside static table");

    unsafe {
        let tss = ptr::addr_of_mut!(CPU_TSS).cast::<TaskStateSegment>().add(cpu_index);
        ptr::write(tss, TaskStateSegment::EMPTY);

        let double_fault_top = stack_top(ptr::addr_of_mut!(DOUBLE_FAULT_STACKS), cpu_index);
        let nmi_top = stack_top(ptr::addr_of_mut!(NMI_STACKS), cpu_index);
        let machine_check_top = stack_top(ptr::addr_of_mut!(MACHINE_CHECK_STACKS), cpu_index);
        let fatal_fault_top = stack_top(ptr::addr_of_mut!(FATAL_FAULT_STACKS), cpu_index);

        let tss_bytes = tss.cast::<u8>();
        ptr::write_unaligned(
            tss_bytes.add(36 + DOUBLE_FAULT_IST * 8).cast::<u64>(),
            double_fault_top,
        );
        ptr::write_unaligned(
            tss_bytes.add(36 + NMI_IST * 8).cast::<u64>(),
            nmi_top,
        );
        ptr::write_unaligned(
            tss_bytes.add(36 + MACHINE_CHECK_IST * 8).cast::<u64>(),
            machine_check_top,
        );
        ptr::write_unaligned(
            tss_bytes.add(36 + FATAL_FAULT_IST * 8).cast::<u64>(),
            fatal_fault_top,
        );
        ptr::write_unaligned(
            tss_bytes.add(102).cast::<u16>(),
            size_of::<TaskStateSegment>() as u16,
        );

        let (tss_low, tss_high) = tss_descriptor(tss as u64);
        let gdt = ptr::addr_of_mut!(CPU_GDTS).cast::<Gdt>().add(cpu_index);
        ptr::write(
            gdt,
            Gdt {
                entries: [
                    0,
                    0x00af_9a00_0000_ffff,
                    0x00cf_9200_0000_ffff,
                    0x00cf_f200_0000_ffff,
                    0x00af_fa00_0000_ffff,
                    tss_low,
                    tss_high,
                ],
            },
        );

        let pointer = DescriptorTablePointer {
            limit: (size_of::<Gdt>() - 1) as u16,
            base: ptr::addr_of!((*gdt).entries) as u64,
        };

        asm!(
            "lgdt [{gdt}]",
            "push {code_selector}",
            "lea rax, [rip + 2f]",
            "push rax",
            "retfq",
            "2:",
            "mov ax, {data_selector}",
            "mov ds, ax",
            "mov es, ax",
            "mov ss, ax",
            "xor eax, eax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ax, {tss_selector}",
            "ltr ax",
            gdt = in(reg) &pointer,
            code_selector = const KERNEL_CODE_SELECTOR as u64,
            data_selector = const KERNEL_DATA_SELECTOR,
            tss_selector = const TSS_SELECTOR,
            out("rax") _,
        );
    }
}

/// Set RSP0 for a CPU. Hardware uses this stack when an interrupt or syscall
/// crosses from ring 3 into WeaveCore.
pub fn set_kernel_stack(cpu_index: usize, stack_top: u64) {
    assert!(cpu_index < MAX_CPUS, "TSS CPU index outside static table");
    assert!(stack_top != 0 && stack_top & 0xf == 0, "RSP0 must be 16-byte aligned");
    unsafe {
        let tss = ptr::addr_of_mut!(CPU_TSS).cast::<TaskStateSegment>().add(cpu_index);
        ptr::write_unaligned(tss.cast::<u8>().add(4).cast::<u64>(), stack_top);
    }
}

unsafe fn stack_top(bank: *mut IstBank, cpu_index: usize) -> u64 {
    let first_byte = bank.cast::<u8>();
    unsafe { first_byte.add((cpu_index + 1) * IST_STACK_SIZE) as u64 }
}

fn tss_descriptor(base: u64) -> (u64, u64) {
    let limit = (size_of::<TaskStateSegment>() - 1) as u64;
    let low = (limit & 0xffff)
        | ((base & 0x00ff_ffff) << 16)
        | (0x89u64 << 40)
        | (((limit >> 16) & 0x0f) << 48)
        | (((base >> 24) & 0xff) << 56);
    let high = base >> 32;
    (low, high)
}
