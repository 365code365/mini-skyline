//! 组件基础定义

use crate::{Canvas, Color, Point, Rect};
use crate::event::Event;
use std::sync::atomic::{AtomicU64, Ordering};

static COMPONENT_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// 组件 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(pub u64);

impl ComponentId {
    pub fn new() -> Self {
        Self(COMPONENT_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for ComponentId {
    fn default() -> Self {
        Self::new()
    }
}

/// 组件样式
#[derive(Debug, Clone)]
pub struct Style {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub background_color: Option<Color>,
    pub text_color: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub border_radius: f32,
    pub padding: [f32; 4],  // top, right, bottom, left
    pub margin: [f32; 4],
    pub opacity: f32,
    pub visible: bool,
    pub z_index: i32,
    
    // Flex 布局属性
    pub display_flex: bool,
    pub flex_direction: Option<String>,
    pub flex_wrap: Option<String>,
    pub justify_content: Option<String>,
    pub align_items: Option<String>,
    pub align_content: Option<String>,
    pub gap: Option<f32>,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Option<f32>,
    
    // 文本属性
    pub font_size: Option<f32>,
    pub font_weight: Option<String>,
    pub text_align: Option<String>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            background_color: None,
            text_color: None,
            border_color: None,
            border_width: 0.0,
            border_radius: 0.0,
            padding: [0.0; 4],
            margin: [0.0; 4],
            opacity: 1.0,
            visible: true,
            z_index: 0,
            
            display_flex: false,
            flex_direction: None,
            flex_wrap: None,
            justify_content: None,
            align_items: None,
            align_content: None,
            gap: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: None,
            
            font_size: None,
            font_weight: None,
            text_align: None,
        }
    }
}

impl Style {
    pub fn bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }
    
    pub fn content_bounds(&self) -> Rect {
        Rect::new(
            self.x + self.padding[3],
            self.y + self.padding[0],
            self.width - self.padding[1] - self.padding[3],
            self.height - self.padding[0] - self.padding[2],
        )
    }
}

/// 组件 trait
pub trait Component: Send + Sync {
    fn id(&self) -> ComponentId;
    fn style(&self) -> &Style;
    fn style_mut(&mut self) -> &mut Style;
    
    /// 渲染组件
    fn render(&self, canvas: &mut Canvas);
    
    /// 处理事件，返回是否消费
    fn on_event(&mut self, event: &Event) -> bool {
        let _ = event;
        false
    }
    
    /// 点击测试
    fn hit_test(&self, point: &Point) -> bool {
        self.style().visible && self.style().bounds().contains(point)
    }
    
    /// 获取子组件
    fn children(&self) -> &[Box<dyn Component>] {
        &[]
    }
    
    /// 获取可变子组件
    fn children_mut(&mut self) -> &mut Vec<Box<dyn Component>> {
        static mut EMPTY: Vec<Box<dyn Component>> = Vec::new();
        unsafe { &mut EMPTY }
    }
    
    /// 添加子组件
    fn add_child(&mut self, _child: Box<dyn Component>) {}
    
    /// 组件类型名
    fn type_name(&self) -> &'static str {
        "Component"
    }
}

/// 组件树
pub struct ComponentTree {
    root: Option<Box<dyn Component>>,
}

unsafe impl Send for ComponentTree {}
unsafe impl Sync for ComponentTree {}

impl ComponentTree {
    pub fn new() -> Self {
        Self {
            root: None,
        }
    }
    
    pub fn set_root(&mut self, root: Box<dyn Component>) {
        self.root = Some(root);
    }
    
    pub fn root(&self) -> Option<&dyn Component> {
        self.root.as_ref().map(|r| r.as_ref())
    }
    
    pub fn root_mut(&mut self) -> Option<&mut Box<dyn Component>> {
        self.root.as_mut()
    }
    
    /// 渲染整个组件树
    pub fn render(&self, canvas: &mut Canvas) {
        if let Some(root) = &self.root {
            self.render_component(root.as_ref(), canvas);
        }
    }
    
    fn render_component(&self, component: &dyn Component, canvas: &mut Canvas) {
        if !component.style().visible {
            return;
        }
        
        component.render(canvas);
        
        for child in component.children() {
            self.render_component(child.as_ref(), canvas);
        }
    }
    
    /// 分发事件
    pub fn dispatch_event(&mut self, event: &Event) -> bool {
        if let Some(root) = &mut self.root {
            return Self::dispatch_to_component(root.as_mut(), event);
        }
        false
    }
    
    fn dispatch_to_component(component: &mut dyn Component, event: &Event) -> bool {
        // 先分发给子组件（从后往前，z-index 高的先处理）
        for child in component.children_mut().iter_mut().rev() {
            if Self::dispatch_to_component(child.as_mut(), event) {
                return true;
            }
        }
        
        // 再处理自己
        component.on_event(event)
    }
    
    /// 点击测试，返回被点击的组件
    pub fn hit_test(&self, point: &Point) -> Option<ComponentId> {
        if let Some(root) = &self.root {
            return self.hit_test_component(root.as_ref(), point);
        }
        None
    }
    
    fn hit_test_component(&self, component: &dyn Component, point: &Point) -> Option<ComponentId> {
        if !component.hit_test(point) {
            return None;
        }
        
        // 检查子组件
        for child in component.children().iter().rev() {
            if let Some(id) = self.hit_test_component(child.as_ref(), point) {
                return Some(id);
            }
        }
        
        Some(component.id())
    }
}

impl Default for ComponentTree {
    fn default() -> Self {
        Self::new()
    }
}
