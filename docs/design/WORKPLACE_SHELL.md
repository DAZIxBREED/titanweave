# Titanweave Workplace Shell — Canonical Interface Direction

`WORKPLACE_SHELL_REFERENCE.png` is the user-approved visual target for the
Titanweave desktop beginning with K12.

## Structure

- **Top bar:** dark navy, thin, global OS identity/status area. Titanweave mark
  and Workplace Shell identity live on the left; time and machine status live
  on the right.
- **Workplace Navigator:** persistent left rail inspired by OS/2's object/workplace
  model. It exposes Dashboard, Terminal, Files & Drives, Network, Processes,
  Archives, VR / Media, Security, Drivers, Settings, quick launch, and concise
  system-health status.
- **Desktop:** blue-steel technical wallpaper with a centered Titanweave mark.
  It must remain useful behind windows rather than behaving like a full-screen
  mobile launcher.
- **Windows:** compact classic frames, blue title bars, visible borders,
  predictable minimize/maximize/close controls, menu bars when useful, and high
  information density.
- **Taskbar:** bottom-aligned Start area, running-window buttons, then a status
  tray. It should feel like a modernized OS/2/desktop workstation rather than a
  tablet dock.

## Visual language

The baseline palette is blue-steel / navy / cool gray with restrained cyan,
purple, and green status accents. The approved Titanweave logo remains the
canonical brand mark. Neon/cyberpunk accents may appear in creator/gaming modes,
but the default shell must remain legible, professional, and information dense.

## Interaction principles

1. Mouse/keyboard first, touch-capable where practical.
2. No hidden mandatory gestures.
3. Window management must remain deterministic and scriptable.
4. System status is visible without opening a settings application.
5. Power-user functionality is not buried to make the UI artificially minimal.
6. VR and low-latency audio status are first-class shell concepts.
7. Multi-GPU status is first-class and must later expose which adapter owns
   scanout, compute, encode, and application workloads.

## K12 implementation note

K12 renders only a deterministic geometric preview of this layout through the
firmware framebuffer. The real shell belongs in userspace (`DISPLAYD` plus the
future Workplace Shell process) and will use ForgeGraphics surfaces. This keeps
branding and UX iteration outside the kernel while preserving a visible
fallback path.

## Canonical Titanweave branding asset

The approved Titanweave OS logo is stored as `TITANWEAVE_OS_LOGO.png` beside
this document. The logo is a resource-layer asset for TitanBoot/DISPLAYD/the
Workplace Shell; kernel graphics contracts must remain independent of the
specific bitmap artwork.
