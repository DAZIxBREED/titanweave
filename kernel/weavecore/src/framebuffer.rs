//! K13 firmware framebuffer fallback.
//!
//! UEFI GOP provides Titanweave with a standards-based linear scanout surface
//! before native GPU drivers exist.  This module deliberately keeps the
//! fallback tiny: bounds-checked 32-bpp pixel access, rectangles, lines, and a
//! boot proof.  The permanent compositor is a userspace service.

use titanweave_boot_protocol::{framebuffer_pixel_format, BootInfo, FramebufferInfo};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    Rgbx8888,
    Bgrx8888,
}

#[derive(Clone, Copy, Debug)]
pub struct Framebuffer {
    base: u64,
    byte_size: u64,
    width: u32,
    height: u32,
    stride: u32,
    format: PixelFormat,
}

impl Framebuffer {
    pub fn from_boot_info(boot_info: &BootInfo) -> Result<Self, &'static str> {
        let info = boot_info.framebuffer;
        if info.is_empty() {
            return Err("UEFI GOP framebuffer was not supplied");
        }
        if !info.is_linear_32bpp() {
            return Err("UEFI framebuffer is not a supported 32-bpp linear format");
        }
        let end = info
            .base_address
            .checked_add(info.byte_size)
            .ok_or("framebuffer address overflow")?;
        if end > boot_info.bootstrap.identity_map_limit {
            return Err("framebuffer lies outside K13 bootstrap identity map");
        }
        let format = match info.pixel_format {
            framebuffer_pixel_format::RGBX8888 => PixelFormat::Rgbx8888,
            framebuffer_pixel_format::BGRX8888 => PixelFormat::Bgrx8888,
            _ => return Err("unsupported framebuffer pixel format"),
        };
        Ok(Self {
            base: info.base_address,
            byte_size: info.byte_size,
            width: info.width,
            height: info.height,
            stride: info.stride,
            format,
        })
    }

    #[must_use]
    pub const fn info(&self) -> FramebufferInfo {
        FramebufferInfo {
            base_address: self.base,
            byte_size: self.byte_size,
            width: self.width,
            height: self.height,
            stride: self.stride,
            pixel_format: match self.format {
                PixelFormat::Rgbx8888 => framebuffer_pixel_format::RGBX8888,
                PixelFormat::Bgrx8888 => framebuffer_pixel_format::BGRX8888,
            },
        }
    }

    #[must_use]
    pub const fn width(&self) -> u32 { self.width }

    #[must_use]
    pub const fn height(&self) -> u32 { self.height }

    pub fn clear(&mut self, rgb: u32) {
        self.fill_rect(0, 0, self.width, self.height, rgb);
    }

    pub fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32, rgb: u32) {
        let x1 = x.saturating_add(width).min(self.width);
        let y1 = y.saturating_add(height).min(self.height);
        for py in y..y1 {
            for px in x..x1 {
                self.put_pixel(px, py, rgb);
            }
        }
    }

    pub fn horizontal_line(&mut self, x: u32, y: u32, width: u32, rgb: u32) {
        self.fill_rect(x, y, width, 1, rgb);
    }

    pub fn vertical_line(&mut self, x: u32, y: u32, height: u32, rgb: u32) {
        self.fill_rect(x, y, 1, height, rgb);
    }

    pub fn put_pixel(&mut self, x: u32, y: u32, rgb: u32) {
        if x >= self.width || y >= self.height {
            return;
        }
        let pixel_index = (y as u64)
            .saturating_mul(self.stride as u64)
            .saturating_add(x as u64);
        let offset = pixel_index.saturating_mul(4);
        if offset.saturating_add(4) > self.byte_size {
            return;
        }
        let r = ((rgb >> 16) & 0xff) as u8;
        let g = ((rgb >> 8) & 0xff) as u8;
        let b = (rgb & 0xff) as u8;
        let packed = match self.format {
            PixelFormat::Rgbx8888 => u32::from_le_bytes([r, g, b, 0]),
            PixelFormat::Bgrx8888 => u32::from_le_bytes([b, g, r, 0]),
        };
        // SAFETY: constructor validated the complete linear framebuffer range
        // against the identity map and every write is bounds checked above.
        unsafe {
            core::ptr::write_volatile((self.base + offset) as *mut u32, packed);
        }
    }

    /// Draw the K13 fallback boot card.  This is intentionally geometric rather
    /// than a baked bitmap; DISPLAYD will own branded resources after bootstrap.
    pub fn draw_boot_card(&mut self) {
        const BG: u32 = 0x030307;
        const PANEL: u32 = 0x080814;
        const PURPLE: u32 = 0x9A2CFF;
        const BLUE: u32 = 0x168CFF;
        const WHITE: u32 = 0xF4F2FF;
        self.clear(BG);

        let card_w = self.width.min(760).max(240);
        let card_h = self.height.min(300).max(160);
        let x = self.width.saturating_sub(card_w) / 2;
        let y = self.height.saturating_sub(card_h) / 2;
        self.fill_rect(x, y, card_w, card_h, PANEL);
        self.horizontal_line(x, y, card_w, PURPLE);
        self.horizontal_line(x, y + card_h.saturating_sub(1), card_w, BLUE);

        // Simplified geometric Titanweave mark inspired by the canonical logo:
        // three descending woven chevrons, deliberately kept as a fallback.
        let mark_x = x + card_w / 2;
        let mark_y = y + 36;
        let span = (card_w / 5).clamp(44, 120);
        let thickness = 6u32;
        for row in 0..3u32 {
            let top = mark_y + row * 22;
            let inset = row * 12;
            for step in 0..span.saturating_sub(inset * 2) {
                let left = mark_x.saturating_sub(span / 2).saturating_add(inset).saturating_add(step);
                let right = mark_x.saturating_add(span / 2).saturating_sub(inset).saturating_sub(step);
                let dy = step / 3;
                self.fill_rect(left, top + dy, thickness, 2, if row == 1 { BLUE } else { PURPLE });
                self.fill_rect(right, top + dy, thickness, 2, if row == 1 { PURPLE } else { BLUE });
            }
        }

        // Wordmark bar.  Full canonical artwork lives in DISPLAYD resources;
        // this bright bar makes framebuffer success visually obvious in QEMU.
        let word_w = (card_w * 2 / 3).max(120);
        let word_x = x + (card_w - word_w) / 2;
        let word_y = y + card_h.saturating_sub(72);
        self.fill_rect(word_x, word_y, word_w, 4, WHITE);
        self.fill_rect(word_x + word_w / 4, word_y + 12, word_w / 2, 2, PURPLE);
    }
}
