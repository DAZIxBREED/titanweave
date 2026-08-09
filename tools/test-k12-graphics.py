#!/usr/bin/env python3
"""K12 graphics regression gate retained by later milestones."""
from pathlib import Path
import re
import tomllib

root = Path(__file__).resolve().parents[1]
def text(path: str) -> str:
    return (root / path).read_text()

with (root / 'Cargo.toml').open('rb') as f:
    cargo = tomllib.load(f)
major, minor, patch = (int(part) for part in cargo['workspace']['package']['version'].split('.'))
assert major == 0 and minor >= 12

boot = text('libraries/boot-protocol/src/lib.rs')
match = re.search(r'BOOT_PROTOCOL_VERSION: u32 = (\d+)', boot)
assert match and int(match.group(1)) >= 12
for token in ['FramebufferInfo', 'framebuffer_pixel_format', 'is_linear_32bpp']:
    assert token in boot, token

loader = text('boot/uefi-loader/src/main.rs')
for token in ['GraphicsOutput', 'capture_framebuffer', 'FALLBACK_MAX_WIDTH', 'FALLBACK_MAX_HEIGHT']:
    assert token in loader, token
assert 'GOP framebuffer captured for' in loader

checks = {
    'kernel/weavecore/src/framebuffer.rs': [
        'from_boot_info', 'write_volatile', 'draw_boot_card', 'Rgbx8888', 'Bgrx8888',
    ],
    'kernel/weavecore/src/graphics_abi.rs': [
        'GRAPHICS_ABI_VERSION', 'DisplayInfo', 'SurfaceCreate', 'PresentRequest',
        'InputEvent', 'PRESENT_FLAG_VSYNC',
    ],
    'kernel/weavecore/src/forgegraphics.rs': [
        'FORGEGRAPHICS_ABI_VERSION', 'CAP_SCANOUT', 'CAP_COMPUTE',
        'CAP_MULTI_GPU_COPY', 'BackendKind', 'AdapterRegistry',
    ],
    'kernel/weavecore/src/compositor.rs': [
        'SurfaceRegistry', 'DamageTracker', 'hit_test', 'present', 'destroy',
    ],
    'kernel/weavecore/src/input_router.rs': [
        'InputRouter', 'capture_pointer', 'route_pointer_move', 'route_key',
    ],
    'kernel/weavecore/src/display.rs': [
        'GOP scanout online', 'packed_primary_mode', 'Workplace Shell reference preview rendered',
    ],
    'kernel/weavecore/src/workplace_shell.rs': [
        'WorkplacePreviewReport', 'render_preview', 'Workplace Navigator',
        'Bottom OS/2-inspired taskbar',
    ],
    'kernel/weavecore/src/syscalls.rs': ['SYS_DISPLAY_QUERY', 'display::packed_primary_mode'],
    'kernel/weavecore/src/service.rs': ['DISPLAYD.ELF', 'ServiceRole::Display'],
}
for path, tokens in checks.items():
    source = text(path)
    for token in tokens:
        assert token in source, (path, token)

assert (root / 'userspace/displayd/displayd.S').is_file()
assert (root / 'docs/design/WORKPLACE_SHELL_REFERENCE.png').stat().st_size > 100_000
assert 'TW_SYS_DISPLAY_QUERY' in text('userspace/include/twabi.inc')
assert 'displayd' in text('tools/build-userspace.sh')
assert 'DISPLAYD.ELF' in text('tools/make-fat32.py')

# K11 syscall vector protection remains mandatory.
interrupts = text('kernel/weavecore/src/arch/x86_64/idt.rs')
assert 'SYSCALL_VECTOR' in interrupts
router = text('kernel/weavecore/src/interrupt_router.rs')
assert 'SYSCALL_VECTOR' in router or '0x80' in router

print('Titanweave K12 graphics/display regression checks passed.')
