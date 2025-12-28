//! text 组件 - 文本显示

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle};
use taffy::prelude::*;

pub struct TextComponent;

impl TextComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        let text_content = get_text_content(node);
        if text_content.is_empty() { return None; }
        
        let sf = ctx.scale_factor;
        // 简单估算文本宽度
        let char_width = ns.font_size * 0.6 * sf;
        let tw = text_content.chars().count() as f32 * char_width;
        
        ts.size = Size { 
            width: length(tw), 
            height: length((ns.font_size + 4.0) * sf) 
        };
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "text".into(),
            text: text_content,
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        _w: f32, 
        _h: f32, 
        sf: f32
    ) {
        let color = node.style.text_color.unwrap_or(Color::BLACK);
        let size = node.style.font_size * sf;
        
        if let Some(tr) = text_renderer {
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            tr.draw_text(canvas, &node.text, x, y + size, size, &paint);
        }
    }
}
