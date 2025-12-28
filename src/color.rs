//! 颜色模块

/// RGBA 颜色
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
            a: 255,
        }
    }

    /// 预混合 alpha
    pub fn premultiply(&self) -> Self {
        let a = self.a as f32 / 255.0;
        Self {
            r: (self.r as f32 * a) as u8,
            g: (self.g as f32 * a) as u8,
            b: (self.b as f32 * a) as u8,
            a: self.a,
        }
    }

    /// Alpha 混合
    pub fn blend(&self, dst: &Color) -> Color {
        let src_a = self.a as f32 / 255.0;
        let dst_a = dst.a as f32 / 255.0;
        let out_a = src_a + dst_a * (1.0 - src_a);

        if out_a == 0.0 {
            return Color::new(0, 0, 0, 0);
        }

        Color {
            r: ((self.r as f32 * src_a + dst.r as f32 * dst_a * (1.0 - src_a)) / out_a) as u8,
            g: ((self.g as f32 * src_a + dst.g as f32 * dst_a * (1.0 - src_a)) / out_a) as u8,
            b: ((self.b as f32 * src_a + dst.b as f32 * dst_a * (1.0 - src_a)) / out_a) as u8,
            a: (out_a * 255.0) as u8,
        }
    }

    // 预定义颜色
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const TRANSPARENT: Color = Color::new(0, 0, 0, 0);
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}
