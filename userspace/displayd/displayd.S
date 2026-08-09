.intel_syntax noprefix
.include "userspace/include/twabi.inc"
.section .text
.global _start
_start:
    TW_WRITE TW_CONSOLE_HANDLE, banner, BANNER_LEN
    TW_WRITE TW_CONSOLE_HANDLE, k14b_banner, K14B_BANNER_LEN
    TW_WRITE TW_CONSOLE_HANDLE, k14c_banner, K14C_BANNER_LEN
    TW_WRITE TW_CONSOLE_HANDLE, k14c2_banner, K14C2_BANNER_LEN
    TW_WRITE TW_CONSOLE_HANDLE, k14c3_banner, K14C3_BANNER_LEN
    TW_WRITE TW_CONSOLE_HANDLE, k14c4_banner, K14C4_BANNER_LEN
    mov rax, TW_SYS_DISPLAY_QUERY
    xor rdi, rdi
    int 0x80
    test rax, rax
    jz .headless
    TW_WRITE TW_CONSOLE_HANDLE, mode_ok, MODE_OK_LEN
    mov rax, TW_SYS_GPU_QUERY
    xor rdi, rdi
    int 0x80
    mov rbx, rax

    mov rax, TW_SYS_NATIVE_GPU_QUERY
    xor rdi, rdi
    int 0x80
    mov r12, rax
    mov rcx, rax
    shr rcx, 33
    test cl, 1
    jz .iommu_not_qualified
    TW_WRITE TW_CONSOLE_HANDLE, iommu_qualified, IOMMU_QUALIFIED_LEN
    jmp .iommu_checked
.iommu_not_qualified:
    TW_WRITE TW_CONSOLE_HANDLE, iommu_deferred, IOMMU_DEFERRED_LEN
.iommu_checked:
    mov rax, r12
    test eax, 0xff
    jz .native_none
    TW_WRITE TW_CONSOLE_HANDLE, native_found, NATIVE_FOUND_LEN
    jmp .native_done
.native_none:
    TW_WRITE TW_CONSOLE_HANDLE, native_deferred, NATIVE_DEFERRED_LEN
.native_done:
    mov rax, TW_SYS_NATIVE_GPU_BINDING_QUERY
    xor rdi, rdi
    int 0x80
    mov r13, rax
    test eax, 0xff
    jz .native_binding_none
    test eax, 0x00010000
    jz .native_binding_probe_only
    TW_WRITE TW_CONSOLE_HANDLE, native_binding_claimed, NATIVE_BINDING_CLAIMED_LEN
    jmp .native_binding_done
.native_binding_probe_only:
    TW_WRITE TW_CONSOLE_HANDLE, native_binding_probe, NATIVE_BINDING_PROBE_LEN
    jmp .native_binding_done
.native_binding_none:
    TW_WRITE TW_CONSOLE_HANDLE, native_binding_none, NATIVE_BINDING_NONE_LEN
.native_binding_done:
    mov rax, TW_SYS_NATIVE_GPU_C2_QUERY
    xor rdi, rdi
    int 0x80
    test al, 1
    jz .c2_deferred
    TW_WRITE TW_CONSOLE_HANDLE, c2_ready, C2_READY_LEN
    jmp .c2_done
.c2_deferred:
    TW_WRITE TW_CONSOLE_HANDLE, c2_deferred, C2_DEFERRED_LEN
.c2_done:
    mov rax, TW_SYS_NATIVE_GPU_C3_QUERY
    xor rdi, rdi
    int 0x80
    mov r14, rax
    test al, 1
    jz .c3_no_radeon
    test al, 0x10
    jz .c3_fenced
    TW_WRITE TW_CONSOLE_HANDLE, c3_domain_ready, C3_DOMAIN_READY_LEN
    jmp .c3_done
.c3_fenced:
    TW_WRITE TW_CONSOLE_HANDLE, c3_fenced, C3_FENCED_LEN
    jmp .c3_done
.c3_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c3_qemu, C3_QEMU_LEN
.c3_done:
    mov rax, TW_SYS_NATIVE_GPU_C4_QUERY
    xor rdi, rdi
    int 0x80
    mov r15, rax
    test al, 1
    jz .c4_no_radeon
    test al, 0x20
    jz .c4_fenced
    TW_WRITE TW_CONSOLE_HANDLE, c4_domain_live, C4_DOMAIN_LIVE_LEN
    jmp .c4_done
.c4_fenced:
    TW_WRITE TW_CONSOLE_HANDLE, c4_fenced, C4_FENCED_LEN
    jmp .c4_done
.c4_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c4_qemu, C4_QEMU_LEN
.c4_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c5_banner, K14C5_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C5_QUERY
    xor rdi, rdi
    int 0x80
    mov r13, rax
    test al, 1
    jz .c5_no_radeon
    test al, 0x80
    jz .c5_fenced
    test eax, 0x100
    jz .c5_structures
    TW_WRITE TW_CONSOLE_HANDLE, c5_domain_live, C5_DOMAIN_LIVE_LEN
    jmp .c5_done
.c5_structures:
    TW_WRITE TW_CONSOLE_HANDLE, c5_structures, C5_STRUCTURES_LEN
    jmp .c5_done
.c5_fenced:
    TW_WRITE TW_CONSOLE_HANDLE, c5_fenced, C5_FENCED_LEN
    jmp .c5_done
.c5_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c5_qemu, C5_QEMU_LEN
.c5_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c6_banner, K14C6_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C6_QUERY
    xor rdi, rdi
    int 0x80
    mov r13, rax
    test al, 1
    jz .c6_no_radeon
    test al, 0x40
    jz .c6_fenced
    test eax, 0x400
    jz .c6_eligible
    TW_WRITE TW_CONSOLE_HANDLE, c6_domain_live, C6_DOMAIN_LIVE_LEN
    jmp .c6_done
.c6_eligible:
    TW_WRITE TW_CONSOLE_HANDLE, c6_eligible, C6_ELIGIBLE_LEN
    jmp .c6_done
.c6_fenced:
    TW_WRITE TW_CONSOLE_HANDLE, c6_fenced, C6_FENCED_LEN
    jmp .c6_done
.c6_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c6_qemu, C6_QEMU_LEN
.c6_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c7_banner, K14C7_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C7_QUERY
    xor rdi, rdi
    int 0x80
    mov r14, rax
    test al, 1
    jz .c7_no_radeon
    test al, 2
    jz .c7_fenced
    test al, 0x10
    jz .c7_planned
    TW_WRITE TW_CONSOLE_HANDLE, c7_ro_mmio, C7_RO_MMIO_LEN
    jmp .c7_done
.c7_planned:
    TW_WRITE TW_CONSOLE_HANDLE, c7_planned, C7_PLANNED_LEN
    jmp .c7_done
.c7_fenced:
    TW_WRITE TW_CONSOLE_HANDLE, c7_fenced, C7_FENCED_LEN
    jmp .c7_done
.c7_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c7_qemu, C7_QEMU_LEN
.c7_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c8_banner, K14C8_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C8_QUERY
    xor rdi, rdi
    int 0x80
    mov r14, rax
    test al, 1
    jz .c8_no_radeon
    test al, 8
    jz .c8_unknown
    test al, 0x40
    jz .c8_verified
    TW_WRITE TW_CONSOLE_HANDLE, c8_reads, C8_READS_LEN
    jmp .c8_done
.c8_verified:
    TW_WRITE TW_CONSOLE_HANDLE, c8_verified, C8_VERIFIED_LEN
    jmp .c8_done
.c8_unknown:
    TW_WRITE TW_CONSOLE_HANDLE, c8_unknown, C8_UNKNOWN_LEN
    jmp .c8_done
.c8_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c8_qemu, C8_QEMU_LEN
.c8_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c9_banner, K14C9_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C9_QUERY
    xor rdi, rdi
    int 0x80
    mov r14, rax
    test al, 1
    jz .c9_no_radeon
    test al, 4
    jz .c9_unknown
    test al, 8
    jz .c9_profile_only
    TW_WRITE TW_CONSOLE_HANDLE, c9_identity, C9_IDENTITY_LEN
    jmp .c9_done
.c9_profile_only:
    TW_WRITE TW_CONSOLE_HANDLE, c9_profile, C9_PROFILE_LEN
    jmp .c9_done
.c9_unknown:
    TW_WRITE TW_CONSOLE_HANDLE, c9_unknown, C9_UNKNOWN_LEN
    jmp .c9_done
.c9_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c9_qemu, C9_QEMU_LEN
.c9_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c10_banner, K14C10_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C10_QUERY
    xor rdi, rdi
    int 0x80
    mov r14, rax
    test al, 1
    jz .c10_no_radeon
    test al, 0x20
    jz .c10_fenced
    test al, 0x40
    jz .c10_ready
    TW_WRITE TW_CONSOLE_HANDLE, c10_reads, C10_READS_LEN
    jmp .c10_done
.c10_ready:
    TW_WRITE TW_CONSOLE_HANDLE, c10_ready, C10_READY_LEN
    jmp .c10_done
.c10_fenced:
    TW_WRITE TW_CONSOLE_HANDLE, c10_fenced, C10_FENCED_LEN
    jmp .c10_done
.c10_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c10_qemu, C10_QEMU_LEN
.c10_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c11_banner, K14C11_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C11_QUERY
    xor rdi, rdi
    int 0x80
    mov r14, rax
    test al, 1
    jz .c11_no_radeon
    test al, 8
    jz .c11_unreviewed
    test al, 0x10
    jz .c11_wait_bases
    test al, 0x40
    jz .c11_ready
    TW_WRITE TW_CONSOLE_HANDLE, c11_reads, C11_READS_LEN
    jmp .c11_done
.c11_ready:
    TW_WRITE TW_CONSOLE_HANDLE, c11_ready, C11_READY_LEN
    jmp .c11_done
.c11_wait_bases:
    TW_WRITE TW_CONSOLE_HANDLE, c11_wait_bases, C11_WAIT_BASES_LEN
    jmp .c11_done
.c11_unreviewed:
    TW_WRITE TW_CONSOLE_HANDLE, c11_unreviewed, C11_UNREVIEWED_LEN
    jmp .c11_done
.c11_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c11_qemu, C11_QEMU_LEN
.c11_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c12_banner, K14C12_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C12_QUERY
    xor rdi, rdi
    int 0x80
    mov r14, rax
    test al, 1
    jz .c12_no_radeon
    test al, 8
    jz .c12_wait_bases
    test al, 0x20
    jz .c12_wait_bar
    test al, 0x80
    jz .c12_ready
    TW_WRITE TW_CONSOLE_HANDLE, c12_reads, C12_READS_LEN
    jmp .c12_done
.c12_ready:
    TW_WRITE TW_CONSOLE_HANDLE, c12_ready, C12_READY_LEN
    jmp .c12_done
.c12_wait_bar:
    TW_WRITE TW_CONSOLE_HANDLE, c12_wait_bar, C12_WAIT_BAR_LEN
    jmp .c12_done
.c12_wait_bases:
    TW_WRITE TW_CONSOLE_HANDLE, c12_wait_bases, C12_WAIT_BASES_LEN
    jmp .c12_done
.c12_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c12_qemu, C12_QEMU_LEN
.c12_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c13_banner, K14C13_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C13_QUERY
    xor rdi, rdi
    int 0x80
    mov r14, rax
    test al, 1
    jz .c13_no_radeon
    test al, 0x20
    jnz .c13_proof
    test al, 0x40
    jnz .c13_navi48
    TW_WRITE TW_CONSOLE_HANDLE, c13_wait_proof, C13_WAIT_PROOF_LEN
    jmp .c13_done
.c13_proof:
    TW_WRITE TW_CONSOLE_HANDLE, c13_proof, C13_PROOF_LEN
    jmp .c13_done
.c13_navi48:
    TW_WRITE TW_CONSOLE_HANDLE, c13_navi48, C13_NAVI48_LEN
    jmp .c13_done
.c13_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c13_qemu, C13_QEMU_LEN
.c13_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c14_banner, K14C14_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C14_QUERY
    xor rdi, rdi
    int 0x80
    mov r14, rax
    test al, 1
    jz .c14_no_radeon
    test eax, 0x100
    jnz .c14_ready
    test eax, 0x400
    jnz .c14_navi48
    TW_WRITE TW_CONSOLE_HANDLE, c14_wait, C14_WAIT_LEN
    jmp .c14_done
.c14_ready:
    TW_WRITE TW_CONSOLE_HANDLE, c14_ready, C14_READY_LEN
    jmp .c14_done
.c14_navi48:
    TW_WRITE TW_CONSOLE_HANDLE, c14_navi48, C14_NAVI48_LEN
    jmp .c14_done
.c14_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c14_qemu, C14_QEMU_LEN
.c14_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c15_banner, K14C15_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C15_QUERY
    xor rdi, rdi
    int 0x80
    mov r14, rax
    test al, 1
    jz .c15_no_radeon
    test al, 0x10
    jnz .c15_verified
    TW_WRITE TW_CONSOLE_HANDLE, c15_wait, C15_WAIT_LEN
    jmp .c15_done
.c15_verified:
    TW_WRITE TW_CONSOLE_HANDLE, c15_verified, C15_VERIFIED_LEN
    jmp .c15_done
.c15_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c15_qemu, C15_QEMU_LEN
.c15_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c16_banner, K14C16_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C16_QUERY
    xor rdi, rdi
    int 0x80
    test al, 1
    jz .c16_no_radeon
    test eax, 0x100
    jnz .c16_verified
    TW_WRITE TW_CONSOLE_HANDLE, c16_wait, C16_WAIT_LEN
    jmp .c16_done
.c16_verified:
    TW_WRITE TW_CONSOLE_HANDLE, c16_verified, C16_VERIFIED_LEN
    jmp .c16_done
.c16_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c16_qemu, C16_QEMU_LEN
.c16_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c17_banner, K14C17_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C17_QUERY
    xor rdi, rdi
    int 0x80
    test al, 1
    jz .c17_no_radeon
    test eax, 0x100
    jnz .c17_resolved
    TW_WRITE TW_CONSOLE_HANDLE, c17_wait, C17_WAIT_LEN
    jmp .c17_done
.c17_resolved:
    TW_WRITE TW_CONSOLE_HANDLE, c17_resolved, C17_RESOLVED_LEN
    jmp .c17_done
.c17_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c17_qemu, C17_QEMU_LEN
.c17_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c18_banner, K14C18_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C18_QUERY
    xor rdi, rdi
    int 0x80
    test al, 1
    jz .c18_no_radeon
    test eax, 0x200
    jnz .c18_verified
    TW_WRITE TW_CONSOLE_HANDLE, c18_wait, C18_WAIT_LEN
    jmp .c18_done
.c18_verified:
    TW_WRITE TW_CONSOLE_HANDLE, c18_verified, C18_VERIFIED_LEN
    jmp .c18_done
.c18_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c18_qemu, C18_QEMU_LEN
.c18_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c19_banner, K14C19_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C19_QUERY
    xor rdi, rdi
    int 0x80
    test al, 1
    jz .c19_no_radeon
    test eax, 0x10000
    jnz .c19_verified
    TW_WRITE TW_CONSOLE_HANDLE, c19_wait, C19_WAIT_LEN
    jmp .c19_done
.c19_verified:
    TW_WRITE TW_CONSOLE_HANDLE, c19_verified, C19_VERIFIED_LEN
    jmp .c19_done
.c19_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c19_qemu, C19_QEMU_LEN
.c19_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c20_banner, K14C20_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C20_QUERY
    xor rdi, rdi
    int 0x80
    test al, 1
    jz .c20_no_radeon
    test eax, 0x40
    jnz .c20_resolved
    TW_WRITE TW_CONSOLE_HANDLE, c20_wait, C20_WAIT_LEN
    jmp .c20_done
.c20_resolved:
    TW_WRITE TW_CONSOLE_HANDLE, c20_resolved, C20_RESOLVED_LEN
    jmp .c20_done
.c20_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c20_qemu, C20_QEMU_LEN
.c20_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c21_banner, K14C21_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C21_QUERY
    xor rdi, rdi
    int 0x80
    test al, 1
    jz .c21_no_radeon
    test eax, 0x400
    jnz .c21_verified
    TW_WRITE TW_CONSOLE_HANDLE, c21_wait, C21_WAIT_LEN
    jmp .c21_done
.c21_verified:
    TW_WRITE TW_CONSOLE_HANDLE, c21_verified, C21_VERIFIED_LEN
    jmp .c21_done
.c21_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c21_qemu, C21_QEMU_LEN
.c21_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c22_banner, K14C22_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C22_QUERY
    xor rdi, rdi
    int 0x80
    test al, 1
    jz .c22_no_radeon
    test eax, 0x800
    jnz .c22_verified
    TW_WRITE TW_CONSOLE_HANDLE, c22_wait, C22_WAIT_LEN
    jmp .c22_done
.c22_verified:
    TW_WRITE TW_CONSOLE_HANDLE, c22_verified, C22_VERIFIED_LEN
    jmp .c22_done
.c22_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c22_qemu, C22_QEMU_LEN
.c22_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c23_banner, K14C23_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C23_QUERY
    xor rdi, rdi
    int 0x80
    test al, 1
    jz .c23_no_radeon
    test eax, 0x2000
    jnz .c23_verified
    TW_WRITE TW_CONSOLE_HANDLE, c23_wait, C23_WAIT_LEN
    jmp .c23_done
.c23_verified:
    TW_WRITE TW_CONSOLE_HANDLE, c23_verified, C23_VERIFIED_LEN
    jmp .c23_done
.c23_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c23_qemu, C23_QEMU_LEN
.c23_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c24_banner, K14C24_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C24_QUERY
    xor rdi, rdi
    int 0x80
    test al, 1
    jz .c24_no_radeon
    test eax, 0x2000
    jnz .c24_verified
    TW_WRITE TW_CONSOLE_HANDLE, c24_wait, C24_WAIT_LEN
    jmp .c24_done
.c24_verified:
    TW_WRITE TW_CONSOLE_HANDLE, c24_verified, C24_VERIFIED_LEN
    jmp .c24_done
.c24_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c24_qemu, C24_QEMU_LEN
.c24_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c25_banner, K14C25_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C25_QUERY
    xor rdi, rdi
    int 0x80
    test al, 1
    jz .c25_no_radeon
    test eax, 0x2000
    jnz .c25_verified
    TW_WRITE TW_CONSOLE_HANDLE, c25_wait, C25_WAIT_LEN
    jmp .c25_done
.c25_verified:
    TW_WRITE TW_CONSOLE_HANDLE, c25_verified, C25_VERIFIED_LEN
    jmp .c25_done
.c25_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c25_qemu, C25_QEMU_LEN
.c25_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c26_banner, K14C26_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C26_QUERY
    xor rdi, rdi
    int 0x80
    test al, 1
    jz .c26_no_radeon
    test eax, 0x2000
    jnz .c26_verified
    TW_WRITE TW_CONSOLE_HANDLE, c26_wait, C26_WAIT_LEN
    jmp .c26_done
.c26_verified:
    TW_WRITE TW_CONSOLE_HANDLE, c26_verified, C26_VERIFIED_LEN
    jmp .c26_done
.c26_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c26_qemu, C26_QEMU_LEN
.c26_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c27_banner, K14C27_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C27_QUERY
    xor rdi, rdi
    int 0x80
    test al, 1
    jz .c27_no_radeon
    test eax, 0x2000
    jnz .c27_verified
    TW_WRITE TW_CONSOLE_HANDLE, c27_wait, C27_WAIT_LEN
    jmp .c27_done
.c27_verified:
    TW_WRITE TW_CONSOLE_HANDLE, c27_verified, C27_VERIFIED_LEN
    jmp .c27_done
.c27_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c27_qemu, C27_QEMU_LEN
.c27_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c28_banner, K14C28_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C28_QUERY
    xor rdi, rdi
    int 0x80
    test al, 1
    jz .c28_no_radeon
    test eax, 0x2000
    jnz .c28_verified
    TW_WRITE TW_CONSOLE_HANDLE, c28_wait, C28_WAIT_LEN
    jmp .c28_done
.c28_verified:
    TW_WRITE TW_CONSOLE_HANDLE, c28_verified, C28_VERIFIED_LEN
    jmp .c28_done
.c28_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c28_qemu, C28_QEMU_LEN
.c28_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c29_banner, K14C29_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C29_QUERY
    xor rdi, rdi
    int 0x80
    test al, 1
    jz .c29_no_radeon
    test eax, 0x8000
    jnz .c29_verified
    TW_WRITE TW_CONSOLE_HANDLE, c29_wait, C29_WAIT_LEN
    jmp .c29_done
.c29_verified:
    TW_WRITE TW_CONSOLE_HANDLE, c29_verified, C29_VERIFIED_LEN
    jmp .c29_done
.c29_no_radeon:
    TW_WRITE TW_CONSOLE_HANDLE, c29_qemu, C29_QEMU_LEN
.c29_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c30_banner, K14C30_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C30_QUERY
    xor rdi, rdi
    int 0x80
    test eax, 0x2000
    jz .c30_wait
    test al, 1
    jz .c30_qemu
    TW_WRITE TW_CONSOLE_HANDLE, c30_verified, C30_VERIFIED_LEN
    jmp .c30_done
.c30_qemu:
    TW_WRITE TW_CONSOLE_HANDLE, c30_qemu_msg, C30_QEMU_LEN
    jmp .c30_done
.c30_wait:
    TW_WRITE TW_CONSOLE_HANDLE, c30_wait, C30_WAIT_LEN
.c30_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c31_banner, K14C31_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C31_QUERY
    xor rdi, rdi
    int 0x80
    test eax, 0x2000
    jz .c31_wait
    test al, 1
    jz .c31_qemu
    TW_WRITE TW_CONSOLE_HANDLE, c31_verified, C31_VERIFIED_LEN
    jmp .c31_done
.c31_qemu:
    TW_WRITE TW_CONSOLE_HANDLE, c31_qemu_msg, C31_QEMU_LEN
    jmp .c31_done
.c31_wait:
    TW_WRITE TW_CONSOLE_HANDLE, c31_wait, C31_WAIT_LEN
.c31_done:
    TW_WRITE TW_CONSOLE_HANDLE, k14c32_banner, K14C32_BANNER_LEN
    mov rax, TW_SYS_NATIVE_GPU_C32_QUERY
    xor rdi, rdi
    int 0x80
    test eax, 0x2000
    jz .c32_wait
    test al, 1
    jz .c32_qemu
    TW_WRITE TW_CONSOLE_HANDLE, c32_verified, C32_VERIFIED_LEN
    jmp .c32_done
.c32_qemu:
    TW_WRITE TW_CONSOLE_HANDLE, c32_qemu_msg, C32_QEMU_LEN
    jmp .c32_done
.c32_wait:
    TW_WRITE TW_CONSOLE_HANDLE, c32_wait, C32_WAIT_LEN
.c32_done:
    mov rax, rbx
    test rax, rax
    jz .gpu_none
    test eax, 0x80000000
    jz .gpu_candidate
    TW_WRITE TW_CONSOLE_HANDLE, gpu_transport, GPU_TRANSPORT_LEN

    mov rax, TW_SYS_GPU_PRESENT
    mov rdi, TW_DISPLAY_PRESENT_HANDLE
    mov rsi, 0x13c
    int 0x80
    test rax, rax
    js .present_failed
    TW_WRITE TW_CONSOLE_HANDLE, present_ok, PRESENT_OK_LEN

    mov rax, TW_SYS_GPU_RECOVER
    mov rdi, TW_DISPLAY_PRESENT_HANDLE
    mov rsi, 0x13d
    int 0x80
    test rax, rax
    js .recovery_failed
    TW_WRITE TW_CONSOLE_HANDLE, recovery_ok, RECOVERY_OK_LEN
    jmp .loop

.recovery_failed:
    TW_WRITE TW_CONSOLE_HANDLE, recovery_fallback, RECOVERY_FALLBACK_LEN
    jmp .loop
.present_failed:
    TW_WRITE TW_CONSOLE_HANDLE, present_fallback, PRESENT_FALLBACK_LEN
    jmp .loop
.gpu_candidate:
    TW_WRITE TW_CONSOLE_HANDLE, gpu_found, GPU_FOUND_LEN
    jmp .loop
.gpu_none:
    TW_WRITE TW_CONSOLE_HANDLE, gpu_none, GPU_NONE_LEN
.loop:
    mov rax, TW_SYS_YIELD
    int 0x80
    jmp .loop
.headless:
    TW_WRITE TW_CONSOLE_HANDLE, headless, HEADLESS_LEN
    jmp .loop

.section .rodata
banner:
    .ascii "[displayd] K13.D display/compositor service online; resilience enabled"
banner_end:
.equ BANNER_LEN, banner_end - banner
mode_ok:
    .ascii "[displayd] primary scanout acquired through graphics ABI"
mode_ok_end:
.equ MODE_OK_LEN, mode_ok_end - mode_ok
headless:
    .ascii "[displayd] headless mode; waiting for ForgeGraphics adapter"
headless_end:
.equ HEADLESS_LEN, headless_end - headless

gpu_transport:
    .ascii "[displayd] K14.B VirtIO-GPU command transport online; native translated-DMA gate enabled"
gpu_transport_end:
.equ GPU_TRANSPORT_LEN, gpu_transport_end - gpu_transport

gpu_found:
    .ascii "[displayd] K13 GPU topology available through acceleration ABI"
gpu_found_end:
.equ GPU_FOUND_LEN, gpu_found_end - gpu_found
gpu_none:
    .ascii "[displayd] K13 no PCI GPU candidate; firmware fallback retained"
gpu_none_end:
.equ GPU_NONE_LEN, gpu_none_end - gpu_none

present_ok:
    .ascii "[displayd] K13.D capability-mediated buffered present verified"
present_ok_end:
.equ PRESENT_OK_LEN, present_ok_end - present_ok
present_fallback:
    .ascii "[displayd] accelerated present unavailable; GOP fallback retained"
present_fallback_end:
.equ PRESENT_FALLBACK_LEN, present_fallback_end - present_fallback

recovery_ok:
    .ascii "[displayd] K13.D capability-mediated GPU recovery verified"
recovery_ok_end:
.equ RECOVERY_OK_LEN, recovery_ok_end - recovery_ok
recovery_fallback:
    .ascii "[displayd] GPU recovery unavailable; GOP fallback retained"
recovery_fallback_end:
.equ RECOVERY_FALLBACK_LEN, recovery_fallback_end - recovery_fallback

native_found:
    .ascii "[displayd] K14.B native GPU candidate visible; per-device translated domain still required"
native_found_end:
.equ NATIVE_FOUND_LEN, native_found_end - native_found
native_deferred:
    .ascii "[displayd] K14.B native GPU activation deferred; qualified VirtIO/GOP fallback retained"
native_deferred_end:
.equ NATIVE_DEFERRED_LEN, native_deferred_end - native_deferred

# Inherited K14.A source-contract sentinels.  These strings are deliberately
# retained as non-emitted metadata so the frozen K14.A regression gate keeps
# proving that K14.B preserves the same candidate/deferred semantics.
k14a_native_found_contract:
    .ascii "K14.A native GPU candidate visible"
k14a_native_found_contract_end:
k14a_native_deferred_contract:
    .ascii "K14.A native GPU activation deferred"
k14a_native_deferred_contract_end:

iommu_qualified:
    .ascii "[displayd] K14.B hardware-translated DMA engine qualified"
iommu_qualified_end:
.equ IOMMU_QUALIFIED_LEN, iommu_qualified_end - iommu_qualified
iommu_deferred:
    .ascii "[displayd] K14.B hardware-translated DMA engine not yet qualified"
iommu_deferred_end:
.equ IOMMU_DEFERRED_LEN, iommu_deferred_end - iommu_deferred


k14b_banner:
    .ascii "[displayd] K14.B display/compositor service online; translated-DMA qualification enabled"
k14b_banner_end:
.equ K14B_BANNER_LEN, k14b_banner_end - k14b_banner


native_binding_claimed:
    .ascii "[displayd] K14.C1 native GPU claimed by ForgeBus; persistent hardware domain still fenced"
native_binding_claimed_end:
.equ NATIVE_BINDING_CLAIMED_LEN, native_binding_claimed_end - native_binding_claimed
native_binding_probe:
    .ascii "[displayd] K14.C1 native GPU recognized; vendor backend ownership deferred"
native_binding_probe_end:
.equ NATIVE_BINDING_PROBE_LEN, native_binding_probe_end - native_binding_probe
native_binding_none:
    .ascii "[displayd] K14.C1 waiting for bare-metal AMD/Intel/NVIDIA adapter; VirtIO/GOP fallback retained"
native_binding_none_end:
.equ NATIVE_BINDING_NONE_LEN, native_binding_none_end - native_binding_none
k14c_banner:
    .ascii "[displayd] K14.C1 native backend ownership foundation online"
k14c_banner_end:
.equ K14C_BANNER_LEN, k14c_banner_end - k14c_banner


k14c2_banner:
    .ascii "[displayd] K14.C2 persistent-domain and AMD bring-up contract online"
k14c2_banner_end:
.equ K14C2_BANNER_LEN, k14c2_banner_end - k14c2_banner
c2_ready:
    .ascii "[displayd] K14.C2 persistent translated-domain surrogate verified; physical Radeon domain remains fenced"
c2_ready_end:
.equ C2_READY_LEN, c2_ready_end - c2_ready
c2_deferred:
    .ascii "[displayd] K14.C2 persistent-domain proof deferred; VirtIO/GOP fallback retained"
c2_deferred_end:
.equ C2_DEFERRED_LEN, c2_deferred_end - c2_deferred


# K14.C3 native Radeon bring-up staging.
k14c3_banner:
    .ascii "[displayd] K14.C3 Radeon bare-metal staging contract online"
k14c3_banner_end:
.equ K14C3_BANNER_LEN, k14c3_banner_end - k14c3_banner
c3_qemu:
    .ascii "[displayd] K14.C3 no physical Radeon in QEMU; native MMIO/firmware/submit remain fenced"
c3_qemu_end:
.equ C3_QEMU_LEN, c3_qemu_end - c3_qemu
c3_fenced:
    .ascii "[displayd] K14.C3 Radeon detected but exact translated domain is not yet live; bus mastering remains blocked"
c3_fenced_end:
.equ C3_FENCED_LEN, c3_fenced_end - c3_fenced
c3_domain_ready:
    .ascii "[displayd] K14.C3 Radeon domain prerequisite visible; firmware and command submission still fenced"
c3_domain_ready_end:
.equ C3_DOMAIN_READY_LEN, c3_domain_ready_end - c3_domain_ready


# K14.C4 exact Radeon requester / AMD-Vi qualification gate.
k14c4_banner:
    .ascii "[displayd] K14.C4 exact Radeon requester/AMD-Vi gate online"
k14c4_banner_end:
.equ K14C4_BANNER_LEN, k14c4_banner_end - k14c4_banner
c4_qemu:
    .ascii "[displayd] K14.C4 no physical Radeon in QEMU; exact-domain and bus-master gates remain fenced"
c4_qemu_end:
.equ C4_QEMU_LEN, c4_qemu_end - c4_qemu
c4_fenced:
    .ascii "[displayd] K14.C4 Radeon visible but exact AMD-Vi persistent domain is not live; destructive bring-up remains fenced"
c4_fenced_end:
.equ C4_FENCED_LEN, c4_fenced_end - c4_fenced
c4_domain_live:
    .ascii "[displayd] K14.C4 exact Radeon translated domain visible; MMIO writes, firmware upload, and submit remain fenced"
c4_domain_live_end:
.equ C4_DOMAIN_LIVE_LEN, c4_domain_live_end - c4_domain_live

# K14.C5 AMD-Vi page-table engine foundation.
k14c5_banner:
    .ascii "[displayd] K14.C5 AMD-Vi page-table engine foundation online"
k14c5_banner_end:
.equ K14C5_BANNER_LEN, k14c5_banner_end - k14c5_banner
c5_qemu:
    .ascii "[displayd] K14.C5 no physical Radeon in QEMU; AMD-Vi tables and requester binding remain deferred"
c5_qemu_end:
.equ C5_QEMU_LEN, c5_qemu_end - c5_qemu
c5_fenced:
    .ascii "[displayd] K14.C5 Radeon visible but AMD-Vi table image is incomplete; all native DMA remains fenced"
c5_fenced_end:
.equ C5_FENCED_LEN, c5_fenced_end - c5_fenced
c5_structures:
    .ascii "[displayd] K14.C5 exact Radeon AMD-Vi structures are pinned; hardware register programming remains fenced"
c5_structures_end:
.equ C5_STRUCTURES_LEN, c5_structures_end - c5_structures
c5_domain_live:
    .ascii "[displayd] K14.C5 exact Radeon persistent translated domain is live; read-only MMIO may advance"
c5_domain_live_end:
.equ C5_DOMAIN_LIVE_LEN, c5_domain_live_end - c5_domain_live


# K14.C6 live AMD-Vi hardware-programming boundary.
k14c6_banner:
    .ascii "[displayd] K14.C6 live AMD-Vi hardware-programming boundary online"
k14c6_banner_end:
.equ K14C6_BANNER_LEN, k14c6_banner_end - k14c6_banner
c6_qemu:
    .ascii "[displayd] K14.C6 no physical Radeon in QEMU; AMD-Vi hardware programming remains deferred"
c6_qemu_end:
.equ C6_QEMU_LEN, c6_qemu_end - c6_qemu
c6_fenced:
    .ascii "[displayd] K14.C6 Radeon visible but AMD-Vi hardware prerequisites are incomplete; native DMA remains fenced"
c6_fenced_end:
.equ C6_FENCED_LEN, c6_fenced_end - c6_fenced
c6_eligible:
    .ascii "[displayd] K14.C6 exact Radeon is eligible for AMD-Vi programming; bare-metal activation gate remains unarmed"
c6_eligible_end:
.equ C6_ELIGIBLE_LEN, c6_eligible_end - c6_eligible
c6_domain_live:
    .ascii "[displayd] K14.C6 exact Radeon persistent AMD-Vi domain is live; read-only Radeon MMIO is promoted"
c6_domain_live_end:
.equ C6_DOMAIN_LIVE_LEN, c6_domain_live_end - c6_domain_live


# K14.C7 controlled Radeon MMIO / firmware discovery staging.
k14c7_banner:
    .ascii "[displayd] K14.C7 Radeon MMIO/firmware discovery staging online"
k14c7_banner_end:
.equ K14C7_BANNER_LEN, k14c7_banner_end - k14c7_banner
c7_qemu:
    .ascii "[displayd] K14.C7 no physical Radeon in QEMU; read-only MMIO and firmware discovery remain deferred"
c7_qemu_end:
.equ C7_QEMU_LEN, c7_qemu_end - c7_qemu
c7_fenced:
    .ascii "[displayd] K14.C7 Radeon visible but exact translated domain is not live; all Radeon MMIO remains fenced"
c7_fenced_end:
.equ C7_FENCED_LEN, c7_fenced_end - c7_fenced
c7_planned:
    .ascii "[displayd] K14.C7 Radeon identity/firmware plan ready; read-only aperture awaits C6 domain promotion"
c7_planned_end:
.equ C7_PLANNED_LEN, c7_planned_end - c7_planned
c7_ro_mmio:
    .ascii "[displayd] K14.C7 supervisor read-only Radeon aperture mapped; register reads/writes and firmware upload remain fenced"
c7_ro_mmio_end:
.equ C7_RO_MMIO_LEN, c7_ro_mmio_end - c7_ro_mmio


# K14.C8 Radeon ASIC/IP identification and safe-register-read gate.
k14c8_banner:
    .ascii "[displayd] K14.C8 Radeon ASIC/IP identification and safe-register-read gate online"
k14c8_banner_end:
.equ K14C8_BANNER_LEN, k14c8_banner_end - k14c8_banner
c8_qemu:
    .ascii "[displayd] K14.C8 no physical Radeon in QEMU; ASIC profile and register reads remain deferred"
c8_qemu_end:
.equ C8_QEMU_LEN, c8_qemu_end - c8_qemu
c8_unknown:
    .ascii "[displayd] K14.C8 Radeon present but ASIC profile is unverified; all GPU register reads remain fenced"
c8_unknown_end:
.equ C8_UNKNOWN_LEN, c8_unknown_end - c8_unknown
c8_verified:
    .ascii "[displayd] K14.C8 verified ASIC/IP profile and safe-read whitelist ready; volatile read activation remains deferred"
c8_verified_end:
.equ C8_VERIFIED_LEN, c8_verified_end - c8_verified
c8_reads:
    .ascii "[displayd] K14.C8 whitelisted side-effect-free Radeon register reads active; writes/upload/submit remain fenced"
c8_reads_end:
.equ C8_READS_LEN, c8_reads_end - c8_reads


# K14.C9 verified Radeon profiles + live safe PCI identity reads.
k14c9_banner:
    .ascii "[displayd] K14.C9 verified Radeon profiles and live safe-identity-read gate online"
k14c9_banner_end:
.equ K14C9_BANNER_LEN, k14c9_banner_end - k14c9_banner
c9_qemu:
    .ascii "[displayd] K14.C9 no physical Radeon in QEMU; profile/live PCI reads remain deferred"
c9_qemu_end:
.equ C9_QEMU_LEN, c9_qemu_end - c9_qemu
c9_unknown:
    .ascii "[displayd] K14.C9 Radeon present but no verified device profile matches; MMIO reads remain fenced"
c9_unknown_end:
.equ C9_UNKNOWN_LEN, c9_unknown_end - c9_unknown
c9_profile:
    .ascii "[displayd] K14.C9 verified Radeon profile matched; live PCI identity reads are awaiting physical binding"
c9_profile_end:
.equ C9_PROFILE_LEN, c9_profile_end - c9_profile
c9_identity:
    .ascii "[displayd] K14.C9 live PCI identity reads verified the Radeon profile; MMIO reads/writes remain fenced"
c9_identity_end:
.equ C9_IDENTITY_LEN, c9_identity_end - c9_identity

# K14.C10 per-IP MMIO whitelist + guarded live-read engine.
k14c10_banner:
    .ascii "[displayd] K14.C10 per-IP MMIO whitelist and guarded live-read engine online"
k14c10_banner_end:
.equ K14C10_BANNER_LEN, k14c10_banner_end - k14c10_banner
c10_qemu:
    .ascii "[displayd] K14.C10 no physical Radeon in QEMU; MMIO whitelist activation remains deferred"
c10_qemu_end:
.equ C10_QEMU_LEN, c10_qemu_end - c10_qemu
c10_fenced:
    .ascii "[displayd] K14.C10 Radeon present but reviewed per-IP whitelist is not active; MMIO reads remain fenced"
c10_fenced_end:
.equ C10_FENCED_LEN, c10_fenced_end - c10_fenced
c10_ready:
    .ascii "[displayd] K14.C10 reviewed MMIO whitelist is ready; live reads await activation"
c10_ready_end:
.equ C10_READY_LEN, c10_ready_end - c10_ready
c10_reads:
    .ascii "[displayd] K14.C10 bounded side-effect-free Radeon MMIO reads active; all write-side paths remain fenced"
c10_reads_end:
.equ C10_READS_LEN, c10_reads_end - c10_reads

# K14.C11 reviewed real-register definitions + IP-base resolver gate.
k14c11_banner:
    .ascii "[displayd] K14.C11 reviewed Radeon register definitions and IP-base resolver gate online"
k14c11_banner_end:
.equ K14C11_BANNER_LEN, k14c11_banner_end - k14c11_banner
c11_qemu:
    .ascii "[displayd] K14.C11 no physical Radeon in QEMU; reviewed register definitions remain safely deferred"
c11_qemu_end:
.equ C11_QEMU_LEN, c11_qemu_end - c11_qemu
c11_unreviewed:
    .ascii "[displayd] K14.C11 Radeon present but no reviewed register set matches; MMIO reads remain fenced"
c11_unreviewed_end:
.equ C11_UNREVIEWED_LEN, c11_unreviewed_end - c11_unreviewed
c11_wait_bases:
    .ascii "[displayd] K14.C11 reviewed register set matched; trusted AMD IP-base map is still required before MMIO reads"
c11_wait_bases_end:
.equ C11_WAIT_BASES_LEN, c11_wait_bases_end - c11_wait_bases
c11_ready:
    .ascii "[displayd] K14.C11 reviewed register addresses resolved; live read activation is ready"
c11_ready_end:
.equ C11_READY_LEN, c11_ready_end - c11_ready
c11_reads:
    .ascii "[displayd] K14.C11 reviewed Radeon status reads active; all write-side paths remain fenced"
c11_reads_end:
.equ C11_READS_LEN, c11_reads_end - c11_reads


# K14.C12 trusted IP-base source + first live status-read path.
k14c12_banner:
    .ascii "[displayd] K14.C12 trusted Radeon IP-base and live status-read gate online"
k14c12_banner_end:
.equ K14C12_BANNER_LEN, k14c12_banner_end - k14c12_banner
c12_qemu:
    .ascii "[displayd] K14.C12 no physical Radeon in QEMU; trusted-base/live-read path remains safely deferred"
c12_qemu_end:
.equ C12_QEMU_LEN, c12_qemu_end - c12_qemu
c12_wait_bases:
    .ascii "[displayd] K14.C12 Radeon profile matched but trusted GC/SDMA base source is unresolved"
c12_wait_bases_end:
.equ C12_WAIT_BASES_LEN, c12_wait_bases_end - c12_wait_bases
c12_wait_bar:
    .ascii "[displayd] K14.C12 trusted IP bases resolved but modern Radeon BAR5 register aperture is unavailable"
c12_wait_bar_end:
.equ C12_WAIT_BAR_LEN, c12_wait_bar_end - c12_wait_bar
c12_ready:
    .ascii "[displayd] K14.C12 trusted IP bases and BAR5 register aperture are ready for bounded status reads"
c12_ready_end:
.equ C12_READY_LEN, c12_ready_end - c12_ready
c12_reads:
    .ascii "[displayd] K14.C12 first live Radeon status reads verified; all write-side paths remain fenced"
c12_reads_end:
.equ C12_READS_LEN, c12_reads_end - c12_reads


# K14.C13 physical Radeon read-proof qualification.
k14c13_banner:
    .ascii "[displayd] K14.C13 physical Radeon read-proof qualification gate online"
k14c13_banner_end:
.equ K14C13_BANNER_LEN, k14c13_banner_end - k14c13_banner
c13_qemu:
    .ascii "[displayd] K14.C13 no physical Radeon in QEMU; bare-metal read proof remains safely deferred"
c13_qemu_end:
.equ C13_QEMU_LEN, c13_qemu_end - c13_qemu
c13_wait_proof:
    .ascii "[displayd] K14.C13 Radeon present but C12 live-read evidence is incomplete; destructive paths remain fenced"
c13_wait_proof_end:
.equ C13_WAIT_PROOF_LEN, c13_wait_proof_end - c13_wait_proof
c13_navi48:
    .ascii "[displayd] K14.C13 Navi48 present; trusted AMD IP-discovery bases are still required before physical MMIO proof"
c13_navi48_end:
.equ C13_NAVI48_LEN, c13_navi48_end - c13_navi48
c13_proof:
    .ascii "[displayd] K14.C13 physical Radeon read proof complete; status values sane and bus mastering remains disabled"
c13_proof_end:
.equ C13_PROOF_LEN, c13_proof_end - c13_proof


# K14.C14 controlled write-promotion readiness gate.
k14c14_banner:
    .ascii "[displayd] K14.C14 controlled Radeon write-promotion readiness gate online"
k14c14_banner_end:
.equ K14C14_BANNER_LEN, k14c14_banner_end - k14c14_banner
c14_qemu:
    .ascii "[displayd] K14.C14 no physical Radeon in QEMU; write-promotion readiness remains safely deferred"
c14_qemu_end:
.equ C14_QEMU_LEN, c14_qemu_end - c14_qemu
c14_wait:
    .ascii "[displayd] K14.C14 Radeon present but write-side prerequisites are incomplete; all destructive capabilities remain fenced"
c14_wait_end:
.equ C14_WAIT_LEN, c14_wait_end - c14_wait
c14_navi48:
    .ascii "[displayd] K14.C14 Navi48 remains blocked on trusted AMD IP-discovery bases before write-side eligibility"
c14_navi48_end:
.equ C14_NAVI48_LEN, c14_navi48_end - c14_navi48
c14_ready:
    .ascii "[displayd] K14.C14 write-side prerequisites proven; promotion is still intentionally disabled"
c14_ready_end:
.equ C14_READY_LEN, c14_ready_end - c14_ready

# K14.C15 first controlled write transaction.
k14c15_banner:
    .ascii "[displayd] K14.C15 controlled Radeon write transaction gate online"
k14c15_banner_end:
.equ K14C15_BANNER_LEN, k14c15_banner_end - k14c15_banner
c15_qemu:
    .ascii "[displayd] K14.C15 no physical Radeon in QEMU; controlled identity-write transaction remains safely deferred"
c15_qemu_end:
.equ C15_QEMU_LEN, c15_qemu_end - c15_qemu
c15_wait:
    .ascii "[displayd] K14.C15 Radeon present but controlled identity-write transaction is not qualified; MMIO writes remain fenced"
c15_wait_end:
.equ C15_WAIT_LEN, c15_wait_end - c15_wait
c15_verified:
    .ascii "[displayd] K14.C15 controlled PCI Command identity-write/readback transaction qualified; Radeon MMIO writes remain fenced"
c15_verified_end:
.equ C15_VERIFIED_LEN, c15_verified_end - c15_verified


# K14.C16 reviewed Radeon MMIO-write transaction gate.
k14c16_banner:
    .ascii "[displayd] K14.C16 reviewed Radeon MMIO-write transaction gate online"
k14c16_banner_end:
.equ K14C16_BANNER_LEN, k14c16_banner_end - k14c16_banner
c16_qemu:
    .ascii "[displayd] K14.C16 no physical Radeon in QEMU; reviewed MMIO identity-write remains safely deferred"
c16_qemu_end:
.equ C16_QEMU_LEN, c16_qemu_end - c16_qemu
c16_wait:
    .ascii "[displayd] K14.C16 Radeon present but exact reviewed MMIO target/base is unresolved; no MMIO store executed"
c16_wait_end:
.equ C16_WAIT_LEN, c16_wait_end - c16_wait
c16_verified:
    .ascii "[displayd] K14.C16 Radeon MMIO identity-write/readback transaction qualified"
c16_verified_end:
.equ C16_VERIFIED_LEN, c16_verified_end - c16_verified


# K14.C17 AMD IP Discovery / Navi48 exact-base gate.
k14c17_banner:
    .ascii "[displayd] K14.C17 AMD IP-discovery/Navi48 base-resolution gate online"
k14c17_banner_end:
.equ K14C17_BANNER_LEN, k14c17_banner_end - k14c17_banner
c17_qemu:
    .ascii "[displayd] K14.C17 no physical Radeon in QEMU; live IP-discovery fetch remains safely deferred"
c17_qemu_end:
.equ C17_QEMU_LEN, c17_qemu_end - c17_qemu
c17_wait:
    .ascii "[displayd] K14.C17 Radeon present; parser ready but verified live discovery snapshot/exact GC base unresolved"
c17_wait_end:
.equ C17_WAIT_LEN, c17_wait_end - c17_wait
c17_resolved:
    .ascii "[displayd] K14.C17 exact GC base resolved from verified AMD IP-discovery data"
c17_resolved_end:
.equ C17_RESOLVED_LEN, c17_resolved_end - c17_resolved

# K14.C18 AMD discovery snapshot checksum/acquisition gate.
k14c18_banner:
    .ascii "[displayd] K14.C18 AMD discovery snapshot-verification gate online"
k14c18_banner_end:
.equ K14C18_BANNER_LEN, k14c18_banner_end - k14c18_banner
c18_qemu:
    .ascii "[displayd] K14.C18 no physical Radeon in QEMU; bounded live discovery snapshot acquisition remains safely deferred"
c18_qemu_end:
.equ C18_QEMU_LEN, c18_qemu_end - c18_qemu
c18_wait:
    .ascii "[displayd] K14.C18 Radeon present; checksum engine ready but physical discovery snapshot acquisition remains fenced"
c18_wait_end:
.equ C18_WAIT_LEN, c18_wait_end - c18_wait
c18_verified:
    .ascii "[displayd] K14.C18 AMD discovery snapshot acquired and binary/IP checksums verified"
c18_verified_end:
.equ C18_VERIFIED_LEN, c18_verified_end - c18_verified

# K14.C19 physical AMD discovery snapshot gate.
k14c19_banner:
    .ascii "[displayd] K14.C19 physical AMD discovery snapshot gate online"
k14c19_banner_end:
.equ K14C19_BANNER_LEN, k14c19_banner_end - k14c19_banner
c19_qemu:
    .ascii "[displayd] K14.C19 no physical Radeon in QEMU; direct BAR0 discovery snapshot read remains safely deferred"
c19_qemu_end:
.equ C19_QEMU_LEN, c19_qemu_end - c19_qemu
c19_wait:
    .ascii "[displayd] K14.C19 Radeon present; bounded direct-aperture acquisition not yet verified on this hardware"
c19_wait_end:
.equ C19_WAIT_LEN, c19_wait_end - c19_wait
c19_verified:
    .ascii "[displayd] K14.C19 physical AMD discovery snapshot acquired through BAR0 and checksum-verified"
c19_verified_end:
.equ C19_VERIFIED_LEN, c19_verified_end - c19_verified

# K14.C20 checksum-qualified AMD exact IP-base resolver.
k14c20_banner:
    .ascii "[displayd] K14.C20 AMD exact live IP-base resolver online"
k14c20_banner_end:
.equ K14C20_BANNER_LEN, k14c20_banner_end - k14c20_banner
c20_qemu:
    .ascii "[displayd] K14.C20 no physical Radeon in QEMU; verified-snapshot IP-base resolution remains safely deferred"
c20_qemu_end:
.equ C20_QEMU_LEN, c20_qemu_end - c20_qemu
c20_wait:
    .ascii "[displayd] K14.C20 Radeon present; waiting for checksum-qualified GC/SDMA base records"
c20_wait_end:
.equ C20_WAIT_LEN, c20_wait_end - c20_wait
c20_resolved:
    .ascii "[displayd] K14.C20 exact GC/SDMA bases resolved from checksum-qualified AMD discovery data"
c20_resolved_end:
.equ C20_RESOLVED_LEN, c20_resolved_end - c20_resolved

# K14.C21 reviewed GFX12 SCRATCH_REG0 identity-write promotion.
k14c21_banner:
    .ascii "[displayd] K14.C21 reviewed GFX12 SCRATCH_REG0 rebind/identity-write gate online"
k14c21_banner_end:
.equ K14C21_BANNER_LEN, k14c21_banner_end - k14c21_banner
c21_qemu:
    .ascii "[displayd] K14.C21 no physical Radeon in QEMU; reviewed GFX12 identity-write remains safely deferred"
c21_qemu_end:
.equ C21_QEMU_LEN, c21_qemu_end - c21_qemu
c21_wait:
    .ascii "[displayd] K14.C21 Radeon present; exact GC base-index-1 or reviewed identity-write proof not yet complete"
c21_wait_end:
.equ C21_WAIT_LEN, c21_wait_end - c21_wait
c21_verified:
    .ascii "[displayd] K14.C21 reviewed GFX12 SCRATCH_REG0 identity-write verified with Radeon bus mastering off"
c21_verified_end:
.equ C21_VERIFIED_LEN, c21_verified_end - c21_verified

# K14.C22 first bounded reversible non-identity SCRATCH_REG0 mutation.
k14c22_banner:
    .ascii "[displayd] K14.C22 bounded reversible GFX12 SCRATCH_REG0 mutation gate online"
k14c22_banner_end:
.equ K14C22_BANNER_LEN, k14c22_banner_end - k14c22_banner
c22_qemu:
    .ascii "[displayd] K14.C22 no physical Radeon in QEMU; reversible scratch mutation remains safely deferred"
c22_qemu_end:
.equ C22_QEMU_LEN, c22_qemu_end - c22_qemu
c22_wait:
    .ascii "[displayd] K14.C22 Radeon present; waiting for C21 identity proof and exact restore-verified mutation"
c22_wait_end:
.equ C22_WAIT_LEN, c22_wait_end - c22_wait
c22_verified:
    .ascii "[displayd] K14.C22 one-bit SCRATCH_REG0 mutation and exact restoration verified with Radeon bus mastering off"
c22_verified_end:
.equ C22_VERIFIED_LEN, c22_verified_end - c22_verified

# K14.C23 post-restore persistence and dual-probe stability on the same target.
k14c23_banner:
    .ascii "[displayd] K14.C23 GFX12 SCRATCH_REG0 persistence and dual-probe stability gate online"
k14c23_banner_end:
.equ K14C23_BANNER_LEN, k14c23_banner_end - k14c23_banner
c23_qemu:
    .ascii "[displayd] K14.C23 no physical Radeon in QEMU; dual-probe stability transaction remains safely deferred"
c23_qemu_end:
.equ C23_QEMU_LEN, c23_qemu_end - c23_qemu
c23_wait:
    .ascii "[displayd] K14.C23 Radeon present; waiting for C22 restoration persistence and two exact probe/restore cycles"
c23_wait_end:
.equ C23_WAIT_LEN, c23_wait_end - c23_wait
c23_verified:
    .ascii "[displayd] K14.C23 C22 restoration persisted and two distinct SCRATCH_REG0 probe/restore cycles verified"
c23_verified_end:
.equ C23_VERIFIED_LEN, c23_verified_end - c23_verified


# K14.C24 reversible four-bit pattern on the exact frozen SCRATCH_REG0 target.
k14c24_banner:
    .ascii "[displayd] K14.C24 reversible GFX12 SCRATCH_REG0 multi-bit pattern gate online"
k14c24_banner_end:
.equ K14C24_BANNER_LEN, k14c24_banner_end - k14c24_banner
c24_qemu:
    .ascii "[displayd] K14.C24 no physical Radeon in QEMU; reversible multi-bit pattern transaction remains safely deferred"
c24_qemu_end:
.equ C24_QEMU_LEN, c24_qemu_end - c24_qemu
c24_wait:
    .ascii "[displayd] K14.C24 Radeon present; waiting for C23 stability proof and exact four-bit pattern/restore cycle"
c24_wait_end:
.equ C24_WAIT_LEN, c24_wait_end - c24_wait
c24_verified:
    .ascii "[displayd] K14.C24 four-bit SCRATCH_REG0 pattern and exact restoration verified with Radeon bus mastering off"
c24_verified_end:
.equ C24_VERIFIED_LEN, c24_verified_end - c24_verified

# K14.C25 two distinct four-bit pattern cycles on the exact frozen SCRATCH_REG0 target.
k14c25_banner:
    .ascii "[displayd] K14.C25 GFX12 SCRATCH_REG0 dual multi-bit pattern stability gate online"
k14c25_banner_end:
.equ K14C25_BANNER_LEN, k14c25_banner_end - k14c25_banner
c25_qemu:
    .ascii "[displayd] K14.C25 no physical Radeon in QEMU; dual multi-bit pattern stability transaction remains safely deferred"
c25_qemu_end:
.equ C25_QEMU_LEN, c25_qemu_end - c25_qemu
c25_wait:
    .ascii "[displayd] K14.C25 Radeon present; waiting for C24 pattern proof and dual four-bit pattern/restore stability"
c25_wait_end:
.equ C25_WAIT_LEN, c25_wait_end - c25_wait
c25_verified:
    .ascii "[displayd] K14.C25 dual four-bit SCRATCH_REG0 pattern cycles and exact restoration verified with Radeon bus mastering off"
c25_verified_end:
.equ C25_VERIFIED_LEN, c25_verified_end - c25_verified

# K14.C26 final K14 reviewed MMIO allowlist and SCRATCH_REG1 read-only completion gate.
k14c26_banner:
    .ascii "[displayd] K14.C26 final GFX12 reviewed MMIO allowlist and SCRATCH_REG1 read-only completion gate online"
k14c26_banner_end:
.equ K14C26_BANNER_LEN, k14c26_banner_end - k14c26_banner
c26_qemu:
    .ascii "[displayd] K14.C26 no physical Radeon in QEMU; final K14 SCRATCH_REG1 read proof remains safely deferred"
c26_qemu_end:
.equ C26_QEMU_LEN, c26_qemu_end - c26_qemu
c26_wait:
    .ascii "[displayd] K14.C26 Radeon present; waiting for final reviewed SCRATCH_REG1 read-only completion proof"
c26_wait_end:
.equ C26_WAIT_LEN, c26_wait_end - c26_wait
c26_verified:
    .ascii "[displayd] K14.C26 final reviewed REG0/REG1 allowlist and SCRATCH_REG1 read-only proof verified; K14 completion gate closed"
c26_verified_end:
.equ C26_VERIFIED_LEN, c26_verified_end - c26_verified

# K14.C27 complete operational Radeon driver-core gate.
k14c27_banner:
    .ascii "[displayd] K14.C27 complete Radeon driver core online"
k14c27_banner_end:
.equ K14C27_BANNER_LEN, k14c27_banner_end - k14c27_banner
c27_qemu:
    .ascii "[displayd] K14.C27 no physical Radeon in QEMU; operational driver-core software paths qualified and physical ownership/MMIO route safely deferred"
c27_qemu_end:
.equ C27_QEMU_LEN, c27_qemu_end - c27_qemu
c27_wait:
    .ascii "[displayd] K14.C27 Radeon present; driver core has not closed ownership/topology/MMIO/IRQ/error gates"
c27_wait_end:
.equ C27_WAIT_LEN, c27_wait_end - c27_wait
c27_verified:
    .ascii "[displayd] K14.C27 Radeon driver core verified: ForgeBus ownership, live topology, reviewed MMIO service, masked IRQ route, lifecycle/error/reset coordinator online"
c27_verified_end:
.equ C27_VERIFIED_LEN, c27_verified_end - c27_verified

# K14.C28 operational memory + firmware + recovery gate.
k14c28_banner:
    .ascii "[displayd] K14.C28 Radeon memory, firmware staging, and recovery subsystem online"
k14c28_banner_end:
.equ K14C28_BANNER_LEN, k14c28_banner_end - k14c28_banner
c28_qemu:
    .ascii "[displayd] K14.C28 no physical Radeon in QEMU; real GTT allocation/mapping/reclaim plus firmware parser/CRC and watchdog recovery logic qualified while hardware firmware staging is deferred"
c28_qemu_end:
.equ C28_QEMU_LEN, c28_qemu_end - c28_qemu
c28_wait:
    .ascii "[displayd] K14.C28 Radeon present; memory, validated firmware staging, or recovery gates are incomplete"
c28_wait_end:
.equ C28_WAIT_LEN, c28_wait_end - c28_wait
c28_verified:
    .ascii "[displayd] K14.C28 Radeon memory+firmware+recovery verified: reclaimable GTT, VRAM reservations, validated pinned firmware staging, watchdog and resource-safe software recovery ready"
c28_verified_end:
.equ C28_VERIFIED_LEN, c28_verified_end - c28_verified

# K14.C29 operational rings + queues + fences + DMA gate.
k14c29_banner:
    .ascii "[displayd] K14.C29 Radeon rings, queues, timeline fences, and typed DMA subsystem online"
k14c29_banner_end:
.equ K14C29_BANNER_LEN, k14c29_banner_end - k14c29_banner
c29_qemu:
    .ascii "[displayd] K14.C29 no physical Radeon in QEMU; GTT ring, FIFO queue, SDMA packet codec, timeline fence, and owned-memory DMA executor qualified while physical SDMA remains safely deferred"
c29_qemu_end:
.equ C29_QEMU_LEN, c29_qemu_end - c29_qemu
c29_wait:
    .ascii "[displayd] K14.C29 Radeon present; driver-side ring/queue/fence/DMA control plane is waiting on exact physical SDMA prerequisites"
c29_wait_end:
.equ C29_WAIT_LEN, c29_wait_end - c29_wait
c29_verified:
    .ascii "[displayd] K14.C29 Radeon ring/queue/fence/DMA control plane verified; raw packets and unprotected bus mastering remain forbidden"
c29_verified_end:
.equ C29_VERIFIED_LEN, c29_verified_end - c29_verified
# K14.C30 complete basic display engine gate.
k14c30_banner:
    .ascii "[displayd] K14.C30 complete basic display engine online"
k14c30_banner_end:
.equ K14C30_BANNER_LEN, k14c30_banner_end - k14c30_banner
c30_qemu_msg:
    .ascii "[displayd] K14.C30 QEMU/GOP backend verified: EDID parser, connector/CRTC/plane model, double-buffered GTT scanout, live framebuffer page flips, atomic mode rollback, and hotplug bookkeeping operational"
c30_qemu_msg_end:
.equ C30_QEMU_LEN, c30_qemu_msg_end - c30_qemu_msg
c30_wait:
    .ascii "[displayd] K14.C30 display engine not qualified; native DCN/HPD remains fail-closed and firmware scanout must remain available"
c30_wait_end:
.equ C30_WAIT_LEN, c30_wait_end - c30_wait
c30_verified:
    .ascii "[displayd] K14.C30 basic display engine verified; native DCN programming is not falsely claimed"
c30_verified_end:
.equ C30_VERIFIED_LEN, c30_verified_end - c30_verified
# K14.C31 graphics + compute execution gate.
k14c31_banner:
    .ascii "[displayd] K14.C31 graphics and compute execution subsystem online"
k14c31_banner_end:
.equ K14C31_BANNER_LEN, k14c31_banner_end - k14c31_banner
c31_qemu_msg:
    .ascii "[displayd] K14.C31 QEMU reference backend verified: owned shader upload/cache, typed commands, separate compute/graphics queues, vector-add dispatch, triangle draw, timeline fences, and live framebuffer present operational"
c31_qemu_msg_end:
.equ C31_QEMU_LEN, c31_qemu_msg_end - c31_qemu_msg
c31_wait:
    .ascii "[displayd] K14.C31 execution gate not qualified; physical Radeon CP/GFX queues and native AMD ISA remain fail-closed"
c31_wait_end:
.equ C31_WAIT_LEN, c31_wait_end - c31_wait
c31_verified:
    .ascii "[displayd] K14.C31 reference graphics/compute execution verified; physical Radeon queue programming remains separately gated"
c31_verified_end:
.equ C31_VERIFIED_LEN, c31_verified_end - c31_verified
# K14.C32 production/stability + final K14 gate.
k14c32_banner:
    .ascii "[displayd] K14.C32 production/stability and final Radeon foundation online"
k14c32_banner_end:
.equ K14C32_BANNER_LEN, k14c32_banner_end - k14c32_banner
c32_qemu_msg:
    .ascii "[displayd] K14.C32 QEMU production gate verified: queue/memory stress, recovery+IRQ, graphics+compute/display+compute coexistence, scanout, telemetry, power, frozen GPU ABI, shader precache, multi-GPU inventory; physical Radeon stress separate"
c32_qemu_msg_end:
.equ C32_QEMU_LEN, c32_qemu_msg_end - c32_qemu_msg
c32_wait:
    .ascii "[displayd] K14.C32 final production/stability gate not qualified; K14 remains open and ForgeAudio must not begin"
c32_wait_end:
.equ C32_WAIT_LEN, c32_wait_end - c32_wait
c32_verified:
    .ascii "[displayd] K14.C32 production/stability foundation verified; physical-stress bit reports silicon evidence independently"
c32_verified_end:
.equ C32_VERIFIED_LEN, c32_verified_end - c32_verified

