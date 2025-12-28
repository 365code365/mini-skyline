//! slider 组件 - 滑动选择器
//! 
//! 属性：
//! - min: 最小值，默认 0
//! - max: 最大值，默认 100
//! - step: 步长，默认 1
//! - value: 当前值
//! - activeColor: 已选择的颜色
//! - backgroundColor: 背景条颜色
//! - block-size: 滑块大小，默认 28
//! - block-color: 滑块颜色
//! - show-value: 是否显示当前值

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path};
use taffy::prelude::*;

pub struct SliderComponent;

impl SliderComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let value = node.get_attr("value").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
        let min = node.get_attr("min").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
        let max = node.get_attr("max").and_then(|s| s.parse::<f32>().ok()).unwrap_or(100.0);
        let active_color = node.get_attr("activeColor").or(node.get_attr("active-color"))
            .and_then(|c| parse_color_str(c)).unwrap_or(Color::from_hex(0x1AAD19));
        let bg_color = node.get_attr("backgroundColor").or(node.get_attr("background-color"))
            .and_then(|c| parse_color_str(c)).unwrap_or(Color::from_hex(0xE9E9E9));
        let block_size = node.get_attr("block-size").and_then(|s| s.parse::<f32>().ok()).unwrap_or(28.0);
        let block_color = node.get_attr("block-color").and_then(|c| parse_color_str(c))
            .unwrap_or(Color::WHITE);
        let show_value = node.get_attr("show-value").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        
        // 滑块高度
        let height = block_size * sf;
        ts.size = Size { width: percent(1.0), height: length(height) };
        ts.flex_direction = FlexDirection::Row;
        ts.align_items = Some(AlignItems::Center);
        
        ns.background_color = Some(bg_color);
        ns.text_color = Some(active_color);
        ns.border_color = Some(block_color);
        ns.custom_data = ((value - min) / (max - min)).clamp(0.0, 1.0);
        ns.border_width = block_size;
        ns.font_size = value; // 存储当前值用于显示
        
        let text = if show_value { format!("{}", value as i32) } else { String::new() };
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "slider".into(),
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
        let progress = style.custom_data;
        let block_size = style.border_width * sf;
        let show_value = !node.text.is_empty();
        
        // 计算轨道区域
        let value_width = if show_value { 40.0 * sf } else { 0.0 };
        let track_width = w - value_width - block_size;
        let track_height = 4.0 * sf;
        let track_x = x + block_size / 2.0;
        let track_y = y + (h - track_height) / 2.0;
        
        // 绘制背景轨道
        if let Some(bg) = style.background_color {
            let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
            let mut path = Path::new();
            path.add_round_rect(track_x, track_y, track_width, track_height, track_height / 2.0);
            canvas.draw_path(&path, &paint);
        }
        
        // 绘制已选择部分
        let active_width = track_width * progress;
        if active_width > 0.0 {
            if let Some(active) = style.text_color {
                let paint = Paint::new().with_color(active).with_style(PaintStyle::Fill);
                let mut path = Path::new();
                path.add_round_rect(track_x, track_y, active_width, track_height, track_height / 2.0);
                canvas.draw_path(&path, &paint);
            }
        }
        
        // 绘制滑块
        let knob_x = track_x + active_width;
        let knob_y = y + h / 2.0;
        let knob_radius = block_size / 2.0;
        
        // 滑块阴影
        let shadow_paint = Paint::new()
            .with_color(Color::new(0, 0, 0, 40))
            .with_style(PaintStyle::Fill);
        let mut shadow = Path::new();
        shadow.add_circle(knob_x, knob_y + 2.0 * sf, knob_radius);
        canvas.draw_path(&shadow, &shadow_paint);
        
        // 滑块本体
        let block_color = style.border_color.unwrap_or(Color::WHITE);
        let knob_paint = Paint::new().with_color(block_color).with_style(PaintStyle::Fill);
        let mut knob = Path::new();
        knob.add_circle(knob_x, knob_y, knob_radius);
        canvas.draw_path(&knob, &knob_paint);
        
        // 滑块边框
        let border_paint = Paint::new()
            .with_color(Color::from_hex(0xE9E9E9))
            .with_style(PaintStyle::Stroke);
        let mut border = Path::new();
        border.add_circle(knob_x, knob_y, knob_radius);
        canvas.draw_path(&border, &border_paint);
        
        // 绘制数值
        if show_value {
            if let Some(tr) = text_renderer {
                let font_size = 14.0 * sf;
                let text_x = x + w - value_width + 8.0 * sf;
                let text_y = y + (h + font_size) / 2.0;
                let paint = Paint::new().with_color(Color::from_hex(0x888888)).with_style(PaintStyle::Fill);
                tr.draw_text(canvas, &node.text, text_x, text_y, font_size, &paint);
            }
        }
    }
}
