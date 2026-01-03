//! Swiper 轮播图组件

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::{Canvas, Color, Paint, PaintStyle, Rect as GeoRect};
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

/// Swiper 状态管理器
pub struct SwiperStateManager {
    states: HashMap<String, SwiperState>,
}

#[derive(Clone)]
pub struct SwiperState {
    pub current: usize,
    pub total: usize,
    pub last_update: std::time::Instant,
    pub autoplay_interval: u64,
    pub autoplay: bool,
}

impl SwiperStateManager {
    pub fn new() -> Self {
        Self { states: HashMap::new() }
    }
    
    pub fn get_or_create(&mut self, id: &str, total: usize, autoplay: bool, interval: u64) -> &mut SwiperState {
        self.states.entry(id.to_string()).or_insert_with(|| SwiperState {
            current: 0,
            total,
            last_update: std::time::Instant::now(),
            autoplay_interval: interval,
            autoplay,
        })
    }
    
    pub fn get(&self, id: &str) -> Option<&SwiperState> {
        self.states.get(id)
    }
    
    pub fn set_current(&mut self, id: &str, current: usize) {
        if let Some(state) = self.states.get_mut(id) {
            state.current = current;
            state.last_update = std::time::Instant::now();
        }
    }
    
    pub fn next(&mut self, id: &str) {
        if let Some(state) = self.states.get_mut(id) {
            state.current = (state.current + 1) % state.total;
            state.last_update = std::time::Instant::now();
        }
    }
    
    pub fn prev(&mut self, id: &str) {
        if let Some(state) = self.states.get_mut(id) {
            if state.current == 0 {
                state.current = state.total.saturating_sub(1);
            } else {
                state.current -= 1;
            }
            state.last_update = std::time::Instant::now();
        }
    }
    
    /// 检查并执行自动播放
    pub fn check_autoplay(&mut self, id: &str) -> bool {
        if let Some(state) = self.states.get_mut(id) {
            if state.autoplay && state.total > 1 {
                let elapsed = state.last_update.elapsed().as_millis() as u64;
                if elapsed >= state.autoplay_interval {
                    state.current = (state.current + 1) % state.total;
                    state.last_update = std::time::Instant::now();
                    return true;
                }
            }
        }
        false
    }
}

/// 全局 Swiper 状态管理器
pub static SWIPER_MANAGER: Lazy<Mutex<SwiperStateManager>> = Lazy::new(|| {
    Mutex::new(SwiperStateManager::new())
});

/// Swiper 组件
pub struct SwiperComponent;

impl SwiperComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        // 默认背景色
        if ns.background_color.is_none() {
            ns.background_color = Some(Color::WHITE);
        }
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "swiper".into(),
            text: String::new(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        // 绘制背景
        draw_background(canvas, &node.style, x, y, w, h);
        
        // 获取属性
        let swiper_id = node.attrs.get("id").cloned()
            .unwrap_or_else(|| format!("swiper_{:.0}_{:.0}", x, y));
        let autoplay = node.attrs.get("autoplay")
            .map(|s| s == "true" || s == "{{true}}")
            .unwrap_or(false);
        let interval = node.attrs.get("interval")
            .and_then(|s| s.parse().ok())
            .unwrap_or(5000u64);
        let indicator_dots = node.attrs.get("indicator-dots")
            .map(|s| s == "true" || s == "{{true}}")
            .unwrap_or(true);
        let indicator_color = node.attrs.get("indicator-color")
            .and_then(|s| parse_color_str(s))
            .unwrap_or(Color::new(0, 0, 0, 76)); // rgba(0,0,0,0.3)
        let indicator_active_color = node.attrs.get("indicator-active-color")
            .and_then(|s| parse_color_str(s))
            .unwrap_or(Color::from_hex(0x000000));
        let circular = node.attrs.get("circular")
            .map(|s| s == "true" || s == "{{true}}")
            .unwrap_or(false);
        
        let total = node.children.len();
        if total == 0 {
            return;
        }
        
        // 获取或创建状态
        let current = {
            if let Ok(mut manager) = SWIPER_MANAGER.lock() {
                let state = manager.get_or_create(&swiper_id, total, autoplay, interval);
                // 检查自动播放
                manager.check_autoplay(&swiper_id);
                manager.get(&swiper_id).map(|s| s.current).unwrap_or(0)
            } else {
                0
            }
        };
        
        // 绘制当前 swiper-item
        if current < node.children.len() {
            let child = &node.children[current];
            // swiper-item 填满整个 swiper
            draw_swiper_item(canvas, child, x, y, w, h, sf);
        }
        
        // 绘制指示点
        if indicator_dots && total > 1 {
            let dot_size = 8.0 * sf;
            let dot_gap = 8.0 * sf;
            let total_width = total as f32 * dot_size + (total - 1) as f32 * dot_gap;
            let start_x = x + (w - total_width) / 2.0;
            let dot_y = y + h - 20.0 * sf;
            
            for i in 0..total {
                let dot_x = start_x + i as f32 * (dot_size + dot_gap);
                let color = if i == current { indicator_active_color } else { indicator_color };
                let paint = Paint::new()
                    .with_color(color)
                    .with_style(PaintStyle::Fill)
                    .with_anti_alias(true);
                canvas.draw_circle(dot_x + dot_size / 2.0, dot_y + dot_size / 2.0, dot_size / 2.0, &paint);
            }
        }
        
        // 绘制边框
        if node.style.border_width > 0.0 {
            let border_color = node.style.border_color.unwrap_or(Color::from_hex(0xE5E5E5));
            let paint = Paint::new()
                .with_color(border_color)
                .with_style(PaintStyle::Stroke)
                .with_stroke_width(node.style.border_width);
            let rect = GeoRect::new(x, y, w, h);
            canvas.draw_rect(&rect, &paint);
        }
    }
    
    /// 切换到下一页
    pub fn next(swiper_id: &str) {
        if let Ok(mut manager) = SWIPER_MANAGER.lock() {
            manager.next(swiper_id);
        }
    }
    
    /// 切换到上一页
    pub fn prev(swiper_id: &str) {
        if let Ok(mut manager) = SWIPER_MANAGER.lock() {
            manager.prev(swiper_id);
        }
    }
    
    /// 切换到指定页
    pub fn set_current(swiper_id: &str, index: usize) {
        if let Ok(mut manager) = SWIPER_MANAGER.lock() {
            manager.set_current(swiper_id, index);
        }
    }
}

/// 绘制 swiper-item
fn draw_swiper_item(canvas: &mut Canvas, node: &RenderNode, x: f32, y: f32, w: f32, h: f32, sf: f32) {
    // 绘制背景
    if let Some(bg) = node.style.background_color {
        let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
        canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
    }
    
    // swiper-item 的子元素需要递归绘制
    // 这里简化处理，只绘制背景和第一层子元素的文本
    for child in &node.children {
        if child.tag == "image" {
            // 绘制图片占位
            if let Some(src) = child.attrs.get("src") {
                // 图片绘制由 ImageComponent 处理
            }
        } else if child.tag == "text" || child.tag == "#text" {
            // 绘制文本
            let text_color = child.style.text_color.unwrap_or(Color::BLACK);
            // 简化：居中显示文本
        }
    }
}

/// SwiperItem 组件
pub struct SwiperItemComponent;

impl SwiperItemComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (ts, ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "swiper-item".into(),
            text: String::new(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
}
