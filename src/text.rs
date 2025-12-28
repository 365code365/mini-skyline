//! 文本渲染模块 - 支持系统字体、Emoji 和高清渲染

use crate::{Canvas, Color, Paint};
use fontdue::{Font, FontSettings};
use std::path::Path;

/// 文本渲染器 - 支持多字体回退（中文 + Emoji）
pub struct TextRenderer {
    /// 主字体（中文/英文）
    main_font: Font,
    /// Emoji 字体
    emoji_font: Option<Font>,
}

impl TextRenderer {
    /// 从字体数据创建
    pub fn from_bytes(font_data: &[u8]) -> Result<Self, String> {
        let settings = FontSettings {
            scale: 40.0,
            ..Default::default()
        };
        let font = Font::from_bytes(font_data, settings)
            .map_err(|e| e.to_string())?;
        Ok(Self { 
            main_font: font,
            emoji_font: None,
        })
    }
    
    /// 从文件路径加载字体
    pub fn from_file(path: &str) -> Result<Self, String> {
        let font_data = std::fs::read(path)
            .map_err(|e| format!("Failed to read font file: {}", e))?;
        Self::from_bytes(&font_data)
    }
    
    /// 加载系统字体（macOS）- 包含 Emoji 支持
    pub fn load_system_font() -> Result<Self, String> {
        // 主字体路径（中文优先）
        let main_font_paths = [
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
            "/System/Library/Fonts/STHeiti Light.ttc",
        ];
        
        // Emoji 字体路径
        let emoji_font_paths = [
            "/System/Library/Fonts/Apple Color Emoji.ttc",
            "/System/Library/Fonts/AppleColorEmoji.ttf",
        ];
        
        // 加载主字体
        let mut renderer: Option<TextRenderer> = None;
        for path in &main_font_paths {
            if Path::new(path).exists() {
                match Self::from_file(path) {
                    Ok(r) => {
                        println!("✅ Main font: {}", path);
                        renderer = Some(r);
                        break;
                    }
                    Err(_) => continue,
                }
            }
        }
        
        let mut renderer = renderer.ok_or("No main font found")?;
        
        // 加载 Emoji 字体
        for path in &emoji_font_paths {
            if Path::new(path).exists() {
                if let Ok(data) = std::fs::read(path) {
                    let settings = FontSettings {
                        scale: 40.0,
                        ..Default::default()
                    };
                    if let Ok(font) = Font::from_bytes(data.as_slice(), settings) {
                        println!("✅ Emoji font: {}", path);
                        renderer.emoji_font = Some(font);
                        break;
                    }
                }
            }
        }
        
        Ok(renderer)
    }

    /// 判断字符是否为 Emoji
    fn is_emoji(ch: char) -> bool {
        let code = ch as u32;
        // Emoji 范围（简化版）
        matches!(code,
            0x1F300..=0x1F9FF |  // Misc Symbols, Emoticons, etc.
            0x2600..=0x26FF |    // Misc Symbols
            0x2700..=0x27BF |    // Dingbats
            0xFE00..=0xFE0F |    // Variation Selectors
            0x1F000..=0x1F02F |  // Mahjong, Domino
            0x1F0A0..=0x1F0FF |  // Playing Cards
            0x1F100..=0x1F1FF |  // Enclosed Alphanumerics
            0x1F200..=0x1F2FF |  // Enclosed Ideographic
            0x1FA00..=0x1FAFF |  // Chess, Extended-A
            0x231A..=0x231B |    // Watch, Hourglass
            0x23E9..=0x23FA |    // Media controls
            0x25AA..=0x25FE |    // Squares
            0x2934..=0x2935 |
            0x2B05..=0x2B07 |
            0x2B1B..=0x2B1C |
            0x2B50 | 0x2B55 |
            0x3030 | 0x303D |
            0x3297 | 0x3299
        )
    }

    /// 渲染文本到画布
    pub fn draw_text(&self, canvas: &mut Canvas, text: &str, x: f32, y: f32, size: f32, paint: &Paint) {
        let mut cursor_x = x;

        for ch in text.chars() {
            // 选择字体
            let font = if Self::is_emoji(ch) {
                self.emoji_font.as_ref().unwrap_or(&self.main_font)
            } else {
                &self.main_font
            };
            
            let (metrics, bitmap) = font.rasterize(ch, size);
            
            if metrics.width == 0 || metrics.height == 0 {
                cursor_x += metrics.advance_width;
                continue;
            }

            let glyph_x = cursor_x + metrics.xmin as f32;
            let glyph_y = y - metrics.height as f32 - metrics.ymin as f32;

            for gy in 0..metrics.height {
                for gx in 0..metrics.width {
                    let coverage = bitmap[gy * metrics.width + gx] as f32 / 255.0;
                    
                    if coverage > 0.001 {
                        let px = (glyph_x + gx as f32).round() as i32;
                        let py = (glyph_y + gy as f32).round() as i32;

                        if px >= 0 && py >= 0 && px < canvas.width() as i32 && py < canvas.height() as i32 {
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
            let font = if Self::is_emoji(ch) {
                self.emoji_font.as_ref().unwrap_or(&self.main_font)
            } else {
                &self.main_font
            };
            let metrics = font.metrics(ch, size);
            width += metrics.advance_width;
        }
        width
    }
    
    /// 测量文本高度
    pub fn measure_height(&self, size: f32) -> f32 {
        let metrics = self.main_font.metrics('M', size);
        metrics.height as f32
    }
    
    /// 自动换行绘制文本
    pub fn draw_text_wrapped(&self, canvas: &mut Canvas, text: &str, x: f32, y: f32, size: f32, max_width: f32, paint: &Paint) {
        if max_width <= 0.0 {
            self.draw_text(canvas, text, x, y, size, paint);
            return;
        }
        
        let line_height = size * 1.4; // 行高
        let mut current_y = y;
        let mut line_start = 0;
        let chars: Vec<char> = text.chars().collect();
        let mut current_width = 0.0;
        
        for (i, ch) in chars.iter().enumerate() {
            let font = if Self::is_emoji(*ch) {
                self.emoji_font.as_ref().unwrap_or(&self.main_font)
            } else {
                &self.main_font
            };
            let metrics = font.metrics(*ch, size);
            let char_width = metrics.advance_width;
            
            // 检查是否需要换行
            if current_width + char_width > max_width && i > line_start {
                // 绘制当前行
                let line: String = chars[line_start..i].iter().collect();
                self.draw_text(canvas, &line, x, current_y, size, paint);
                
                // 移动到下一行
                current_y += line_height;
                line_start = i;
                current_width = char_width;
            } else {
                current_width += char_width;
            }
        }
        
        // 绘制最后一行
        if line_start < chars.len() {
            let line: String = chars[line_start..].iter().collect();
            self.draw_text(canvas, &line, x, current_y, size, paint);
        }
    }
}
