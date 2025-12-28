//! Button 组件 - 可点击按钮

use crate::{Canvas, Color, Paint, PaintStyle, Path, Point, Rect};
use crate::event::Event;
use super::component::{Component, ComponentId, Style};

/// Button - 按钮组件
pub struct Button {
    id: ComponentId,
    style: Style,
    label: String,
    font_size: f32,
    text_color: Color,
    pressed: bool,
    disabled: bool,
    on_tap: Option<Box<dyn Fn() + Send + Sync>>,
}

impl Button {
    pub fn new(label: &str) -> Self {
        let mut style = Style::default();
        style.background_color = Some(Color::from_hex(0x007AFF));
        style.border_radius = 8.0;
        style.width = 120.0;
        style.height = 44.0;
        
        Self {
            id: ComponentId::new(),
            style,
            label: label.to_string(),
            font_size: 16.0,
            text_color: Color::WHITE,
            pressed: false,
            disabled: false,
            on_tap: None,
        }
    }
    
    pub fn with_frame(mut self, x: f32, y: f32, width: f32, height: f32) -> Self {
        self.style.x = x;
        self.style.y = y;
        self.style.width = width;
        self.style.height = height;
        self
    }
    
    pub fn with_background(mut self, color: Color) -> Self {
        self.style.background_color = Some(color);
        self
    }
    
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }
    
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }
    
    pub fn with_border_radius(mut self, radius: f32) -> Self {
        self.style.border_radius = radius;
        self
    }
    
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    
    pub fn on_tap<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_tap = Some(Box::new(callback));
        self
    }
    
    pub fn set_label(&mut self, label: &str) {
        self.label = label.to_string();
    }
    
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }
}

impl Component for Button {
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
        
        // 计算背景颜色
        let bg_color = if self.disabled {
            Color::from_hex(0xCCCCCC)
        } else if self.pressed {
            // 按下时颜色变深
            if let Some(bg) = self.style.background_color {
                Color::new(
                    (bg.r as f32 * 0.8) as u8,
                    (bg.g as f32 * 0.8) as u8,
                    (bg.b as f32 * 0.8) as u8,
                    bg.a
                )
            } else {
                Color::from_hex(0x005BB5)
            }
        } else {
            self.style.background_color.unwrap_or(Color::from_hex(0x007AFF))
        };
        
        // 绘制背景
        let paint = Paint::new()
            .with_color(bg_color)
            .with_style(PaintStyle::Fill);
        
        if self.style.border_radius > 0.0 {
            let mut path = Path::new();
            path.add_round_rect(
                bounds.x, bounds.y, bounds.width, bounds.height,
                self.style.border_radius
            );
            canvas.draw_path(&path, &paint);
        } else {
            canvas.draw_rect(&bounds, &paint);
        }
        
        // 绘制文本（简化版）
        let text_color = if self.disabled {
            Color::from_hex(0x999999)
        } else {
            self.text_color
        };
        
        let char_width = self.font_size * 0.6;
        let char_height = self.font_size;
        let total_width = self.label.chars().count() as f32 * char_width;
        let start_x = bounds.x + (bounds.width - total_width) / 2.0;
        let start_y = bounds.y + (bounds.height - char_height) / 2.0;
        
        let text_paint = Paint::new()
            .with_color(text_color)
            .with_style(PaintStyle::Fill);
        
        for (i, _ch) in self.label.chars().enumerate() {
            let x = start_x + i as f32 * char_width;
            let char_rect = Rect::new(x + 1.0, start_y + 2.0, char_width - 2.0, char_height - 4.0);
            canvas.draw_rect(&char_rect, &text_paint);
        }
    }
    
    fn on_event(&mut self, event: &Event) -> bool {
        if self.disabled {
            return false;
        }
        
        match event {
            Event::TouchStart(touch) => {
                if let Some(t) = touch.touches.first() {
                    if self.hit_test(&t.position()) {
                        self.pressed = true;
                        return true;
                    }
                }
            }
            Event::TouchEnd(touch) => {
                if self.pressed {
                    self.pressed = false;
                    if let Some(t) = touch.changed_touches.first() {
                        if self.hit_test(&t.position()) {
                            // 触发点击回调
                            if let Some(callback) = &self.on_tap {
                                callback();
                            }
                            return true;
                        }
                    }
                }
            }
            Event::TouchCancel(_) => {
                self.pressed = false;
            }
            Event::Tap(tap) => {
                if self.hit_test(&Point::new(tap.x, tap.y)) {
                    if let Some(callback) = &self.on_tap {
                        callback();
                    }
                    return true;
                }
            }
            _ => {}
        }
        
        false
    }
    
    fn type_name(&self) -> &'static str {
        "Button"
    }
}
