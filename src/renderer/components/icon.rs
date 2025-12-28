//! icon 组件 - 图标
//! 
//! 微信官方图标类型：
//! - success: 成功（绿色对勾）
//! - success_no_circle: 成功无圆圈
//! - info: 信息（蓝色）
//! - warn: 警告（红色）
//! - waiting: 等待（蓝色）
//! - cancel: 取消（红色X）
//! - download: 下载
//! - search: 搜索
//! - clear: 清除

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::{Canvas, Color, Paint, PaintStyle, Path};
use taffy::prelude::*;

pub struct IconComponent;

impl IconComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let icon_type = node.get_attr("type").unwrap_or("success");
        let icon_size = node.get_attr("size").and_then(|s| s.parse::<f32>().ok()).unwrap_or(23.0);
        let icon_color = node.get_attr("color").and_then(|c| parse_color_str(c));
        
        let size = icon_size * sf;
        ts.size = Size { width: length(size), height: length(size) };
        
        // 根据类型设置默认颜色（微信官方配色）
        let default_color = match icon_type {
            "success" | "success_no_circle" => Color::from_hex(0x09BB07),
            "info" | "info_circle" => Color::from_hex(0x10AEFF),
            "warn" => Color::from_hex(0xF76260),
            "waiting" | "waiting_circle" => Color::from_hex(0x10AEFF),
            "cancel" | "clear" => Color::from_hex(0xF43530),
            "download" => Color::from_hex(0x09BB07),
            "search" => Color::from_hex(0xB2B2B2),
            _ => Color::from_hex(0x09BB07),
        };
        
        ns.text_color = icon_color.or(Some(default_color));
        ns.font_size = icon_size;
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "icon".into(),
            text: icon_type.into(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        let color = node.style.text_color.unwrap_or(Color::from_hex(0x09BB07));
        let icon_type = node.text.as_str();
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        let r = w.min(h) / 2.0;
        
        match icon_type {
            "success" => Self::draw_success(canvas, cx, cy, r, color, true),
            "success_no_circle" => Self::draw_success(canvas, cx, cy, r, color, false),
            "info" | "info_circle" => Self::draw_info(canvas, cx, cy, r, color),
            "warn" => Self::draw_warn(canvas, cx, cy, r, color),
            "waiting" | "waiting_circle" => Self::draw_waiting(canvas, cx, cy, r, color),
            "cancel" | "clear" => Self::draw_cancel(canvas, cx, cy, r, color),
            "download" => Self::draw_download(canvas, cx, cy, r, color),
            "search" => Self::draw_search(canvas, cx, cy, r, color),
            _ => Self::draw_success(canvas, cx, cy, r, color, true),
        }
    }
    
    fn draw_success(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color, with_circle: bool) {
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
        
        if with_circle {
            // 绘制圆形背景
            let mut path = Path::new();
            path.add_circle(cx, cy, r);
            canvas.draw_path(&path, &paint);
            
            // 绘制白色对勾
            let check_paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Stroke);
            let mut check = Path::new();
            check.move_to(cx - r * 0.35, cy);
            check.line_to(cx - r * 0.1, cy + r * 0.25);
            check.line_to(cx + r * 0.35, cy - r * 0.25);
            canvas.draw_path(&check, &check_paint);
        } else {
            // 只绘制对勾
            let stroke_paint = Paint::new().with_color(color).with_style(PaintStyle::Stroke);
            let mut check = Path::new();
            check.move_to(cx - r * 0.5, cy);
            check.line_to(cx - r * 0.1, cy + r * 0.4);
            check.line_to(cx + r * 0.5, cy - r * 0.4);
            canvas.draw_path(&check, &stroke_paint);
        }
    }
    
    fn draw_info(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color) {
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
        
        // 绘制圆形背景
        let mut path = Path::new();
        path.add_circle(cx, cy, r);
        canvas.draw_path(&path, &paint);
        
        // 绘制白色 "i"
        let white = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill);
        // 点
        let mut dot = Path::new();
        dot.add_circle(cx, cy - r * 0.35, r * 0.12);
        canvas.draw_path(&dot, &white);
        // 竖线
        canvas.draw_rect(&crate::Rect::new(cx - r * 0.08, cy - r * 0.1, r * 0.16, r * 0.5), &white);
    }
    
    fn draw_warn(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color) {
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
        
        // 绘制圆形背景
        let mut path = Path::new();
        path.add_circle(cx, cy, r);
        canvas.draw_path(&path, &paint);
        
        // 绘制白色 "!"
        let white = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill);
        // 竖线
        canvas.draw_rect(&crate::Rect::new(cx - r * 0.08, cy - r * 0.45, r * 0.16, r * 0.5), &white);
        // 点
        let mut dot = Path::new();
        dot.add_circle(cx, cy + r * 0.35, r * 0.12);
        canvas.draw_path(&dot, &white);
    }
    
    fn draw_waiting(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color) {
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
        
        // 绘制圆形背景
        let mut path = Path::new();
        path.add_circle(cx, cy, r);
        canvas.draw_path(&path, &paint);
        
        // 绘制白色时钟指针
        let white = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Stroke);
        let mut clock = Path::new();
        clock.move_to(cx, cy);
        clock.line_to(cx, cy - r * 0.35);
        clock.move_to(cx, cy);
        clock.line_to(cx + r * 0.25, cy);
        canvas.draw_path(&clock, &white);
    }
    
    fn draw_cancel(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color) {
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
        
        // 绘制圆形背景
        let mut path = Path::new();
        path.add_circle(cx, cy, r);
        canvas.draw_path(&path, &paint);
        
        // 绘制白色 X
        let white = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Stroke);
        let offset = r * 0.35;
        let mut x_path = Path::new();
        x_path.move_to(cx - offset, cy - offset);
        x_path.line_to(cx + offset, cy + offset);
        x_path.move_to(cx + offset, cy - offset);
        x_path.line_to(cx - offset, cy + offset);
        canvas.draw_path(&x_path, &white);
    }
    
    fn draw_download(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color) {
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Stroke);
        
        // 绘制圆形边框
        let mut circle = Path::new();
        circle.add_circle(cx, cy, r);
        canvas.draw_path(&circle, &paint);
        
        // 绘制下载箭头
        let mut arrow = Path::new();
        arrow.move_to(cx, cy - r * 0.4);
        arrow.line_to(cx, cy + r * 0.2);
        arrow.move_to(cx - r * 0.25, cy);
        arrow.line_to(cx, cy + r * 0.25);
        arrow.line_to(cx + r * 0.25, cy);
        canvas.draw_path(&arrow, &paint);
        
        // 底部横线
        let mut line = Path::new();
        line.move_to(cx - r * 0.35, cy + r * 0.4);
        line.line_to(cx + r * 0.35, cy + r * 0.4);
        canvas.draw_path(&line, &paint);
    }
    
    fn draw_search(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Color) {
        let paint = Paint::new().with_color(color).with_style(PaintStyle::Stroke);
        
        // 绘制放大镜圆圈
        let lens_r = r * 0.5;
        let lens_cx = cx - r * 0.15;
        let lens_cy = cy - r * 0.15;
        let mut lens = Path::new();
        lens.add_circle(lens_cx, lens_cy, lens_r);
        canvas.draw_path(&lens, &paint);
        
        // 绘制手柄
        let mut handle = Path::new();
        handle.move_to(lens_cx + lens_r * 0.7, lens_cy + lens_r * 0.7);
        handle.line_to(cx + r * 0.4, cy + r * 0.4);
        canvas.draw_path(&handle, &paint);
    }
}
