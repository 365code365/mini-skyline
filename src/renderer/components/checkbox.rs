//! checkbox 组件 - 复选框
//! 
//! 属性：
//! - value: checkbox 标识
//! - checked: 是否选中
//! - disabled: 是否禁用
//! - color: 选中时的颜色

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::{Canvas, Color, Paint, PaintStyle, Path};
use taffy::prelude::*;

pub struct CheckboxComponent;

impl CheckboxComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let checked = node.get_attr("checked").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let disabled = node.get_attr("disabled").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let checkbox_color = node.get_attr("color").and_then(|c| parse_color_str(c))
            .unwrap_or(Color::from_hex(0x09BB07));
        
        // 微信官方 checkbox 尺寸 - 增加点击区域
        let visual_size = 24.0 * sf;
        let touch_size = 40.0 * sf; // 更大的点击区域
        ts.size = Size { width: length(touch_size), height: length(touch_size) };
        ts.justify_content = Some(JustifyContent::Center);
        ts.align_items = Some(AlignItems::Center);
        
        // 设置颜色
        let bg_color = if checked {
            if disabled { Color::from_hex(0xA9DCA8) } else { checkbox_color }
        } else {
            Color::WHITE
        };
        
        let border_color = if checked {
            if disabled { Color::from_hex(0xA9DCA8) } else { checkbox_color }
        } else {
            if disabled { Color::from_hex(0xE1E1E1) } else { Color::from_hex(0xD1D1D1) }
        };
        
        ns.background_color = Some(bg_color);
        ns.border_color = Some(border_color);
        ns.border_width = visual_size; // 存储 visual_size
        ns.border_radius = 4.0 * sf;
        ns.custom_data = if checked { 1.0 } else { 0.0 };
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "checkbox".into(),
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
        let visual_size = style.border_width; // 从 border_width 获取 visual_size
        let draw_x = x + (w - visual_size) / 2.0;
        let draw_y = y + (h - visual_size) / 2.0;
        let draw_w = visual_size;
        let draw_h = visual_size;
        let radius = style.border_radius;
        
        // 绘制背景
        if let Some(bg) = style.background_color {
            let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
            let mut path = Path::new();
            path.add_round_rect(draw_x, draw_y, draw_w, draw_h, radius);
            canvas.draw_path(&path, &paint);
        }
        
        // 绘制边框
        if let Some(bc) = style.border_color {
            let paint = Paint::new().with_color(bc).with_style(PaintStyle::Stroke);
            let mut path = Path::new();
            path.add_round_rect(draw_x, draw_y, draw_w, draw_h, radius);
            canvas.draw_path(&path, &paint);
        }
        
        // 绘制对勾
        if checked {
            let check_paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Stroke);
            let cx = draw_x + draw_w / 2.0;
            let cy = draw_y + draw_h / 2.0;
            let mut check = Path::new();
            check.move_to(cx - draw_w * 0.28, cy);
            check.line_to(cx - draw_w * 0.05, cy + draw_h * 0.22);
            check.line_to(cx + draw_w * 0.28, cy - draw_h * 0.22);
            canvas.draw_path(&check, &check_paint);
        }
    }
}
