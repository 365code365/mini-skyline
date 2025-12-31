//! input 组件 - 输入框
//! 
//! 支持完整的 CSS 样式，同时保留微信默认样式作为 fallback
//! 
//! 属性：
//! - value: 输入框的值
//! - type: text / number / idcard / digit / safe-password / nickname
//! - password: 是否是密码类型
//! - placeholder: 占位符
//! - placeholder-style: 占位符样式
//! - placeholder-class: 占位符样式类
//! - disabled: 是否禁用
//! - maxlength: 最大输入长度
//! - focus: 是否获取焦点

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use taffy::prelude::*;

pub struct InputComponent;

impl InputComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        // 使用 base 的样式解析，获取 CSS 定义的样式
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let value = node.get_attr("value").unwrap_or("");
        let placeholder = node.get_attr("placeholder").unwrap_or("");
        let password = node.get_attr("password").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let disabled = node.get_attr("disabled").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let is_textarea = node.tag_name == "textarea";
        
        // 检查 CSS 是否定义了样式
        let has_custom_width = !matches!(ts.size.width, Dimension::Auto);
        let has_custom_height = !matches!(ts.size.height, Dimension::Auto);
        let has_custom_padding = !matches!(ts.padding.top, LengthPercentage::Length(0.0)) ||
                                  !matches!(ts.padding.left, LengthPercentage::Length(0.0));
        let has_custom_bg = ns.background_color.is_some();
        let has_custom_border = ns.border_color.is_some() || ns.border_width > 0.0;
        let has_custom_radius = ns.border_radius > 0.0;
        let has_custom_font_size = ns.font_size != 14.0; // 14.0 是 NodeStyle 默认值
        
        // 尺寸处理 - 支持 flex 布局
        if !has_custom_width {
            // 如果设置了 flex-grow，不设置固定宽度
            if ts.flex_grow == 0.0 {
                ts.size.width = percent(1.0);
            }
        }
        
        // 允许收缩
        if ts.flex_shrink == 0.0 {
            ts.flex_shrink = 1.0;
        }
        
        // 默认高度
        if !has_custom_height {
            ts.size.height = length(if is_textarea { 80.0 * sf } else { 42.0 * sf });
        }
        
        // 默认 padding
        if !has_custom_padding {
            ts.padding = Rect { 
                top: length(8.0 * sf), 
                right: length(12.0 * sf), 
                bottom: length(8.0 * sf), 
                left: length(12.0 * sf) 
            };
        }
        
        // 默认背景色
        if !has_custom_bg {
            ns.background_color = Some(Color::WHITE);
        }
        
        // 默认边框
        if !has_custom_border {
            ns.border_color = Some(Color::from_hex(0xD9D9D9));
            ns.border_width = 1.0 * sf;
        }
        
        // 默认圆角
        if !has_custom_radius {
            ns.border_radius = 4.0 * sf;
        }
        
        // 默认字体大小
        if !has_custom_font_size {
            ns.font_size = 16.0;
        }
        
        // 显示文本
        let display_text = if value.is_empty() {
            placeholder.to_string()
        } else if password {
            "•".repeat(value.len())
        } else {
            value.to_string()
        };
        
        // 文本颜色 - 只在没有自定义颜色时使用默认值
        if ns.text_color.is_none() {
            ns.text_color = Some(if value.is_empty() {
                Color::from_hex(0xBFBFBF) // placeholder 颜色
            } else if disabled {
                Color::from_hex(0xBFBFBF)
            } else {
                Color::BLACK
            });
        }
        
        // 禁用状态背景（覆盖自定义样式）
        if disabled && !has_custom_bg {
            ns.background_color = Some(Color::from_hex(0xF5F5F5));
        }
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: node.tag_name.clone(),
            text: display_text,
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
        Self::draw_with_cursor(node, canvas, text_renderer, x, y, w, h, sf, false, 0);
    }
    
    pub fn draw_with_cursor(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        sf: f32,
        focused: bool,
        cursor_pos: usize,
    ) {
        Self::draw_with_selection(node, canvas, text_renderer, x, y, w, h, sf, focused, cursor_pos, None);
    }
    
    pub fn draw_with_selection(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        sf: f32,
        focused: bool,
        cursor_pos: usize,
        selection: Option<(usize, usize)>,
    ) {
        let style = &node.style;
        
        // 获取圆角值（支持四角独立设置）
        let radius_tl = style.border_radius_tl.unwrap_or(style.border_radius);
        let radius_tr = style.border_radius_tr.unwrap_or(style.border_radius);
        let radius_br = style.border_radius_br.unwrap_or(style.border_radius);
        let radius_bl = style.border_radius_bl.unwrap_or(style.border_radius);
        let has_radius = radius_tl > 0.0 || radius_tr > 0.0 || radius_br > 0.0 || radius_bl > 0.0;
        let uniform_radius = radius_tl == radius_tr && radius_tr == radius_br && radius_br == radius_bl;
        
        // 绘制盒子阴影
        if let Some(shadow) = &style.box_shadow {
            draw_box_shadow(canvas, shadow, x, y, w, h, style.border_radius);
        }
        
        // 绘制背景
        if let Some(bg) = style.background_color {
            // 应用透明度
            let bg = if style.opacity < 1.0 {
                Color::new(bg.r, bg.g, bg.b, (bg.a as f32 * style.opacity) as u8)
            } else {
                bg
            };
            
            let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
            if has_radius {
                let mut path = Path::new();
                if uniform_radius {
                    path.add_round_rect(x, y, w, h, radius_tl);
                } else {
                    path.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
                }
                canvas.draw_path(&path, &paint);
            } else {
                canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
            }
        }
        
        // 绘制边框 - 聚焦时高亮（除非有自定义边框颜色）
        let border_color = if focused {
            Color::from_hex(0x07C160) // 微信绿色
        } else {
            style.border_color.unwrap_or(Color::from_hex(0xD9D9D9))
        };
        
        if style.border_width > 0.0 {
            let paint = Paint::new().with_color(border_color).with_style(PaintStyle::Stroke);
            if has_radius {
                let mut path = Path::new();
                if uniform_radius {
                    path.add_round_rect(x, y, w, h, radius_tl);
                } else {
                    path.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
                }
                canvas.draw_path(&path, &paint);
            } else {
                canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
            }
        }
        
        // 计算文本位置
        let font_size = style.font_size * sf;
        let padding_left = 12.0 * sf; // 默认左边距
        let text_x = x + padding_left;
        
        // 根据 text-align 和 vertical-align 计算位置
        let text_y = match style.vertical_align {
            VerticalAlign::Top => y + font_size + 4.0 * sf,
            VerticalAlign::Bottom => y + h - 4.0 * sf,
            _ => y + (h + font_size) / 2.0 - 2.0 * sf, // Middle/Baseline - 垂直居中
        };
        
        if let Some(tr) = text_renderer {
            // 绘制选中背景
            if let Some((sel_start, sel_end)) = selection {
                if sel_start != sel_end && focused {
                    let start_text: String = node.text.chars().take(sel_start).collect();
                    let sel_text: String = node.text.chars().skip(sel_start).take(sel_end - sel_start).collect();
                    
                    let sel_x = text_x + tr.measure_text(&start_text, font_size);
                    let sel_w = tr.measure_text(&sel_text, font_size);
                    let sel_y = y + (h - font_size) / 2.0 - 2.0 * sf;
                    let sel_h = font_size + 4.0 * sf;
                    
                    let sel_paint = Paint::new()
                        .with_color(Color::new(7, 193, 96, 80)) // 半透明绿色
                        .with_style(PaintStyle::Fill);
                    canvas.draw_rect(&GeoRect::new(sel_x, sel_y, sel_w, sel_h), &sel_paint);
                }
            }
            
            // 绘制文本
            let color = style.text_color.unwrap_or(Color::BLACK);
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            
            if !node.text.is_empty() {
                // 根据 text-align 调整 x 位置
                let text_width = tr.measure_text(&node.text, font_size);
                let available_width = w - padding_left * 2.0;
                
                let final_x = match style.text_align {
                    TextAlign::Center => text_x + (available_width - text_width) / 2.0,
                    TextAlign::Right => text_x + available_width - text_width,
                    _ => text_x, // Left
                };
                
                tr.draw_text(canvas, &node.text, final_x, text_y, font_size, &paint);
            }
            
            // 绘制光标（只在没有选中或选中范围为空时显示）
            if focused && selection.map(|(s, e)| s == e).unwrap_or(true) {
                let cursor_text: String = node.text.chars().take(cursor_pos).collect();
                let cursor_x = text_x + tr.measure_text(&cursor_text, font_size);
                let cursor_y1 = y + (h - font_size) / 2.0;
                let cursor_y2 = cursor_y1 + font_size;
                
                let cursor_paint = Paint::new()
                    .with_color(Color::from_hex(0x07C160))
                    .with_style(PaintStyle::Stroke);
                
                let mut cursor_path = Path::new();
                cursor_path.move_to(cursor_x, cursor_y1);
                cursor_path.line_to(cursor_x, cursor_y2);
                canvas.draw_path(&cursor_path, &cursor_paint);
            }
        }
    }
}
