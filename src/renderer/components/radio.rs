//! radio 组件 - 单选框
//! 
//! 属性：
//! - value: radio 标识
//! - checked: 是否选中
//! - disabled: 是否禁用
//! - color: 选中时的颜色

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::{Canvas, Color, Paint, PaintStyle, Path};
use taffy::prelude::*;

pub struct RadioComponent;

impl RadioComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let checked = node.get_attr("checked").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let disabled = node.get_attr("disabled").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let radio_color = node.get_attr("color").and_then(|c| parse_color_str(c))
            .unwrap_or(Color::from_hex(0x09BB07));
        
        // 微信官方 radio 尺寸 - 增加点击区域
        let visual_size = 24.0 * sf;
        let touch_size = 40.0 * sf;
        ts.size = Size { width: length(touch_size), height: length(touch_size) };
        ts.justify_content = Some(JustifyContent::Center);
        ts.align_items = Some(AlignItems::Center);
        
        // 设置颜色
        let bg_color = if checked {
            if disabled { Color::from_hex(0xA9DCA8) } else { radio_color }
        } else {
            Color::WHITE
        };
        
        let border_color = if checked {
            if disabled { Color::from_hex(0xA9DCA8) } else { radio_color }
        } else {
            if disabled { Color::from_hex(0xE1E1E1) } else { Color::from_hex(0xD1D1D1) }
        };
        
        ns.background_color = Some(bg_color);
        ns.border_color = Some(border_color);
        ns.border_width = visual_size;
        ns.border_radius = visual_size / 2.0;
        ns.custom_data = if checked { 1.0 } else { 0.0 };
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "radio".into(),
            text: String::new(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, _sf: f32) {
        let style = &node.style;
        let checked = style.custom_data > 0.5;
        
        // 计算居中的实际绘制区域
        let visual_size = style.border_width;
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        let r = visual_size / 2.0;
        
        // 绘制背景圆 - 使用抗锯齿
        if let Some(bg) = style.background_color {
            let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill).with_anti_alias(true);
            canvas.draw_circle(cx, cy, r, &paint);
        }
        
        // 绘制边框 - 使用抗锯齿
        if !checked {
            if let Some(bc) = style.border_color {
                // 使用填充方式绘制圆环边框
                let border_width = 2.0;
                let paint = Paint::new().with_color(bc).with_style(PaintStyle::Fill).with_anti_alias(true);
                canvas.draw_circle(cx, cy, r, &paint);
                
                // 内圆（挖空）
                let inner_paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
                canvas.draw_circle(cx, cy, r - border_width, &inner_paint);
            }
        }
        
        // 绘制对勾 - 使用粗线条
        if checked {
            let thickness = r * 0.15;
            
            // 对勾的三个关键点
            let p1 = (cx - r * 0.35, cy);
            let p2 = (cx - r * 0.05, cy + r * 0.28);
            let p3 = (cx + r * 0.35, cy - r * 0.28);
            
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
