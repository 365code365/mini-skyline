//! progress 组件 - 进度条
//! 
//! 属性：
//! - percent: 进度百分比 0-100
//! - stroke-width: 进度条宽度，默认 6px
//! - activeColor: 已选择的进度条颜色
//! - backgroundColor: 未选择的进度条颜色
//! - active: 进度条从左往右的动画
//! - show-info: 在进度条右侧显示百分比

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use taffy::prelude::*;

pub struct ProgressComponent;

impl ProgressComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let pct = node.get_attr("percent").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
        let stroke_width = node.get_attr("stroke-width").and_then(|s| s.parse::<f32>().ok()).unwrap_or(6.0);
        let active_color = node.get_attr("activeColor").or(node.get_attr("active-color"))
            .and_then(|c| parse_color_str(c)).unwrap_or(Color::from_hex(0x09BB07));
        let bg_color = node.get_attr("backgroundColor").or(node.get_attr("background-color"))
            .and_then(|c| parse_color_str(c)).unwrap_or(Color::from_hex(0xEBEBEB));
        let show_info = node.get_attr("show-info").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        
        // 进度条高度 - 使用固定的容器高度，内部绘制时使用 stroke_width
        let container_height = (stroke_width + 4.0) * sf; // 增加一点 padding
        
        // 保持原有的 margin 设置
        ts.size = Size { width: percent(1.0), height: length(container_height) };
        ts.flex_direction = FlexDirection::Row;
        ts.align_items = Some(AlignItems::Center);
        ts.margin.bottom = length(4.0 * sf); // 添加底部间距
        
        ns.background_color = Some(bg_color);
        ns.text_color = Some(active_color);
        ns.custom_data = pct.clamp(0.0, 100.0);
        ns.border_width = stroke_width; // 存储实际的进度条高度
        ns.border_radius = stroke_width * sf / 2.0;
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        // 存储 show_info 状态
        let text = if show_info { format!("{}%", pct as i32) } else { String::new() };
        
        Some(RenderNode {
            tag: "progress".into(),
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
        let percent = style.custom_data / 100.0;
        let stroke_width = style.border_width * sf; // 实际进度条高度
        let radius = stroke_width / 2.0; // 圆角半径
        
        // 计算进度条实际宽度（如果显示百分比，需要减去文字宽度）
        let show_info = !node.text.is_empty();
        let info_width = if show_info { 50.0 * sf } else { 0.0 };
        let bar_width = w - info_width;
        
        // 垂直居中
        let bar_y = y + (h - stroke_width) / 2.0;
        
        // 绘制背景轨道
        if let Some(bg) = style.background_color {
            let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill).with_anti_alias(true);
            if radius > 0.0 {
                let mut path = Path::new();
                path.add_round_rect(x, bar_y, bar_width, stroke_width, radius);
                canvas.draw_path(&path, &paint);
            } else {
                canvas.draw_rect(&GeoRect::new(x, bar_y, bar_width, stroke_width), &paint);
            }
        }
        
        // 绘制进度
        if percent > 0.0 {
            if let Some(active) = style.text_color {
                let paint = Paint::new().with_color(active).with_style(PaintStyle::Fill).with_anti_alias(true);
                let progress_width = (bar_width * percent).max(stroke_width); // 最小宽度为高度，保证圆角
                if radius > 0.0 {
                    let mut path = Path::new();
                    path.add_round_rect(x, bar_y, progress_width, stroke_width, radius);
                    canvas.draw_path(&path, &paint);
                } else {
                    canvas.draw_rect(&GeoRect::new(x, bar_y, progress_width, stroke_width), &paint);
                }
            }
        }
        
        // 绘制百分比文字
        if show_info {
            if let Some(tr) = text_renderer {
                let font_size = 12.0 * sf;
                let text_x = x + bar_width + 8.0 * sf;
                let text_y = bar_y + (stroke_width + font_size) / 2.0 - 2.0 * sf;
                let paint = Paint::new().with_color(Color::from_hex(0x999999)).with_style(PaintStyle::Fill);
                tr.draw_text(canvas, &node.text, text_x, text_y, font_size, &paint);
            }
        }
    }
}
