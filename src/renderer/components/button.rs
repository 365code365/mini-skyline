//! button 组件 - 按钮
//! 
//! 微信官方按钮样式：
//! - type: default(灰色) / primary(绿色) / warn(红色)
//! - size: default / mini
//! - plain: 镂空按钮
//! - disabled: 禁用状态

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use taffy::prelude::*;

pub struct ButtonComponent;

impl ButtonComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let text = get_text_content(node);
        let btn_type = node.get_attr("type").unwrap_or("default");
        let btn_size = node.get_attr("size").unwrap_or("default");
        let plain = node.get_attr("plain").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let disabled = node.get_attr("disabled").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        
        // 根据类型设置颜色（微信官方配色）
        let (bg, fg, border) = match btn_type {
            "primary" => {
                if plain {
                    (Color::WHITE, Color::from_hex(0x07C160), Some(Color::from_hex(0x07C160)))
                } else {
                    (Color::from_hex(0x07C160), Color::WHITE, None)
                }
            }
            "warn" => {
                if plain {
                    (Color::WHITE, Color::from_hex(0xE64340), Some(Color::from_hex(0xE64340)))
                } else {
                    (Color::from_hex(0xE64340), Color::WHITE, None)
                }
            }
            _ => { // default
                if plain {
                    (Color::WHITE, Color::from_hex(0x353535), Some(Color::from_hex(0x353535)))
                } else {
                    (Color::from_hex(0xF8F8F8), Color::BLACK, Some(Color::from_hex(0xD9D9D9)))
                }
            }
        };
        
        // 禁用状态
        let (bg, fg) = if disabled {
            (Color::from_hex(0xF7F7F7), Color::from_hex(0xB2B2B2))
        } else {
            (bg, fg)
        };
        
        ns.background_color = Some(bg);
        ns.text_color = Some(fg);
        ns.border_color = border;
        ns.border_width = if border.is_some() { 1.0 * sf } else { 0.0 };
        
        // 根据 size 设置样式
        let (font_size, padding_v, padding_h, radius) = match btn_size {
            "mini" => (13.0, 4.0, 12.0, 3.0),
            _ => (18.0, 12.0, 24.0, 5.0),
        };
        
        ns.font_size = font_size;
        ns.border_radius = radius * sf;
        
        // 估算文本宽度
        let char_width = font_size * 0.6 * sf;
        let tw = text.chars().count() as f32 * char_width;
        
        ts.size = Size { 
            width: if btn_size == "mini" { length(tw + padding_h * 2.0 * sf) } else { percent(1.0) },
            height: length((font_size + padding_v * 2.0) * sf) 
        };
        ts.padding = Rect { 
            top: length(padding_v * sf), 
            right: length(padding_h * sf), 
            bottom: length(padding_v * sf), 
            left: length(padding_h * sf) 
        };
        ts.margin = Rect { 
            top: length(5.0 * sf), 
            right: length(0.0), 
            bottom: length(5.0 * sf), 
            left: length(0.0) 
        };
        ts.align_items = Some(AlignItems::Center);
        ts.justify_content = Some(JustifyContent::Center);
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "button".into(),
            text,
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
        w: f32, 
        h: f32, 
        sf: f32
    ) {
        let style = &node.style;
        
        // 绘制背景
        if let Some(bg) = style.background_color {
            let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
            if style.border_radius > 0.0 {
                let mut path = Path::new();
                path.add_round_rect(x, y, w, h, style.border_radius);
                canvas.draw_path(&path, &paint);
            } else {
                canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
            }
        }
        
        // 绘制边框
        if style.border_width > 0.0 {
            if let Some(bc) = style.border_color {
                let paint = Paint::new().with_color(bc).with_style(PaintStyle::Stroke);
                if style.border_radius > 0.0 {
                    let mut path = Path::new();
                    path.add_round_rect(x, y, w, h, style.border_radius);
                    canvas.draw_path(&path, &paint);
                } else {
                    canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
                }
            }
        }
        
        // 绘制文本（居中）
        if let Some(tr) = text_renderer {
            let color = style.text_color.unwrap_or(Color::WHITE);
            let size = style.font_size * sf;
            let tw = tr.measure_text(&node.text, size);
            let tx = x + (w - tw) / 2.0;
            let ty = y + (h - size) / 2.0 + size;
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            tr.draw_text(canvas, &node.text, tx, ty, size, &paint);
        }
    }
}
