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
        let _mode = node.get_attr("mode").unwrap_or("scaleToFill");
        
        // 默认图片大小 150x100（更合理的默认尺寸）
        let default_width = 150.0;
        let default_height = 100.0;
        
        if ts.size.width == Dimension::Auto {
            ts.size.width = length(default_width * sf);
        }
        if ts.size.height == Dimension::Auto {
            ts.size.height = length(default_height * sf);
        }
        
        // 图片占位符背景
        ns.background_color = Some(Color::from_hex(0xF5F5F5));
        ns.border_radius = 4.0 * sf;
        
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
        _text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        _sf: f32
    ) {
        let style = &node.style;
        let radius = style.border_radius;
        
        // 绘制背景占位符
        let bg_color = style.background_color.unwrap_or(Color::from_hex(0xF5F5F5));
        let bg_paint = Paint::new()
            .with_color(bg_color)
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        
        if radius > 0.0 {
            let mut path = Path::new();
            path.add_round_rect(x, y, w, h, radius);
            canvas.draw_path(&path, &bg_paint);
        } else {
            canvas.draw_rect(&GeoRect::new(x, y, w, h), &bg_paint);
        }
        
        // 绘制图片图标占位符（山形+太阳）
        let icon_size = w.min(h) * 0.35;
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        
        let icon_color = Color::from_hex(0xCCCCCC);
        let icon_paint = Paint::new()
            .with_color(icon_color)
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        
        // 太阳（圆形，使用抗锯齿）
        let sun_x = cx - icon_size * 0.25;
        let sun_y = cy - icon_size * 0.3;
        let sun_r = icon_size * 0.15;
        canvas.draw_circle(sun_x, sun_y, sun_r, &icon_paint);
        
        // 山形（三角形）
        let mut mountain = Path::new();
        // 左边小山
        mountain.move_to(cx - icon_size * 0.5, cy + icon_size * 0.35);
        mountain.line_to(cx - icon_size * 0.15, cy - icon_size * 0.05);
        mountain.line_to(cx + icon_size * 0.1, cy + icon_size * 0.35);
        mountain.close();
        canvas.draw_path(&mountain, &icon_paint);
        
        // 右边大山
        let mut mountain2 = Path::new();
        mountain2.move_to(cx - icon_size * 0.1, cy + icon_size * 0.35);
        mountain2.line_to(cx + icon_size * 0.25, cy - icon_size * 0.25);
        mountain2.line_to(cx + icon_size * 0.55, cy + icon_size * 0.35);
        mountain2.close();
        canvas.draw_path(&mountain2, &icon_paint);
        
        // 绘制边框
        if style.border_width > 0.0 {
            if let Some(bc) = style.border_color {
                let border_paint = Paint::new()
                    .with_color(bc)
                    .with_style(PaintStyle::Stroke)
                    .with_anti_alias(true);
                if radius > 0.0 {
                    let mut path = Path::new();
                    path.add_round_rect(x, y, w, h, radius);
                    canvas.draw_path(&path, &border_paint);
                } else {
                    canvas.draw_rect(&GeoRect::new(x, y, w, h), &border_paint);
                }
            }
        }
    }
}
