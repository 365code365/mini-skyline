//! switch 组件 - 开关选择器
//! 
//! 支持完整的 CSS 样式，同时保留微信默认样式作为 fallback
//! 属性：
//! - checked: 是否选中
//! - disabled: 是否禁用
//! - type: switch(默认) / checkbox
//! - color: 选中时的颜色
//! 
//! CSS 支持：
//! - width/height: 自定义尺寸
//! - background-color: 自定义轨道背景色
//! - border-radius: 自定义圆角
//! - opacity: 透明度
//! - box-shadow: 阴影

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
        
        // 检查 CSS 是否定义了尺寸和颜色
        let has_custom_size = !matches!(ts.size.width, Dimension::Auto) || 
                              !matches!(ts.size.height, Dimension::Auto);
        let has_custom_bg = ns.background_color.is_some();
        let has_custom_radius = ns.border_radius > 0.0;
        
        // 微信官方 switch 尺寸 - 只在没有自定义尺寸时使用
        let (default_width, default_height) = if switch_type == "checkbox" {
            (24.0, 24.0)
        } else {
            (51.0, 31.0)
        };
        
        if !has_custom_size {
            ts.size = Size { width: length(default_width * sf), height: length(default_height * sf) };
        }
        
        // 只在 CSS 没有定义时使用微信默认颜色
        if !has_custom_bg {
            let bg_color = if checked {
                if disabled { Color::from_hex(0xA9DCA8) } else { switch_color }
            } else {
                if disabled { Color::from_hex(0xF0F0F0) } else { Color::from_hex(0xDFDFDF) }
            };
            ns.background_color = Some(bg_color);
        }
        
        // 只在 CSS 没有定义时使用默认圆角
        if !has_custom_radius {
            ns.border_radius = if switch_type == "checkbox" { 
                4.0 * sf 
            } else { 
                default_height * sf / 2.0 
            };
        }
        
        ns.custom_data = if checked { 1.0 } else { 0.0 };
        ns.border_width = default_width;
        ns.font_size = default_height;
        
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
        
        // 绘制盒子阴影
        if let Some(shadow) = &style.box_shadow {
            draw_box_shadow(canvas, shadow, x, y, w, h, style.border_radius);
        }
        
        if switch_type == "checkbox" {
            Self::draw_checkbox(canvas, style, checked, x, y, w, h, sf);
        } else {
            Self::draw_switch(canvas, style, checked, x, y, w, h, sf);
        }
    }
    
    fn draw_switch(canvas: &mut Canvas, style: &NodeStyle, checked: bool, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        // 应用透明度
        let apply_opacity = |color: Color| -> Color {
            if style.opacity < 1.0 {
                Color::new(color.r, color.g, color.b, (color.a as f32 * style.opacity) as u8)
            } else {
                color
            }
        };
        
        // 获取圆角值（支持四个角独立设置）
        let radius_tl = style.border_radius_tl.unwrap_or(style.border_radius);
        let radius_tr = style.border_radius_tr.unwrap_or(style.border_radius);
        let radius_br = style.border_radius_br.unwrap_or(style.border_radius);
        let radius_bl = style.border_radius_bl.unwrap_or(style.border_radius);
        let uniform_radius = radius_tl == radius_tr && radius_tr == radius_br && radius_br == radius_bl;
        let radius = if uniform_radius { radius_tl } else { h / 2.0 };
        
        // 绘制背景轨道 - 使用抗锯齿
        if let Some(bg) = style.background_color {
            let paint = Paint::new().with_color(apply_opacity(bg)).with_style(PaintStyle::Fill).with_anti_alias(true);
            let mut path = Path::new();
            if uniform_radius {
                path.add_round_rect(x, y, w, h, radius);
            } else {
                path.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
            }
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
        // 应用透明度
        let apply_opacity = |color: Color| -> Color {
            if style.opacity < 1.0 {
                Color::new(color.r, color.g, color.b, (color.a as f32 * style.opacity) as u8)
            } else {
                color
            }
        };
        
        // 获取圆角值
        let radius_tl = style.border_radius_tl.unwrap_or(style.border_radius);
        let radius_tr = style.border_radius_tr.unwrap_or(style.border_radius);
        let radius_br = style.border_radius_br.unwrap_or(style.border_radius);
        let radius_bl = style.border_radius_bl.unwrap_or(style.border_radius);
        let uniform_radius = radius_tl == radius_tr && radius_tr == radius_br && radius_br == radius_bl;
        let radius = radius_tl;
        
        // 绘制背景 - 使用抗锯齿
        if let Some(bg) = style.background_color {
            let paint = Paint::new().with_color(apply_opacity(bg)).with_style(PaintStyle::Fill).with_anti_alias(true);
            let mut path = Path::new();
            if uniform_radius {
                path.add_round_rect(x, y, w, h, radius);
            } else {
                path.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
            }
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
            if uniform_radius {
                border.add_round_rect(x, y, w, h, radius);
            } else {
                border.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
            }
            canvas.draw_path(&border, &border_paint);
            
            // 内部白色
            let inner_paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
            let mut inner = Path::new();
            let inner_radius = |r: f32| (r - border_width).max(0.0);
            if uniform_radius {
                inner.add_round_rect(
                    x + border_width, 
                    y + border_width, 
                    w - border_width * 2.0, 
                    h - border_width * 2.0, 
                    inner_radius(radius)
                );
            } else {
                inner.add_round_rect_varying(
                    x + border_width, 
                    y + border_width, 
                    w - border_width * 2.0, 
                    h - border_width * 2.0, 
                    inner_radius(radius_tl), inner_radius(radius_tr),
                    inner_radius(radius_br), inner_radius(radius_bl)
                );
            }
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
