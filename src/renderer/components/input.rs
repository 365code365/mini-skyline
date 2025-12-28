//! input 组件 - 输入框
//! 
//! 属性：
//! - value: 输入框的值
//! - type: text / number / idcard / digit / safe-password / nickname
//! - password: 是否是密码类型
//! - placeholder: 占位符
//! - placeholder-style: 占位符样式
//! - disabled: 是否禁用
//! - maxlength: 最大输入长度
//! - focus: 是否获取焦点

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use taffy::prelude::*;

pub struct InputComponent;

impl InputComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let value = node.get_attr("value").unwrap_or("");
        let placeholder = node.get_attr("placeholder").unwrap_or("");
        let password = node.get_attr("password").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let disabled = node.get_attr("disabled").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let is_textarea = node.tag_name == "textarea";
        
        // 设置尺寸
        if ts.size.width == Dimension::Auto {
            ts.size.width = percent(1.0);
        }
        if ts.size.height == Dimension::Auto {
            ts.size.height = length(if is_textarea { 80.0 * sf } else { 42.0 * sf });
        }
        
        ts.padding = Rect { 
            top: length(8.0 * sf), 
            right: length(12.0 * sf), 
            bottom: length(8.0 * sf), 
            left: length(12.0 * sf) 
        };
        
        // 默认样式
        if ns.background_color.is_none() {
            ns.background_color = Some(Color::WHITE);
        }
        if ns.border_color.is_none() {
            ns.border_color = Some(Color::from_hex(0xD9D9D9));
        }
        ns.border_width = 1.0 * sf;
        ns.border_radius = 4.0 * sf;
        ns.font_size = 16.0;
        
        // 显示文本
        let display_text = if value.is_empty() {
            placeholder.to_string()
        } else if password {
            "•".repeat(value.len())
        } else {
            value.to_string()
        };
        
        // 文本颜色
        ns.text_color = Some(if value.is_empty() {
            Color::from_hex(0xBFBFBF) // placeholder 颜色
        } else if disabled {
            Color::from_hex(0xBFBFBF)
        } else {
            Color::BLACK
        });
        
        // 禁用状态背景
        if disabled {
            ns.background_color = Some(Color::from_hex(0xF5F5F5));
        }
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: node.tag_name.clone(),
            text: display_text,
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
        Self::draw_with_cursor(node, canvas, text_renderer, x, y, w, h, sf, false, 0);
    }
    
    pub fn draw_with_cursor(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        sf: f32,
        focused: bool,
        cursor_pos: usize,
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
        
        // 绘制边框 - 聚焦时高亮
        let border_color = if focused {
            Color::from_hex(0x07C160) // 微信绿色
        } else {
            style.border_color.unwrap_or(Color::from_hex(0xD9D9D9))
        };
        
        if style.border_width > 0.0 {
            let paint = Paint::new().with_color(border_color).with_style(PaintStyle::Stroke);
            if style.border_radius > 0.0 {
                let mut path = Path::new();
                path.add_round_rect(x, y, w, h, style.border_radius);
                canvas.draw_path(&path, &paint);
            } else {
                canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
            }
        }
        
        // 绘制文本
        let padding = 12.0 * sf;
        let font_size = style.font_size * sf;
        let text_y = y + (h + font_size) / 2.0 - 2.0 * sf;
        
        if let Some(tr) = text_renderer {
            let color = style.text_color.unwrap_or(Color::BLACK);
            let text_x = x + padding;
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            
            if !node.text.is_empty() {
                tr.draw_text(canvas, &node.text, text_x, text_y, font_size, &paint);
            }
            
            // 绘制光标
            if focused {
                let cursor_text: String = node.text.chars().take(cursor_pos).collect();
                let cursor_x = text_x + tr.measure_text(&cursor_text, font_size);
                let cursor_y1 = y + (h - font_size) / 2.0;
                let cursor_y2 = cursor_y1 + font_size;
                
                let cursor_paint = Paint::new()
                    .with_color(Color::from_hex(0x07C160))
                    .with_style(PaintStyle::Stroke);
                
                let mut cursor_path = Path::new();
                cursor_path.move_to(cursor_x, cursor_y1);
                cursor_path.line_to(cursor_x, cursor_y2);
                canvas.draw_path(&cursor_path, &cursor_paint);
            }
        }
    }
}
