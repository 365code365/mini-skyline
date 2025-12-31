//! Image 组件 - 图片显示

use crate::{Canvas, Color, Paint, PaintStyle};
use super::component::{Component, ComponentId, Style};

/// 图片缩放模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageMode {
    /// 缩放到填满，可能裁剪
    Cover,
    /// 缩放到完全显示，可能留白
    Contain,
    /// 拉伸填满
    Fill,
    /// 保持原始尺寸
    None,
}

/// Image - 图片组件
pub struct Image {
    id: ComponentId,
    style: Style,
    src: String,
    mode: ImageMode,
    pixels: Option<Vec<Color>>,
    image_width: u32,
    image_height: u32,
}

impl Image {
    pub fn new(src: &str) -> Self {
        Self {
            id: ComponentId::new(),
            style: Style::default(),
            src: src.to_string(),
            mode: ImageMode::Cover,
            pixels: None,
            image_width: 0,
            image_height: 0,
        }
    }
    
    pub fn with_frame(mut self, x: f32, y: f32, width: f32, height: f32) -> Self {
        self.style.x = x;
        self.style.y = y;
        self.style.width = width;
        self.style.height = height;
        self
    }
    
    pub fn with_mode(mut self, mode: ImageMode) -> Self {
        self.mode = mode;
        self
    }
    
    /// 从 RGBA 数据加载图片
    pub fn load_rgba(&mut self, data: &[u8], width: u32, height: u32) {
        let mut pixels = Vec::with_capacity((width * height) as usize);
        for chunk in data.chunks(4) {
            if chunk.len() == 4 {
                pixels.push(Color::new(chunk[0], chunk[1], chunk[2], chunk[3]));
            }
        }
        self.pixels = Some(pixels);
        self.image_width = width;
        self.image_height = height;
    }
    
    /// 从文件加载图片
    pub fn load_file(&mut self, path: &str) -> Result<(), String> {
        use image::GenericImageView;
        
        let img = image::open(path).map_err(|e| e.to_string())?;
        let (width, height) = img.dimensions();
        let rgba = img.to_rgba8();
        
        self.load_rgba(&rgba, width, height);
        self.src = path.to_string();
        Ok(())
    }
    
    pub fn src(&self) -> &str {
        &self.src
    }
}

impl Component for Image {
    fn id(&self) -> ComponentId {
        self.id
    }
    
    fn style(&self) -> &Style {
        &self.style
    }
    
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
    
    fn render(&self, canvas: &mut Canvas) {
        let bounds = self.style.bounds();
        
        // 如果没有加载图片，显示占位符
        if self.pixels.is_none() {
            let paint = Paint::new()
                .with_color(Color::from_hex(0xEEEEEE))
                .with_style(PaintStyle::Fill);
            canvas.draw_rect(&bounds, &paint);
            
            // 绘制对角线表示占位
            let line_paint = Paint::new()
                .with_color(Color::from_hex(0xCCCCCC))
                .with_stroke_width(1.0);
            canvas.draw_line(bounds.x, bounds.y, bounds.right(), bounds.bottom(), &line_paint);
            canvas.draw_line(bounds.right(), bounds.y, bounds.x, bounds.bottom(), &line_paint);
            return;
        }
        
        // 简单的图片渲染（最近邻采样）
        if let Some(pixels) = &self.pixels {
            let src_w = self.image_width as f32;
            let src_h = self.image_height as f32;
            let dst_w = bounds.width;
            let dst_h = bounds.height;
            
            // 计算缩放比例
            let (scale_x, scale_y, offset_x, offset_y) = match self.mode {
                ImageMode::Fill => (src_w / dst_w, src_h / dst_h, 0.0, 0.0),
                ImageMode::Contain => {
                    let scale = (src_w / dst_w).max(src_h / dst_h);
                    let ox = (dst_w - src_w / scale) / 2.0;
                    let oy = (dst_h - src_h / scale) / 2.0;
                    (scale, scale, ox, oy)
                }
                ImageMode::Cover => {
                    let scale = (src_w / dst_w).min(src_h / dst_h);
                    (scale, scale, 0.0, 0.0)
                }
                ImageMode::None => (1.0, 1.0, 0.0, 0.0),
            };
            
            for dy in 0..dst_h as i32 {
                for dx in 0..dst_w as i32 {
                    let sx = ((dx as f32 - offset_x) * scale_x) as i32;
                    let sy = ((dy as f32 - offset_y) * scale_y) as i32;
                    
                    if sx >= 0 && sx < self.image_width as i32 &&
                       sy >= 0 && sy < self.image_height as i32 {
                        let idx = (sy as u32 * self.image_width + sx as u32) as usize;
                        if idx < pixels.len() {
                            let color = pixels[idx];
                            canvas.set_pixel(
                                bounds.x as i32 + dx,
                                bounds.y as i32 + dy,
                                color
                            );
                        }
                    }
                }
            }
        }
    }
    
    fn type_name(&self) -> &'static str {
        "Image"
    }
}
