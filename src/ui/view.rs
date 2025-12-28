//! View 组件 - 基础容器

use crate::{Canvas, Color, Paint, PaintStyle, Path};
use super::component::{Component, ComponentId, Style};

/// View - 基础容器组件
pub struct View {
    id: ComponentId,
    style: Style,
    children: Vec<Box<dyn Component>>,
}

impl View {
    pub fn new() -> Self {
        Self {
            id: ComponentId::new(),
            style: Style::default(),
            children: Vec::new(),
        }
    }
    
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
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
    
    pub fn with_border(mut self, color: Color, width: f32) -> Self {
        self.style.border_color = Some(color);
        self.style.border_width = width;
        self
    }
    
    pub fn with_border_radius(mut self, radius: f32) -> Self {
        self.style.border_radius = radius;
        self
    }
    
    pub fn with_padding(mut self, padding: f32) -> Self {
        self.style.padding = [padding; 4];
        self
    }
}

impl Default for View {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for View {
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
        
        // 绘制背景
        if let Some(bg_color) = self.style.background_color {
            let paint = Paint::new()
                .with_color(Color::new(
                    bg_color.r, bg_color.g, bg_color.b,
                    (bg_color.a as f32 * self.style.opacity) as u8
                ))
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
        }
        
        // 绘制边框
        if let Some(border_color) = self.style.border_color {
            if self.style.border_width > 0.0 {
                let paint = Paint::new()
                    .with_color(border_color)
                    .with_style(PaintStyle::Stroke)
                    .with_stroke_width(self.style.border_width);
                
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
            }
        }
    }
    
    fn children(&self) -> &[Box<dyn Component>] {
        &self.children
    }
    
    fn children_mut(&mut self) -> &mut Vec<Box<dyn Component>> {
        &mut self.children
    }
    
    fn add_child(&mut self, child: Box<dyn Component>) {
        self.children.push(child);
    }
    
    fn type_name(&self) -> &'static str {
        "View"
    }
}
