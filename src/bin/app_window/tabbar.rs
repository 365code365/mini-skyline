//! TabBar 相关逻辑

use mini_render::parser::wxml::WxmlNode;
use mini_render::parser::wxss::StyleSheet;
use mini_render::{Canvas, Color, Paint, PaintStyle, Rect};
use mini_render::text::TextRenderer;
use super::config::TabBarConfig;

pub const TABBAR_HEIGHT: u32 = 56;
pub const LOGICAL_WIDTH: u32 = 375;

/// 自定义 TabBar 组件
pub struct CustomTabBar {
    pub wxml_nodes: Vec<WxmlNode>,
    pub stylesheet: StyleSheet,
    pub js_code: String,
}

/// 解析颜色字符串
pub fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim().trim_start_matches('#');
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(Color::new(r, g, b, 255))
    } else {
        None
    }
}

/// 渲染原生 TabBar
pub fn render_native_tabbar(
    canvas: &mut Canvas,
    text_renderer: &TextRenderer,
    tab_bar: &TabBarConfig,
    current_path: &str,
    scale_factor: f64,
) {
    let sf = scale_factor as f32;
    let width = LOGICAL_WIDTH as f32 * sf;
    let _height = TABBAR_HEIGHT as f32 * sf;
    
    // 背景色
    let bg_color = parse_color(&tab_bar.background_color).unwrap_or(Color::WHITE);
    canvas.clear(bg_color);
    
    // 顶部分割线
    let line_paint = Paint::new().with_color(Color::from_hex(0xE5E5E5)).with_style(PaintStyle::Fill);
    canvas.draw_rect(&Rect::new(0.0, 0.0, width, 1.0 * sf), &line_paint);
    
    let normal_color = parse_color(&tab_bar.color).unwrap_or(Color::from_hex(0x999999));
    let selected_color = parse_color(&tab_bar.selected_color).unwrap_or(Color::from_hex(0x007AFF));
    
    let item_count = tab_bar.list.len();
    if item_count == 0 { return; }
    
    let item_width = width / item_count as f32;
    let icon_size = 24.0 * sf;
    let label_size = 10.0 * sf;
    let icon_y = 8.0 * sf;
    let label_y = icon_y + icon_size + 4.0 * sf;
    
    for (i, item) in tab_bar.list.iter().enumerate() {
        let is_selected = item.page_path == current_path;
        let color = if is_selected { selected_color } else { normal_color };
        let x = i as f32 * item_width;
        
        // 绘制图标占位符（用文字代替）
        let icon_text = item.text.chars().next().unwrap_or('?').to_string();
        let icon_paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
        let icon_text_width = text_renderer.measure_text(&icon_text, icon_size);
        let icon_x = x + (item_width - icon_text_width) / 2.0;
        text_renderer.draw_text(canvas, &icon_text, icon_x, icon_y + icon_size, icon_size, &icon_paint);
        
        // 绘制标签
        let label_paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
        let label_width = text_renderer.measure_text(&item.text, label_size);
        let label_x = x + (item_width - label_width) / 2.0;
        text_renderer.draw_text(canvas, &item.text, label_x, label_y + label_size, label_size, &label_paint);
    }
}

/// 处理原生 TabBar 点击，返回目标路径（如果需要切换）
pub fn handle_native_tabbar_click(
    tab_bar: &TabBarConfig,
    x: f32,
    current_path: &str,
) -> Option<String> {
    let item_count = tab_bar.list.len();
    if item_count == 0 { return None; }
    
    let item_width = LOGICAL_WIDTH as f32 / item_count as f32;
    let clicked_index = (x / item_width) as usize;
    
    if clicked_index < item_count {
        let target_path = &tab_bar.list[clicked_index].page_path;
        if target_path != current_path {
            println!("👆 TabBar -> {} ({})", tab_bar.list[clicked_index].text, target_path);
            return Some(target_path.clone());
        }
    }
    None
}
