//! 事件系统 - 处理用户交互

use crate::Point;

/// 事件类型
#[derive(Debug, Clone)]
pub enum Event {
    // 触摸/鼠标事件
    TouchStart(TouchEvent),
    TouchMove(TouchEvent),
    TouchEnd(TouchEvent),
    TouchCancel(TouchEvent),
    
    // 点击事件
    Tap(TapEvent),
    LongPress(TapEvent),
    
    // 键盘事件
    KeyDown(KeyEvent),
    KeyUp(KeyEvent),
    
    // 生命周期
    AppShow,
    AppHide,
    PageLoad,
    PageUnload,
    PageShow,
    PageHide,
}

/// 触摸事件
#[derive(Debug, Clone)]
pub struct TouchEvent {
    pub touches: Vec<Touch>,
    pub changed_touches: Vec<Touch>,
    pub timestamp: u64,
}

/// 单个触摸点
#[derive(Debug, Clone)]
pub struct Touch {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub force: f32,
}

impl Touch {
    pub fn new(id: u32, x: f32, y: f32) -> Self {
        Self { id, x, y, force: 1.0 }
    }
    
    pub fn position(&self) -> Point {
        Point::new(self.x, self.y)
    }
}

/// 点击事件
#[derive(Debug, Clone)]
pub struct TapEvent {
    pub x: f32,
    pub y: f32,
    pub timestamp: u64,
}

/// 键盘事件
#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub key: String,
    pub code: String,
    pub alt: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub meta: bool,
}

/// 事件目标
pub trait EventTarget {
    fn on_event(&mut self, event: &Event) -> bool;
    fn hit_test(&self, point: &Point) -> bool;
}

/// 事件分发器
pub struct EventDispatcher {
    listeners: Vec<(String, Box<dyn Fn(&Event) + Send + Sync>)>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self { listeners: Vec::new() }
    }
    
    pub fn add_listener<F>(&mut self, event_type: &str, callback: F)
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        self.listeners.push((event_type.to_string(), Box::new(callback)));
    }
    
    pub fn dispatch(&self, event_type: &str, event: &Event) {
        for (t, callback) in &self.listeners {
            if t == event_type {
                callback(event);
            }
        }
    }
    
    pub fn remove_listeners(&mut self, event_type: &str) {
        self.listeners.retain(|(t, _)| t != event_type);
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
