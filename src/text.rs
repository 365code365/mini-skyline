//! 文本渲染模块 - 支持系统字体和高清渲染

use crate::{Canvas, Color, Paint};
use fontdue::{Font, FontSettings};
use std::path::Path;

/// 文本渲染器
pub struct TextRenderer {
    font: Font,
}

impl TextRenderer {
    /// 从字体数据创建
    pub fn from_bytes(font_data: &[u8]) -> Result<Self, String> {
        let settings = FontSettings {
            scale: 40.0,  // 提高默认缩放以获得更好的渲染质量
            ..Default::default()
        };
        let font = Font::from_bytes(font_data, settings)
            .map_err(|e| e.to_string())?;
        Ok(Self { font })
    }
    
    /// 从文件路径加载字体
    pub fn from_file(path: &str) -> Result<Self, String> {
        let font_data = std::fs::read(path)
            .map_err(|e| format!("Failed to read font file: {}", e))?;
        Self::from_bytes(&font_data)
    }
    
    /// 加载系统字体（macOS）
    pub fn load_system_font() -> Result<Self, String> {
        // macOS 系统字体路径优先级 - 优先使用更清晰的字体
        let font_paths = [
            // Noto Sans SC - 开源高质量中文字体
            "/Library/Fonts/NotoSansSC-Regular.ttf",
            "/Library/Fonts/NotoSansSC-Regular.otf",
            // Source Han Sans - 思源黑体
            "/Library/Fonts/SourceHanSansSC-Regular.otf",
            // PingFang SC - 苹方简体（系统默认中文）
            "/System/Library/Fonts/PingFang.ttc",
            // Hiragino Sans - 冬青黑体
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/Library/Fonts/Hiragino Sans GB W3.otf",
            // SF Pro - 系统默认英文字体
            "/System/Library/Fonts/SFNS.ttf",
            "/System/Library/Fonts/SFNSText.ttf",
            // Helvetica Neue
            "/System/Library/Fonts/HelveticaNeue.ttc",
            // STHeiti - 华文黑体（备用）
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/System/Library/Fonts/STHeiti Medium.ttc",
            // Arial Unicode - 最后备用
            "/Library/Fonts/Arial Unicode.ttf",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ];
        
        for path in &font_paths {
            if Path::new(path).exists() {
                match Self::from_file(path) {
                    Ok(renderer) => {
                        println!("✅ Loaded system font: {}", path);
                        return Ok(renderer);
                    }
                    Err(e) => {
                        println!("⚠️ Failed to load {}: {}", path, e);
                    }
                }
            }
        }
        
        Err("No system font found".to_string())
    }

    /// 渲染文本到画布（带亚像素抗锯齿）
    pub fn draw_text(&self, canvas: &mut Canvas, text: &str, x: f32, y: f32, size: f32, paint: &Paint) {
        let mut cursor_x = x;

        for ch in text.chars() {
            let (metrics, bitmap) = self.font.rasterize(ch, size);
            
            if metrics.width == 0 || metrics.height == 0 {
                cursor_x += metrics.advance_width;
                continue;
            }

            // 计算字形位置 - y 是基线位置
            let glyph_x = cursor_x + metrics.xmin as f32;
            let glyph_y = y - metrics.height as f32 - metrics.ymin as f32;

            for gy in 0..metrics.height {
                for gx in 0..metrics.width {
                    let coverage = bitmap[gy * metrics.width + gx] as f32 / 255.0;
                    
                    // 使用更低的阈值以获得更平滑的边缘
                    if coverage > 0.001 {
                        let px = (glyph_x + gx as f32).round() as i32;
                        let py = (glyph_y + gy as f32).round() as i32;

                        if px >= 0 && py >= 0 && px < canvas.width() as i32 && py < canvas.height() as i32 {
                            // Gamma 校正以获得更好的视觉效果
                            let gamma_coverage = coverage.powf(0.8);
                            let alpha = (paint.color.a as f32 * gamma_coverage) as u8;
                            
                            if alpha > 0 {
                                let color = Color::new(paint.color.r, paint.color.g, paint.color.b, alpha);
                                let dst = canvas.get_pixel(px as u32, py as u32);
                                let blended = color.blend(&dst);
                                canvas.set_pixel_direct(px, py, blended);
                            }
                        }
                    }
                }
            }

            cursor_x += metrics.advance_width;
        }
    }

    /// 测量文本宽度
    pub fn measure_text(&self, text: &str, size: f32) -> f32 {
        let mut width = 0.0;
        for ch in text.chars() {
            let metrics = self.font.metrics(ch, size);
            width += metrics.advance_width;
        }
        width
    }
    
    /// 测量文本高度
    pub fn measure_height(&self, size: f32) -> f32 {
        // 使用大写字母 M 作为参考
        let metrics = self.font.metrics('M', size);
        metrics.height as f32
    }
}
