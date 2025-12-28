//! Text 组件 - 文本显示

use crate::{Canvas, Color, Paint, PaintStyle};
use super::component::{Component, ComponentId, Style};

/// Text - 文本组件
pub struct Text {
    id: ComponentId,
    style: Style,
    content: String,
    font_size: f32,
    text_color: Color,
    font_weight: FontWeight,
    text_align: TextAlign,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontWeight {
    Normal,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

impl Text {
    pub fn new(content: &str) -> Self {
        Self {
            id: ComponentId::new(),
            style: Style::default(),
            content: content.to_string(),
            font_size: 16.0,
            text_color: Color::BLACK,
            font_weight: FontWeight::Normal,
            text_align: TextAlign::Left,
        }
    }
    
    pub fn with_frame(mut self, x: f32, y: f32, width: f32, height: f32) -> Self {
        self.style.x = x;
        self.style.y = y;
        self.style.width = width;
        self.style.height = height;
        self
    }
    
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }
    
    pub fn with_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }
    
    pub fn with_font_weight(mut self, weight: FontWeight) -> Self {
        self.font_weight = weight;
        self
    }
    
    pub fn with_text_align(mut self, align: TextAlign) -> Self {
        self.text_align = align;
        self
    }
    
    pub fn set_content(&mut self, content: &str) {
        self.content = content.to_string();
    }
    
    pub fn content(&self) -> &str {
        &self.content
    }
}

impl Component for Text {
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
        // 简单的位图文本渲染（每个字符用矩形表示）
        // 实际项目中应该使用 TextRenderer
        let bounds = self.style.content_bounds();
        let char_width = self.font_size * 0.6;
        let char_height = self.font_size;
        
        let total_width = self.content.chars().count() as f32 * char_width;
        let start_x = match self.text_align {
            TextAlign::Left => bounds.x,
            TextAlign::Center => bounds.x + (bounds.width - total_width) / 2.0,
            TextAlign::Right => bounds.x + bounds.width - total_width,
        };
        let start_y = bounds.y + (bounds.height - char_height) / 2.0;
        
        let paint = Paint::new()
            .with_color(self.text_color)
            .with_style(PaintStyle::Fill);
        
        // 简化：用小矩形表示每个字符
        // 真实实现应该用 fontdue 渲染
        for (i, _ch) in self.content.chars().enumerate() {
            let x = start_x + i as f32 * char_width;
            // 绘制简单的字符占位符
            let char_rect = crate::Rect::new(
                x + 1.0,
                start_y + 2.0,
                char_width - 2.0,
                char_height - 4.0
            );
            canvas.draw_rect(&char_rect, &paint);
        }
    }
    
    fn type_name(&self) -> &'static str {
        "Text"
    }
}
