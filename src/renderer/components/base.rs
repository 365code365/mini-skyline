//! 组件基础定义

use crate::parser::wxml::WxmlNode;
use crate::parser::wxss::{StyleSheet, StyleValue, LengthUnit, rpx_to_px};
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use std::collections::HashMap;
use taffy::prelude::*;

/// 渲染节点
#[derive(Clone)]
pub struct RenderNode {
    pub tag: String,
    pub text: String,
    pub attrs: HashMap<String, String>,
    pub taffy_node: NodeId,
    pub style: NodeStyle,
    pub children: Vec<RenderNode>,
    pub events: Vec<(String, String, HashMap<String, String>)>,
}

/// 节点样式
#[derive(Clone, Default)]
pub struct NodeStyle {
    pub background_color: Option<Color>,
    pub text_color: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub border_radius: f32,
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub opacity: f32,
    pub text_align: TextAlign,
    pub text_decoration: TextDecoration,
    /// 组件特定数据（如 progress 的百分比、switch 的选中状态等）
    pub custom_data: f32,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum FontWeight {
    #[default]
    Normal,
    Bold,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum TextDecoration {
    #[default]
    None,
    Underline,
    LineThrough,
}

/// 组件上下文
pub struct ComponentContext<'a> {
    pub scale_factor: f32,
    pub screen_width: f32,
    pub screen_height: f32,
    pub stylesheet: &'a StyleSheet,
    pub taffy: &'a mut TaffyTree,
}

/// 组件 trait
pub trait Component {
    fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode>;
    fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32);
}

/// 解析颜色字符串
pub fn parse_color_str(s: &str) -> Option<Color> {
    let s = s.trim();
    if s.starts_with('#') {
        let hex = s.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::new(r, g, b, 255));
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            return Some(Color::new(r, g, b, 255));
        }
    } else if s.starts_with("rgb") {
        let nums: Vec<u8> = s.chars()
            .filter(|c| c.is_ascii_digit() || *c == ',')
            .collect::<String>()
            .split(',')
            .filter_map(|n| n.trim().parse().ok())
            .collect();
        if nums.len() >= 3 {
            return Some(Color::new(nums[0], nums[1], nums[2], 255));
        }
    }
    None
}

/// 提取事件绑定
pub fn extract_events(node: &WxmlNode) -> Vec<(String, String, HashMap<String, String>)> {
    let mut events = vec![];
    for attr in ["bindtap", "catchtap", "bindchange", "bindinput", "bindblur", "bindfocus"] {
        if let Some(h) = node.get_attr(attr) {
            let mut d = HashMap::new();
            for (k, v) in &node.attributes {
                if k.starts_with("data-") { 
                    d.insert(k[5..].into(), v.clone()); 
                }
            }
            let event_type = attr.trim_start_matches("bind").trim_start_matches("catch");
            events.push((event_type.into(), h.into(), d));
        }
    }
    events
}

/// 获取节点的 class 列表
pub fn get_classes(node: &WxmlNode) -> Vec<&str> {
    node.get_attr("class").map(|s| s.split_whitespace().collect()).unwrap_or_default()
}

/// 获取节点的文本内容
pub fn get_text_content(node: &WxmlNode) -> String {
    use crate::parser::wxml::WxmlNodeType;
    let mut s = String::new();
    for c in &node.children {
        if c.node_type == WxmlNodeType::Text { 
            s.push_str(&c.text_content); 
        } else { 
            s.push_str(&get_text_content(c)); 
        }
    }
    s.trim().into()
}

/// 将 StyleValue 转换为像素值
pub fn to_px(v: &StyleValue, screen_width: f32, screen_height: f32) -> Option<f32> {
    match v {
        StyleValue::Length(n, u) => Some(match u {
            LengthUnit::Px => *n,
            LengthUnit::Rpx => rpx_to_px(*n, screen_width),
            LengthUnit::Percent => *n / 100.0 * screen_width,
            LengthUnit::Em | LengthUnit::Rem => *n * 16.0,
            LengthUnit::Vw => *n / 100.0 * screen_width,
            LengthUnit::Vh => *n / 100.0 * screen_height,
        }),
        StyleValue::Number(n) => Some(*n),
        _ => None,
    }
}

/// 将 StyleValue 转换为 Dimension
pub fn to_dimension(v: &StyleValue, screen_width: f32, screen_height: f32, sf: f32) -> Option<Dimension> {
    match v {
        StyleValue::Auto => Some(Dimension::Auto),
        StyleValue::Length(n, LengthUnit::Percent) => Some(percent(*n / 100.0)),
        _ => to_px(v, screen_width, screen_height).map(|px| length(px * sf)),
    }
}

/// 构建基础 Taffy 样式
pub fn build_base_style(
    node: &WxmlNode,
    ctx: &mut ComponentContext,
) -> (Style, NodeStyle) {
    let sf = ctx.scale_factor;
    let classes = get_classes(node);
    let css = ctx.stylesheet.get_styles(&classes, &node.tag_name);
    
    let mut ns = NodeStyle { font_size: 14.0, opacity: 1.0, ..Default::default() };
    let mut ts = Style { display: Display::Flex, flex_direction: FlexDirection::Column, ..Default::default() };

    for (name, value) in &css {
        match name.as_str() {
            "width" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.size.width = v; }
            "height" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.size.height = v; }
            "min-width" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.min_size.width = v; }
            "min-height" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.min_size.height = v; }
            "padding" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) {
                let sv = v * sf;
                ts.padding = Rect { top: length(sv), right: length(sv), bottom: length(sv), left: length(sv) };
            }
            "padding-top" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.top = length(v * sf); }
            "padding-right" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.right = length(v * sf); }
            "padding-bottom" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.bottom = length(v * sf); }
            "padding-left" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.left = length(v * sf); }
            "margin" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) {
                let sv = v * sf;
                ts.margin = Rect { top: length(sv), right: length(sv), bottom: length(sv), left: length(sv) };
            }
            "margin-top" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.top = length(v * sf); }
            "margin-right" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.right = length(v * sf); }
            "margin-bottom" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.bottom = length(v * sf); }
            "margin-left" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.left = length(v * sf); }
            "display" => if let StyleValue::String(s) = value {
                ts.display = match s.as_str() {
                    "none" => Display::None,
                    _ => Display::Flex,
                };
            }
            "flex-direction" => if let StyleValue::String(s) = value {
                ts.flex_direction = match s.as_str() {
                    "row" => FlexDirection::Row,
                    "row-reverse" => FlexDirection::RowReverse,
                    "column-reverse" => FlexDirection::ColumnReverse,
                    _ => FlexDirection::Column,
                };
            }
            "flex-wrap" => if let StyleValue::String(s) = value {
                ts.flex_wrap = match s.as_str() {
                    "wrap" => FlexWrap::Wrap,
                    "wrap-reverse" => FlexWrap::WrapReverse,
                    _ => FlexWrap::NoWrap,
                };
            }
            "flex" | "flex-grow" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.flex_grow = v; }
            "flex-shrink" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.flex_shrink = v; }
            "justify-content" => if let StyleValue::String(s) = value {
                ts.justify_content = Some(match s.as_str() {
                    "center" => JustifyContent::Center,
                    "space-between" => JustifyContent::SpaceBetween,
                    "space-around" => JustifyContent::SpaceAround,
                    "space-evenly" => JustifyContent::SpaceEvenly,
                    "flex-end" => JustifyContent::FlexEnd,
                    _ => JustifyContent::FlexStart,
                });
            }
            "align-items" => if let StyleValue::String(s) = value {
                ts.align_items = Some(match s.as_str() {
                    "center" => AlignItems::Center,
                    "flex-end" => AlignItems::FlexEnd,
                    "stretch" => AlignItems::Stretch,
                    "baseline" => AlignItems::Baseline,
                    _ => AlignItems::FlexStart,
                });
            }
            "gap" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { 
                let sv = v * sf;
                ts.gap = Size { width: length(sv), height: length(sv) }; 
            }
            "background-color" | "background" => if let StyleValue::Color(c) = value { ns.background_color = Some(*c); }
            "color" => if let StyleValue::Color(c) = value { ns.text_color = Some(*c); }
            "border-color" => if let StyleValue::Color(c) = value { ns.border_color = Some(*c); }
            "border-width" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_width = v * sf; }
            "border-radius" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_radius = v * sf; }
            "font-size" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.font_size = v; }
            "font-weight" => if let StyleValue::String(s) = value {
                ns.font_weight = match s.as_str() {
                    "bold" | "700" | "800" | "900" => FontWeight::Bold,
                    _ => FontWeight::Normal,
                };
            }
            "text-align" => if let StyleValue::String(s) = value {
                ns.text_align = match s.as_str() {
                    "center" => TextAlign::Center,
                    "right" => TextAlign::Right,
                    _ => TextAlign::Left,
                };
            }
            "opacity" => if let StyleValue::Number(n) = value { ns.opacity = *n; }
            "position" => if let StyleValue::String(s) = value {
                ts.position = match s.as_str() {
                    "absolute" => Position::Absolute,
                    _ => Position::Relative,
                };
            }
            "top" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.inset.top = LengthPercentageAuto::Length(v * sf); }
            "right" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.inset.right = LengthPercentageAuto::Length(v * sf); }
            "bottom" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.inset.bottom = LengthPercentageAuto::Length(v * sf); }
            "left" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.inset.left = LengthPercentageAuto::Length(v * sf); }
            _ => {}
        }
    }
    (ts, ns)
}

/// 绘制背景和边框
pub fn draw_background(canvas: &mut Canvas, style: &NodeStyle, x: f32, y: f32, w: f32, h: f32) {
    // 绘制背景
    if let Some(bg) = style.background_color {
        let mut paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
        if style.opacity < 1.0 { 
            paint.color.a = (paint.color.a as f32 * style.opacity) as u8; 
        }
        if style.border_radius > 0.0 {
            let mut path = Path::new();
            path.add_round_rect(x, y, w, h, style.border_radius);
            canvas.draw_path(&path, &paint);
        } else {
            canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
        }
    }
    
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
