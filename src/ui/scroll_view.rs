//! ScrollView 组件 - 可滚动容器

use crate::{Canvas, Color, Paint, PaintStyle, Point, Rect};
use crate::event::Event;
use super::component::{Component, ComponentId, Style};

/// ScrollView - 可滚动容器
pub struct ScrollView {
    id: ComponentId,
    style: Style,
    children: Vec<Box<dyn Component>>,
    scroll_x: f32,
    scroll_y: f32,
    content_width: f32,
    content_height: f32,
    scroll_enabled_x: bool,
    scroll_enabled_y: bool,
    show_scrollbar: bool,
    // 滚动状态
    is_scrolling: bool,
    last_touch: Option<Point>,
    velocity_y: f32,
}

impl ScrollView {
    pub fn new() -> Self {
        Self {
            id: ComponentId::new(),
            style: Style::default(),
            children: Vec::new(),
            scroll_x: 0.0,
            scroll_y: 0.0,
            content_width: 0.0,
            content_height: 0.0,
            scroll_enabled_x: false,
            scroll_enabled_y: true,
            show_scrollbar: true,
            is_scrolling: false,
            last_touch: None,
            velocity_y: 0.0,
        }
    }
    
    pub fn with_frame(mut self, x: f32, y: f32, width: f32, height: f32) -> Self {
        self.style.x = x;
        self.style.y = y;
        self.style.width = width;
        self.style.height = height;
        self
    }
    
    pub fn with_content_size(mut self, width: f32, height: f32) -> Self {
        self.content_width = width;
        self.content_height = height;
        self
    }
    
    pub fn with_scroll_x(mut self, enabled: bool) -> Self {
        self.scroll_enabled_x = enabled;
        self
    }
    
    pub fn with_scroll_y(mut self, enabled: bool) -> Self {
        self.scroll_enabled_y = enabled;
        self
    }
    
    pub fn with_show_scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }
    
    pub fn scroll_to(&mut self, x: f32, y: f32) {
        self.scroll_x = x.max(0.0).min(self.max_scroll_x());
        self.scroll_y = y.max(0.0).min(self.max_scroll_y());
    }
    
    fn max_scroll_x(&self) -> f32 {
        (self.content_width - self.style.width).max(0.0)
    }
    
    fn max_scroll_y(&self) -> f32 {
        (self.content_height - self.style.height).max(0.0)
    }
    
    /// 更新惯性滚动
    pub fn update(&mut self, _dt: f32) {
        if !self.is_scrolling && self.velocity_y.abs() > 0.1 {
            self.scroll_y += self.velocity_y;
            self.velocity_y *= 0.95; // 摩擦力
            
            // 边界检查
            if self.scroll_y < 0.0 {
                self.scroll_y = 0.0;
                self.velocity_y = 0.0;
            } else if self.scroll_y > self.max_scroll_y() {
                self.scroll_y = self.max_scroll_y();
                self.velocity_y = 0.0;
            }
        }
    }
}

impl Default for ScrollView {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ScrollView {
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
        if let Some(bg) = self.style.background_color {
            let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
            canvas.draw_rect(&bounds, &paint);
        }
        
        // 设置裁剪区域
        canvas.clip_rect(bounds);
        
        // 渲染子组件（带偏移）
        for child in &self.children {
            if child.style().visible {
                // 计算子组件在滚动后的位置
                let child_bounds = child.style().bounds();
                let visible_y = child_bounds.y - self.scroll_y;
                let visible_x = child_bounds.x - self.scroll_x;
                
                // 简单的可见性检查
                if visible_y + child_bounds.height >= bounds.y &&
                   visible_y <= bounds.bottom() {
                    // 临时修改子组件位置进行渲染
                    // 注意：这里简化处理，实际应该用变换矩阵
                    child.render(canvas);
                }
            }
        }
        
        // 重置裁剪
        canvas.reset_clip();
        
        // 绘制滚动条
        if self.show_scrollbar && self.content_height > self.style.height {
            let scrollbar_height = (self.style.height / self.content_height * self.style.height).max(20.0);
            let scrollbar_y = bounds.y + (self.scroll_y / self.max_scroll_y()) * (self.style.height - scrollbar_height);
            
            let scrollbar_rect = Rect::new(
                bounds.right() - 4.0,
                scrollbar_y,
                3.0,
                scrollbar_height
            );
            
            let paint = Paint::new()
                .with_color(Color::new(0, 0, 0, 80))
                .with_style(PaintStyle::Fill);
            canvas.draw_rect(&scrollbar_rect, &paint);
        }
    }
    
    fn on_event(&mut self, event: &Event) -> bool {
        match event {
            Event::TouchStart(touch) => {
                if let Some(t) = touch.touches.first() {
                    if self.hit_test(&t.position()) {
                        self.is_scrolling = true;
                        self.last_touch = Some(t.position());
                        self.velocity_y = 0.0;
                        return true;
                    }
                }
            }
            Event::TouchMove(touch) => {
                if self.is_scrolling {
                    if let (Some(t), Some(last)) = (touch.touches.first(), &self.last_touch) {
                        let dy = last.y - t.y;
                        let dx = last.x - t.x;
                        
                        if self.scroll_enabled_y {
                            self.scroll_y = (self.scroll_y + dy).max(0.0).min(self.max_scroll_y());
                            self.velocity_y = dy;
                        }
                        if self.scroll_enabled_x {
                            self.scroll_x = (self.scroll_x + dx).max(0.0).min(self.max_scroll_x());
                        }
                        
                        self.last_touch = Some(t.position());
                        return true;
                    }
                }
            }
            Event::TouchEnd(_) | Event::TouchCancel(_) => {
                self.is_scrolling = false;
                self.last_touch = None;
            }
            _ => {}
        }
        
        false
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
        "ScrollView"
    }
}
