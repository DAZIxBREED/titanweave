//! K12 Workplace Shell visual-reference renderer.
//!
//! This is deliberately a *qualification/reference* renderer, not Titanweave's
//! permanent desktop implementation.  It proves that the K12 scanout and
//! compositor foundation can reproduce the geometry of the canonical
//! Titanweave Workplace Shell: top status bar, left navigator, overlapping
//! OS/2-inspired utility windows, and bottom taskbar.
//!
//! DISPLAYD will eventually render the real shell through ForgeGraphics.  The
//! kernel keeps this tiny fallback so graphics regressions remain visible even
//! before a native GPU driver or userspace compositor is available.

use crate::framebuffer::Framebuffer;

const REF_W: u32 = 1672;
const REF_H: u32 = 941;

// Canonical K12 "blue steel / Workplace Shell" palette derived from the
// user-approved Titanweave interface reference.
const DESKTOP: u32 = 0x173A60;
const DESKTOP_DARK: u32 = 0x0B2745;
const TOPBAR: u32 = 0x062544;
const TOPBAR_EDGE: u32 = 0x4C84B8;
const PANEL: u32 = 0xD7DCE1;
const PANEL_LIGHT: u32 = 0xF1F2F3;
const PANEL_DARK: u32 = 0x777F87;
const TITLEBAR: u32 = 0x2366A6;
const TITLEBAR_DARK: u32 = 0x124C83;
const SELECTED: u32 = 0x0D4A87;
const INK: u32 = 0x16212B;
const WHITE: u32 = 0xF7F8FA;
const GREEN: u32 = 0x3B9A56;
const PURPLE: u32 = 0x8B4CD6;
const CYAN: u32 = 0x34A6D8;

#[derive(Clone, Copy, Debug)]
pub struct WorkplacePreviewReport {
    pub windows: u32,
    pub navigator_rows: u32,
    pub task_buttons: u32,
}

#[inline]
fn sx(value: u32, width: u32) -> u32 {
    ((u64::from(value) * u64::from(width)) / u64::from(REF_W)) as u32
}

#[inline]
fn sy(value: u32, height: u32) -> u32 {
    ((u64::from(value) * u64::from(height)) / u64::from(REF_H)) as u32
}

fn frame_rect(fb: &mut Framebuffer, x: u32, y: u32, w: u32, h: u32, border: u32, fill: u32) {
    if w < 2 || h < 2 {
        return;
    }
    fb.fill_rect(x, y, w, h, fill);
    fb.horizontal_line(x, y, w, border);
    fb.horizontal_line(x, y.saturating_add(h.saturating_sub(1)), w, border);
    fb.vertical_line(x, y, h, border);
    fb.vertical_line(x.saturating_add(w.saturating_sub(1)), y, h, border);
}

fn title_window(fb: &mut Framebuffer, x: u32, y: u32, w: u32, h: u32, rows: u32, seed: u32) {
    frame_rect(fb, x, y, w, h, PANEL_DARK, PANEL_LIGHT);
    let title_h = (h / 11).clamp(12, 28);
    fb.fill_rect(x.saturating_add(1), y.saturating_add(1), w.saturating_sub(2), title_h, TITLEBAR);
    fb.fill_rect(x.saturating_add(7), y.saturating_add(5), (w / 3).max(20), 3, WHITE);

    // OS/2-style caption buttons.
    let button = title_h.saturating_sub(6).clamp(6, 18);
    for index in 0..3u32 {
        let bx = x.saturating_add(w).saturating_sub(5).saturating_sub((index + 1) * (button + 3));
        frame_rect(fb, bx, y.saturating_add(4), button, button, PANEL_DARK, PANEL);
    }

    let body_y = y.saturating_add(title_h).saturating_add(7);
    let inner_w = w.saturating_sub(18);
    let row_h = ((h.saturating_sub(title_h + 16)) / rows.max(1)).max(5);
    for row in 0..rows {
        let ry = body_y.saturating_add(row * row_h);
        if ry + 3 >= y.saturating_add(h) {
            break;
        }
        let inset = 8 + ((row.wrapping_mul(13).wrapping_add(seed)) % 35);
        let line_w = inner_w.saturating_mul(55 + ((row + seed) % 35)) / 100;
        fb.fill_rect(x.saturating_add(inset), ry, line_w.saturating_sub(inset.min(line_w)), 2, INK);
        if row % 3 == 0 {
            fb.fill_rect(x.saturating_add(w.saturating_sub(24)), ry, 5, 5, GREEN);
        }
    }
}

fn draw_center_mark(fb: &mut Framebuffer, width: u32, height: u32) {
    let cx = width / 2;
    let cy = sy(205, height);
    let span = sx(170, width).clamp(70, 260);
    let thick = sx(9, width).clamp(3, 12);

    // Broad, interlocked T / woven geometry.  This is intentionally geometric;
    // the exact canonical logo remains an external branded resource.
    fb.fill_rect(cx.saturating_sub(span / 2), cy, span, thick, PANEL_LIGHT);
    fb.fill_rect(cx.saturating_sub(span / 2), cy.saturating_add(thick + 3), span, thick / 2 + 1, CYAN);
    fb.fill_rect(cx.saturating_sub(thick / 2), cy, thick, span / 2, PANEL_LIGHT);

    let wing = span / 2;
    for step in 0..wing {
        let y = cy.saturating_add(22).saturating_add(step / 2);
        let left = cx.saturating_sub(wing).saturating_add(step);
        let right = cx.saturating_add(wing).saturating_sub(step);
        if step % 3 == 0 {
            fb.fill_rect(left, y, thick / 2 + 1, 2, CYAN);
            fb.fill_rect(right, y, thick / 2 + 1, 2, PURPLE);
        }
    }
}

/// Render the canonical K12 Workplace Shell layout reference into the firmware
/// framebuffer.  This does not imply the shell runs in-kernel; it is a visual
/// qualification target until DISPLAYD owns scanout composition.
pub fn render_preview(fb: &mut Framebuffer) -> WorkplacePreviewReport {
    let width = fb.width();
    let height = fb.height();
    if width < 640 || height < 400 {
        fb.draw_boot_card();
        return WorkplacePreviewReport { windows: 0, navigator_rows: 0, task_buttons: 0 };
    }

    fb.clear(DESKTOP);

    // Subtle desktop bands evoke the approved blue-steel wallpaper without
    // embedding a bitmap in the kernel.
    for band in 0..7u32 {
        let y = sy(120 + band * 95, height);
        fb.fill_rect(0, y, width, sy(14, height).max(2), if band % 2 == 0 { DESKTOP_DARK } else { TITLEBAR_DARK });
    }

    draw_center_mark(fb, width, height);

    // Top global status bar.
    let top_h = sy(30, height).clamp(18, 42);
    fb.fill_rect(0, 0, width, top_h, TOPBAR);
    fb.horizontal_line(0, top_h.saturating_sub(1), width, TOPBAR_EDGE);
    fb.fill_rect(sx(8, width), sy(7, height), sx(18, width).max(6), sy(16, height).max(5), CYAN);
    fb.fill_rect(sx(38, width), sy(9, height), sx(120, width).max(32), 3, WHITE);
    fb.fill_rect(width.saturating_sub(sx(360, width)), sy(9, height), sx(330, width), 2, PANEL_LIGHT);

    // Left Workplace Navigator rail.
    let side_x = sx(10, width);
    let side_y = sy(40, height);
    let side_w = sx(188, width).clamp(120, width / 4);
    let side_h = height.saturating_sub(side_y + sy(70, height));
    frame_rect(fb, side_x, side_y, side_w, side_h, PANEL_DARK, PANEL);
    fb.fill_rect(side_x + 8, side_y + 10, side_w.saturating_sub(16), sy(70, height).clamp(36, 90), PANEL_LIGHT);
    fb.fill_rect(side_x + 18, side_y + 18, sx(34, width).max(16), sx(34, width).max(16), TITLEBAR);
    fb.fill_rect(side_x + 60, side_y + 22, side_w.saturating_sub(72), 4, INK);
    fb.fill_rect(side_x + 60, side_y + 34, side_w.saturating_sub(92), 2, PANEL_DARK);

    let nav_start = side_y + sy(105, height).clamp(70, 125);
    let nav_row_h = sy(28, height).clamp(18, 34);
    let navigator_rows = 10u32;
    for row in 0..navigator_rows {
        let y = nav_start + row * nav_row_h;
        if y + nav_row_h >= side_y + side_h.saturating_sub(110) {
            break;
        }
        if row == 0 {
            fb.fill_rect(side_x + 8, y, side_w.saturating_sub(16), nav_row_h.saturating_sub(2), SELECTED);
        }
        fb.fill_rect(side_x + 18, y + 6, 10, 10, if row == 0 { WHITE } else { TITLEBAR });
        fb.fill_rect(side_x + 38, y + 9, side_w.saturating_sub(55), 2, if row == 0 { WHITE } else { INK });
    }

    // Quick-launch/status blocks at bottom of navigator.
    let quick_y = side_y + side_h.saturating_sub(sy(235, height).clamp(130, 260));
    fb.horizontal_line(side_x + 8, quick_y, side_w.saturating_sub(16), PANEL_DARK);
    for row in 0..6u32 {
        let y = quick_y + 18 + row * 18;
        fb.fill_rect(side_x + 18, y, 7, 7, if row % 2 == 0 { TITLEBAR } else { GREEN });
        fb.fill_rect(side_x + 34, y + 2, side_w.saturating_sub(50), 2, INK);
    }

    // Major desktop windows positioned to match the approved reference.
    title_window(fb, sx(210, width), sy(40, height), sx(405, width), sy(342, height), 12, 1);
    title_window(fb, sx(1005, width), sy(40, height), sx(615, width), sy(348, height), 11, 2);
    title_window(fb, sx(210, width), sy(390, height), sx(595, width), sy(278, height), 10, 3);
    title_window(fb, sx(825, width), sy(390, height), sx(305, width), sy(270, height), 8, 4);
    title_window(fb, sx(1140, width), sy(390, height), sx(490, width), sy(270, height), 8, 5);
    title_window(fb, sx(210, width), sy(672, height), sx(405, width), sy(198, height), 7, 6);
    title_window(fb, sx(625, width), sy(672, height), sx(500, width), sy(198, height), 7, 7);
    title_window(fb, sx(1135, width), sy(672, height), sx(495, width), sy(198, height), 7, 8);

    // Bottom OS/2-inspired taskbar.
    let task_h = sy(58, height).clamp(30, 74);
    let task_y = height.saturating_sub(task_h);
    fb.fill_rect(0, task_y, width, task_h, PANEL);
    fb.horizontal_line(0, task_y, width, WHITE);
    frame_rect(fb, sx(12, width), task_y + 8, sx(82, width).max(55), task_h.saturating_sub(16), PANEL_DARK, PANEL_LIGHT);
    fb.fill_rect(sx(24, width), task_y + task_h / 2, sx(50, width).max(24), 3, INK);

    let task_buttons = 8u32;
    let button_x = sx(108, width);
    let button_w = sx(145, width).clamp(70, 190);
    let gap = sx(8, width).max(4);
    for index in 0..task_buttons {
        let x = button_x + index * (button_w + gap);
        if x + button_w >= width.saturating_sub(sx(120, width)) {
            break;
        }
        frame_rect(fb, x, task_y + 8, button_w, task_h.saturating_sub(16), PANEL_DARK, PANEL_LIGHT);
        fb.fill_rect(x + 10, task_y + task_h / 2, button_w.saturating_sub(20), 2, if index == 1 { SELECTED } else { INK });
    }

    // Right-side status tray.
    let tray_w = sx(115, width).max(70);
    frame_rect(fb, width.saturating_sub(tray_w + 10), task_y + 8, tray_w, task_h.saturating_sub(16), PANEL_DARK, PANEL_LIGHT);
    fb.fill_rect(width.saturating_sub(tray_w), task_y + 16, 7, 7, GREEN);
    fb.fill_rect(width.saturating_sub(tray_w - 14), task_y + 16, 7, 7, TITLEBAR);

    WorkplacePreviewReport {
        windows: 8,
        navigator_rows,
        task_buttons,
    }
}
