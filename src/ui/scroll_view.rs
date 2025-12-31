//! ScrollView 组件 - 可滚动容器

use crate::{Canvas, Color, Paint, PaintStyle, Point, Rect};
use crate::event::Event;
use super::component::{Component, ComponentId, Style};
use super::scroll_controller::ScrollController;

/// ScrollView - 可滚动容器
pub struct ScrollView {
    id: ComponentId,
    style: Style,
    children: Vec<Box<dyn Component>>,
    scroll_x: f32,
    // scroll_y 由 controller 管理
    controller: ScrollController,
    content_width: f32,
    content_height: f32,
    scroll_enabled_x: bool,
    scroll_enabled_y: bool,
    show_scrollbar: bool,
    // 滚动状态
    last_touch: Option<Point>,
}

impl ScrollView {
    pub fn new() -> Self {
        Self {
            id: ComponentId::new(),
            style: Style::default(),
            children: Vec::new(),
            scroll_x: 0.0,
            controller: ScrollController::new(0.0, 0.0),
            content_width: 0.0,
            content_height: 0.0,
            scroll_enabled_x: false,
            scroll_enabled_y: true,
            show_scrollbar: true,
            last_touch: None,
        }
    }
    
    pub fn with_frame(mut self, x: f32, y: f32, width: f32, height: f32) -> Self {
        self.style.x = x;
        self.style.y = y;
        self.style.width = width;
        self.style.height = height;
        self.controller.update_content_height(self.content_height, height);
        self
    }
    
    pub fn with_content_size(mut self, width: f32, height: f32) -> Self {
        self.content_width = width;
        self.content_height = height;
        self.controller.update_content_height(height, self.style.height);
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
    
    pub fn scroll_to(&mut self, x: f32, _y: f32) {
        self.scroll_x = x.max(0.0).min(self.max_scroll_x());
        // self.scroll_y = y.max(0.0).min(self.max_scroll_y());
        // TODO: Controller doesn't have direct set_position exposed cleanly for jump without animation, 
        // but we can add it or just rely on drag logic. 
        // For now, let's ignore Y scroll_to or implement a set_position method in ScrollController if needed.
    }
    
    fn max_scroll_x(&self) -> f32 {
        (self.content_width - self.style.width).max(0.0)
    }
    
    fn max_scroll_y(&self) -> f32 {
        (self.content_height - self.style.height).max(0.0)
    }
    
    /// 更新惯性滚动
    pub fn update(&mut self, dt: f32) {
        self.controller.update(dt);
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
        
        let scroll_y = self.controller.get_position();
        
        // 渲染子组件（带偏移）
        for child in &self.children {
            if child.style().visible {
                // 计算子组件在滚动后的位置
                let child_bounds = child.style().bounds();
                let visible_y = child_bounds.y - scroll_y;
                let _visible_x = child_bounds.x - self.scroll_x;
                
                // 简单的可见性检查
                if visible_y + child_bounds.height >= bounds.y &&
                   visible_y <= bounds.bottom() {
                    // 临时修改子组件位置进行渲染
                    // 注意：这里简化处理，实际应该用变换矩阵
                    // 保存画布状态
                    canvas.save();
                    canvas.translate(-self.scroll_x, -scroll_y);
                    child.render(canvas);
                    canvas.restore();
                }
            }
        }
        
        // 重置裁剪
        canvas.reset_clip();
        
        // 绘制滚动条
        if self.show_scrollbar && self.content_height > self.style.height {
            let scrollbar_height = (self.style.height / self.content_height * self.style.height).max(20.0);
            let scrollbar_y = bounds.y + (scroll_y / self.max_scroll_y()) * (self.style.height - scrollbar_height);
            
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
                        self.last_touch = Some(t.position());
                        if self.scroll_enabled_y {
                            self.controller.begin_drag(t.y, touch.timestamp);
                        }
                        return true;
                    }
                }
            }
            Event::TouchMove(touch) => {
                if let (Some(t), Some(last)) = (touch.touches.first(), &self.last_touch) {
                    let _dy = last.y - t.y;
                    let dx = last.x - t.x;
                    
                    if self.scroll_enabled_y {
                        self.controller.update_drag(t.y, touch.timestamp);
                    }
                    if self.scroll_enabled_x {
                        self.scroll_x = (self.scroll_x + dx).max(0.0).min(self.max_scroll_x());
                    }
                    
                    self.last_touch = Some(t.position());
                    return true;
                }
            }
            Event::TouchEnd(_) | Event::TouchCancel(_) => {
                if self.scroll_enabled_y {
                    self.controller.end_drag();
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Touch, Event, TouchEvent};

    enum TouchPhase {
        Started,
        Moved,
        Ended,
        Cancelled,
    }

    fn create_touch_event(phase: TouchPhase, x: f32, y: f32, timestamp: u64) -> Event {
        let touch = Touch {
            id: 0,
            x,
            y,
            force: 0.0,
        };
        let touch_evt = TouchEvent {
            touches: vec![touch.clone()],
            changed_touches: vec![touch],
            timestamp,
        };
        match phase {
            TouchPhase::Started => Event::TouchStart(touch_evt),
            TouchPhase::Moved => Event::TouchMove(touch_evt),
            TouchPhase::Ended => Event::TouchEnd(touch_evt),
            TouchPhase::Cancelled => Event::TouchCancel(touch_evt),
        }
    }

    #[test]
    fn test_scroll_view_initial_state() {
        let sv = ScrollView::new();
        assert_eq!(sv.scroll_x, 0.0);
        assert_eq!(sv.controller.get_position(), 0.0);
    }

    #[test]
    fn test_scroll_view_drag() {
        let mut sv = ScrollView::new()
            .with_frame(0.0, 0.0, 100.0, 100.0)
            .with_content_size(100.0, 200.0)
            .with_scroll_y(true);

        // Touch start at t=0
        let start_evt = create_touch_event(TouchPhase::Started, 50.0, 50.0, 0);
        assert!(sv.on_event(&start_evt));
        assert!(sv.controller.is_dragging);

        // Touch move (drag up -> scroll down) at t=16ms
        // Drag from 50 to 40 (delta -10), scroll should increase by 10
        let move_evt = create_touch_event(TouchPhase::Moved, 50.0, 40.0, 16);
        assert!(sv.on_event(&move_evt));
        
        // Check scroll position
        // Initial 0, dragged 10 pixels content up (finger down), so scroll position should be 10.0
        assert!((sv.controller.get_position() - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_scroll_view_inertia() {
        let mut sv = ScrollView::new()
            .with_frame(0.0, 0.0, 100.0, 100.0)
            .with_content_size(100.0, 500.0)
            .with_scroll_y(true);

        // Simulate a fling
        // Start at 0ms, y=100
        let start_evt = create_touch_event(TouchPhase::Started, 50.0, 100.0, 0);
        sv.on_event(&start_evt);

        // Move at 16ms, y=50 (Moved 50px up quickly)
        let move_evt = create_touch_event(TouchPhase::Moved, 50.0, 50.0, 16);
        sv.on_event(&move_evt);

        // End at 32ms
        let end_evt = create_touch_event(TouchPhase::Ended, 50.0, 50.0, 32);
        sv.on_event(&end_evt);

        assert!(!sv.controller.is_dragging);
        assert!(sv.controller.is_animating());

        // Update with time delta
        let initial_pos = sv.controller.get_position();
        // Simulate 16ms frame
        sv.update(0.016); 
        let next_pos = sv.controller.get_position();

        // Should continue scrolling in the same direction (increasing scroll position)
        assert!(next_pos > initial_pos, "Position should increase due to inertia: {} -> {}", initial_pos, next_pos);
    }

    #[test]
    fn test_scroll_view_bounds_constraint() {
        let mut sv = ScrollView::new()
            .with_frame(0.0, 0.0, 100.0, 100.0)
            .with_content_size(100.0, 200.0) // Max scroll = 100
            .with_scroll_y(true);

        // Manually drive controller to test overshoot logic directly via events
        // Start drag at 0
        sv.controller.begin_drag(300.0, 0); 
        // Drag 200px up (finger moves from 300 to 100). 
        // Scroll should attempt to go to 200. Max is 100. Overshoot is 100.
        sv.controller.update_drag(100.0, 16); 
        
        // With rubber banding, it won't be exactly 200.
        let pos = sv.controller.get_position();
        assert!(pos > 100.0); // Should be allowed to overshoot
        assert!(pos < 200.0); // Should be resisted
        
        // End drag to start bounce back
        sv.controller.end_drag();
        
        // Simulate frames to let it settle (bounce back)
        // Simulate 1 second
        for _ in 0..60 {
            sv.update(0.016);
        }
        
        // Should settle near max scroll (100.0)
        assert!((sv.controller.get_position() - 100.0).abs() < 1.0, "Should bounce back to 100.0, got {}", sv.controller.get_position());
    }
    
    #[test]
    fn test_scroll_view_interruption() {
         let mut sv = ScrollView::new()
            .with_frame(0.0, 0.0, 100.0, 100.0)
            .with_content_size(100.0, 500.0)
            .with_scroll_y(true);
            
         // Start animation (fling)
         sv.on_event(&create_touch_event(TouchPhase::Started, 50.0, 100.0, 0));
         sv.on_event(&create_touch_event(TouchPhase::Moved, 50.0, 50.0, 16));
         sv.on_event(&create_touch_event(TouchPhase::Ended, 50.0, 50.0, 32));
         
         assert!(sv.controller.is_animating());
         
         // Interrupt with touch start
         sv.on_event(&create_touch_event(TouchPhase::Started, 50.0, 50.0, 100));
         
         assert!(!sv.controller.is_animating());
         assert!(sv.controller.is_dragging);
    }
}
