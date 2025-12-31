//! button 组件 - 按钮
//! 
//! 支持完整的 CSS 样式，同时保留微信默认样式作为 fallback
//! - type: default(灰色) / primary(绿色) / warn(红色)
//! - size: default / mini
//! - plain: 镂空按钮
//! - disabled: 禁用状态

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use taffy::prelude::*;

pub struct ButtonComponent;

impl ButtonComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        // 首先使用 base 的样式解析，获取 CSS 定义的样式
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let text = get_text_content(node);
        let btn_type = node.get_attr("type").unwrap_or("default");
        let btn_size = node.get_attr("size").unwrap_or("default");
        let plain = node.get_attr("plain").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let disabled = node.get_attr("disabled").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        
        // 只有在 CSS 没有定义时才使用微信默认样式
        let has_custom_bg = ns.background_color.is_some();
        let has_custom_color = ns.text_color.is_some();
        let has_custom_border = ns.border_color.is_some() || ns.border_width > 0.0;
        
        // 微信默认配色（仅在没有自定义样式时使用）
        if !has_custom_bg || !has_custom_color {
            let (default_bg, default_fg, default_border) = match btn_type {
                "primary" => {
                    if plain {
                        (Color::WHITE, Color::from_hex(0x07C160), Some(Color::from_hex(0x07C160)))
                    } else {
                        (Color::from_hex(0x07C160), Color::WHITE, None)
                    }
                }
                "warn" => {
                    if plain {
                        (Color::WHITE, Color::from_hex(0xE64340), Some(Color::from_hex(0xE64340)))
                    } else {
                        (Color::from_hex(0xE64340), Color::WHITE, None)
                    }
                }
                _ => { // default
                    if plain {
                        (Color::WHITE, Color::from_hex(0x353535), Some(Color::from_hex(0x353535)))
                    } else {
                        (Color::from_hex(0xF8F8F8), Color::BLACK, Some(Color::from_hex(0xD9D9D9)))
                    }
                }
            };
            
            if !has_custom_bg {
                ns.background_color = Some(default_bg);
            }
            if !has_custom_color {
                ns.text_color = Some(default_fg);
            }
            if !has_custom_border {
                ns.border_color = default_border;
                if default_border.is_some() {
                    ns.border_width = 1.0 * sf;
                }
            }
        }
        
        // 禁用状态覆盖颜色
        if disabled {
            ns.background_color = Some(Color::from_hex(0xF7F7F7));
            ns.text_color = Some(Color::from_hex(0xB2B2B2));
        }
        
        // 只有在 CSS 没有定义尺寸时才使用默认尺寸
        let has_custom_size = !matches!(ts.size.width, Dimension::Auto) || 
                              !matches!(ts.size.height, Dimension::Auto);
        let has_custom_padding = !matches!(ts.padding.top, LengthPercentage::Length(0.0)) ||
                                  !matches!(ts.padding.left, LengthPercentage::Length(0.0));
        
        if !has_custom_size || !has_custom_padding {
            let (font_size, padding_v, padding_h, radius) = match btn_size {
                "mini" => (13.0, 4.0, 12.0, 3.0),
                _ => (18.0, 12.0, 24.0, 5.0),
            };
            
            // 只在没有自定义 font-size 时使用默认值
            if ns.font_size == 14.0 { // 14.0 是 NodeStyle 的默认值
                ns.font_size = font_size;
            }
            
            // 只在没有自定义 border-radius 时使用默认值
            if ns.border_radius == 0.0 {
                ns.border_radius = radius * sf;
            }
            
            if !has_custom_size {
                // 估算文本宽度
                let char_width = ns.font_size * 0.6 * sf;
                let tw = text.chars().count() as f32 * char_width;
                
                ts.size = Size { 
                    width: if btn_size == "mini" { length(tw + padding_h * 2.0 * sf) } else { percent(1.0) },
                    height: length((ns.font_size + padding_v * 2.0) * sf) 
                };
            }
            
            if !has_custom_padding {
                ts.padding = Rect { 
                    top: length(padding_v * sf), 
                    right: length(padding_h * sf), 
                    bottom: length(padding_v * sf), 
                    left: length(padding_h * sf) 
                };
            }
        }
        
        // 默认 margin（如果没有自定义）
        let has_custom_margin = !matches!(ts.margin.top, LengthPercentageAuto::Length(0.0));
        if !has_custom_margin {
            ts.margin = Rect { 
                top: length(5.0 * sf), 
                right: length(0.0), 
                bottom: length(5.0 * sf), 
                left: length(0.0) 
            };
        }
        
        // 默认居中对齐
        if ts.align_items.is_none() {
            ts.align_items = Some(AlignItems::Center);
        }
        if ts.justify_content.is_none() {
            ts.justify_content = Some(JustifyContent::Center);
        }
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "button".into(),
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
        Self::draw_with_state(node, canvas, text_renderer, x, y, w, h, sf, false);
    }
    
    pub fn draw_with_state(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        sf: f32,
        pressed: bool,
    ) {
        let style = &node.style;
        let disabled = node.attrs.get("disabled")
            .map(|s| s == "true" || s == "{{true}}")
            .unwrap_or(false);
        
        // 获取圆角值（支持四个角独立设置）
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
        
        // 获取背景色，按下时变暗
        let bg = if let Some(bg) = style.background_color {
            if pressed && !disabled {
                Self::darken_color(bg, 0.1)
            } else {
                bg
            }
        } else {
            Color::WHITE
        };
        
        // 应用透明度
        let bg = if style.opacity < 1.0 {
            Color::new(bg.r, bg.g, bg.b, (bg.a as f32 * style.opacity) as u8)
        } else {
            bg
        };
        
        // 绘制背景
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
        
        // 绘制边框
        if style.border_width > 0.0 {
            if let Some(bc) = style.border_color {
                let border_color = if pressed && !disabled {
                    Self::darken_color(bc, 0.1)
                } else {
                    bc
                };
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
        }
        
        // 按下时绘制半透明遮罩
        if pressed && !disabled {
            let overlay = Paint::new()
                .with_color(Color::new(0, 0, 0, 25))
                .with_style(PaintStyle::Fill);
            if has_radius {
                let mut path = Path::new();
                if uniform_radius {
                    path.add_round_rect(x, y, w, h, radius_tl);
                } else {
                    path.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
                }
                canvas.draw_path(&path, &overlay);
            } else {
                canvas.draw_rect(&GeoRect::new(x, y, w, h), &overlay);
            }
        }
        
        // 绘制文本
        if let Some(tr) = text_renderer {
            let color = style.text_color.unwrap_or(Color::WHITE);
            let size = style.font_size * sf;
            let tw = tr.measure_text(&node.text, size);
            
            // 根据 text-align 计算 x 位置
            let tx = match style.text_align {
                TextAlign::Center => x + (w - tw) / 2.0,
                TextAlign::Right => x + w - tw - 8.0 * sf, // 右边留点 padding
                TextAlign::Left | TextAlign::Justify => x + 8.0 * sf, // 左边留点 padding
            };
            
            // 垂直居中
            let ty = y + (h - size) / 2.0 + size;
            
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            tr.draw_text(canvas, &node.text, tx, ty, size, &paint);
        }
    }
    
    /// 使颜色变暗
    fn darken_color(color: Color, amount: f32) -> Color {
        let factor = 1.0 - amount;
        Color::new(
            (color.r as f32 * factor) as u8,
            (color.g as f32 * factor) as u8,
            (color.b as f32 * factor) as u8,
            color.a,
        )
    }
}
