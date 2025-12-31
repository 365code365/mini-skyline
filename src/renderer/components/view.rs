//! view 组件 - 基础视图容器

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::Canvas;

pub struct ViewComponent;

impl ViewComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (ts, ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: node.tag_name.clone(),
            text: String::new(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, _sf: f32) {
        draw_background(canvas, &node.style, x, y, w, h);
    }
    
    /// 绘制带按下状态的 view（用于有点击事件的 view）
    /// view 不需要特殊的按下状态效果，直接使用普通绘制
    pub fn draw_with_state(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, _sf: f32, _pressed: bool) {
        draw_background(canvas, &node.style, x, y, w, h);
    }
}
