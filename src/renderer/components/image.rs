//! image 组件 - 图片
//! 
//! 属性：
//! - src: 图片资源地址
//! - mode: 图片裁剪、缩放模式
//!   - scaleToFill: 缩放模式，不保持纵横比缩放图片
//!   - aspectFit: 缩放模式，保持纵横比缩放图片，完整显示
//!   - aspectFill: 缩放模式，保持纵横比缩放图片，只保证短边完全显示
//!   - widthFix: 缩放模式，宽度不变，高度自动变化
//!   - heightFix: 缩放模式，高度不变，宽度自动变化
//! - lazy-load: 懒加载
//! - show-menu-by-longpress: 长按显示菜单

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use taffy::prelude::*;

pub struct ImageComponent;

impl ImageComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let src = node.get_attr("src").unwrap_or("");
        let mode = node.get_attr("mode").unwrap_or("scaleToFill");
        
        // 默认图片大小 320x240（微信官方默认）
        let default_width = 320.0;
        let default_height = 240.0;
        
        if ts.size.width == Dimension::Auto {
            ts.size.width = length(default_width * sf);
        }
        if ts.size.height == Dimension::Auto {
            ts.size.height = length(default_height * sf);
        }
        
        // 图片占位符背景
        ns.background_color = Some(Color::from_hex(0xEEEEEE));
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "image".into(),
            text: src.into(), // 存储 src
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
        
        // 绘制背景占位符
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
        
        // 绘制图片图标占位符
        let icon_size = w.min(h) * 0.3;
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        
        // 绘制山形图标
        let icon_paint = Paint::new()
            .with_color(Color::from_hex(0xCCCCCC))
            .with_style(PaintStyle::Fill);
        
        // 山形
        let mut mountain = Path::new();
        mountain.move_to(cx - icon_size * 0.6, cy + icon_size * 0.3);
        mountain.line_to(cx - icon_size * 0.2, cy - icon_size * 0.1);
        mountain.line_to(cx + icon_size * 0.1, cy + icon_size * 0.2);
        mountain.line_to(cx + icon_size * 0.3, cy - icon_size * 0.3);
        mountain.line_to(cx + icon_size * 0.6, cy + icon_size * 0.3);
        mountain.close();
        canvas.draw_path(&mountain, &icon_paint);
        
        // 太阳
        let mut sun = Path::new();
        sun.add_circle(cx - icon_size * 0.3, cy - icon_size * 0.25, icon_size * 0.12);
        canvas.draw_path(&sun, &icon_paint);
        
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
    }
}
