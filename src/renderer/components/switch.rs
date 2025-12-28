//! switch 组件 - 开关选择器
//! 
//! 属性：
//! - checked: 是否选中
//! - disabled: 是否禁用
//! - type: switch(默认) / checkbox
//! - color: 选中时的颜色

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::{Canvas, Color, Paint, PaintStyle, Path};
use taffy::prelude::*;

pub struct SwitchComponent;

impl SwitchComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let checked = node.get_attr("checked").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let disabled = node.get_attr("disabled").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let switch_type = node.get_attr("type").unwrap_or("switch");
        let switch_color = node.get_attr("color").and_then(|c| parse_color_str(c))
            .unwrap_or(Color::from_hex(0x04BE02));
        
        // 微信官方 switch 尺寸
        let (width, height) = if switch_type == "checkbox" {
            (24.0, 24.0)
        } else {
            (51.0, 31.0)
        };
        
        ts.size = Size { width: length(width * sf), height: length(height * sf) };
        
        // 设置颜色
        let bg_color = if checked {
            if disabled { Color::from_hex(0xA9DCA8) } else { switch_color }
        } else {
            if disabled { Color::from_hex(0xF0F0F0) } else { Color::from_hex(0xDFDFDF) }
        };
        
        ns.background_color = Some(bg_color);
        ns.border_radius = if switch_type == "checkbox" { 4.0 * sf } else { height * sf / 2.0 };
        ns.custom_data = if checked { 1.0 } else { 0.0 };
        ns.border_width = width;
        ns.font_size = height;
        
        let text = switch_type.to_string();
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "switch".into(),
            text,
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        let style = &node.style;
        let checked = style.custom_data > 0.5;
        let switch_type = node.text.as_str();
        
        if switch_type == "checkbox" {
            Self::draw_checkbox(canvas, style, checked, x, y, w, h, sf);
        } else {
            Self::draw_switch(canvas, style, checked, x, y, w, h, sf);
        }
    }
    
    fn draw_switch(canvas: &mut Canvas, style: &NodeStyle, checked: bool, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        // 绘制背景轨道 - 使用抗锯齿
        if let Some(bg) = style.background_color {
            let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill).with_anti_alias(true);
            let mut path = Path::new();
            path.add_round_rect(x, y, w, h, h / 2.0);
            canvas.draw_path(&path, &paint);
        }
        
        // 绘制圆形滑块
        let knob_radius = (h - 4.0 * sf) / 2.0;
        let knob_y = y + h / 2.0;
        let knob_x = if checked {
            x + w - knob_radius - 2.0 * sf
        } else {
            x + knob_radius + 2.0 * sf
        };
        
        // 滑块阴影 - 使用抗锯齿
        let shadow_paint = Paint::new()
            .with_color(Color::new(0, 0, 0, 30))
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        canvas.draw_circle(knob_x, knob_y + 1.0 * sf, knob_radius, &shadow_paint);
        
        // 滑块 - 使用抗锯齿
        let knob_paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
        canvas.draw_circle(knob_x, knob_y, knob_radius, &knob_paint);
    }
    
    fn draw_checkbox(canvas: &mut Canvas, style: &NodeStyle, checked: bool, x: f32, y: f32, w: f32, h: f32, _sf: f32) {
        // 绘制背景 - 使用抗锯齿
        if let Some(bg) = style.background_color {
            let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill).with_anti_alias(true);
            let mut path = Path::new();
            path.add_round_rect(x, y, w, h, style.border_radius);
            canvas.draw_path(&path, &paint);
        }
        
        // 绘制边框
        if !checked {
            let border_width = 2.0;
            let border_paint = Paint::new()
                .with_color(Color::from_hex(0xD1D1D1))
                .with_style(PaintStyle::Fill)
                .with_anti_alias(true);
            let mut border = Path::new();
            border.add_round_rect(x, y, w, h, style.border_radius);
            canvas.draw_path(&border, &border_paint);
            
            // 内部白色
            let inner_paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
            let mut inner = Path::new();
            inner.add_round_rect(
                x + border_width, 
                y + border_width, 
                w - border_width * 2.0, 
                h - border_width * 2.0, 
                (style.border_radius - border_width).max(0.0)
            );
            canvas.draw_path(&inner, &inner_paint);
        }
        
        // 绘制对勾 - 使用粗线条
        if checked {
            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let thickness = w * 0.12;
            
            let p1 = (cx - w * 0.25, cy);
            let p2 = (cx - w * 0.05, cy + h * 0.2);
            let p3 = (cx + w * 0.25, cy - h * 0.2);
            
            let paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
            let half = thickness / 2.0;
            
            // 第一段
            let angle1 = ((p2.1 - p1.1) / (p2.0 - p1.0)).atan();
            let dx1 = half * angle1.sin();
            let dy1 = half * angle1.cos();
            
            let mut seg1 = Path::new();
            seg1.move_to(p1.0 - dx1, p1.1 + dy1);
            seg1.line_to(p1.0 + dx1, p1.1 - dy1);
            seg1.line_to(p2.0 + dx1, p2.1 - dy1);
            seg1.line_to(p2.0 - dx1, p2.1 + dy1);
            seg1.close();
            canvas.draw_path(&seg1, &paint);
            
            // 第二段
            let angle2 = ((p3.1 - p2.1) / (p3.0 - p2.0)).atan();
            let dx2 = half * angle2.sin();
            let dy2 = half * angle2.cos();
            
            let mut seg2 = Path::new();
            seg2.move_to(p2.0 - dx2, p2.1 + dy2);
            seg2.line_to(p2.0 + dx2, p2.1 - dy2);
            seg2.line_to(p3.0 + dx2, p3.1 - dy2);
            seg2.line_to(p3.0 - dx2, p3.1 + dy2);
            seg2.close();
            canvas.draw_path(&seg2, &paint);
            
            // 拐点圆形
            canvas.draw_circle(p2.0, p2.1, half, &paint);
        }
    }
}
