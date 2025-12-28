//! WXML 渲染器 - 完整支持微信小程序样式

use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::parser::wxss::{StyleSheet, StyleValue, LengthUnit, rpx_to_px};
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use taffy::prelude::*;

#[derive(Debug, Clone)]
pub struct EventBinding {
    pub event_type: String,
    pub handler: String,
    pub data: HashMap<String, String>,
    pub bounds: GeoRect,
}

struct RenderNode {
    tag: String,
    text: String,
    taffy_node: NodeId,
    style: NodeStyle,
    children: Vec<RenderNode>,
    events: Vec<(String, String, HashMap<String, String>)>,
}

#[derive(Clone, Default)]
struct NodeStyle {
    background_color: Option<Color>,
    text_color: Option<Color>,
    border_radius: f32,
    font_size: f32,
    opacity: f32,
}

pub struct WxmlRenderer {
    stylesheet: StyleSheet,
    screen_width: f32,
    screen_height: f32,
    event_bindings: Vec<EventBinding>,
    text_renderer: Option<TextRenderer>,
    scale_factor: f32,
}

impl WxmlRenderer {
    pub fn new(stylesheet: StyleSheet, screen_width: f32, screen_height: f32) -> Self {
        Self::new_with_scale(stylesheet, screen_width, screen_height, 1.0)
    }
    
    pub fn new_with_scale(stylesheet: StyleSheet, screen_width: f32, screen_height: f32, scale_factor: f32) -> Self {
        let text_renderer = TextRenderer::load_system_font()
            .or_else(|_| TextRenderer::from_bytes(include_bytes!("../../assets/ArialUnicode.ttf")))
            .ok();
        
        if text_renderer.is_some() { 
            println!("✅ Text renderer ready (scale: {}x)", scale_factor); 
        }
        
        Self { 
            stylesheet, 
            screen_width,
            screen_height,
            event_bindings: Vec::new(), 
            text_renderer,
            scale_factor,
        }
    }

    pub fn render(&mut self, canvas: &mut Canvas, nodes: &[WxmlNode], data: &JsonValue) {
        self.event_bindings.clear();
        let rendered = crate::parser::TemplateEngine::render(nodes, data);
        let mut taffy = TaffyTree::new();
        
        let mut render_nodes = Vec::new();
        for node in &rendered {
            if let Some(rn) = self.build_tree(&mut taffy, node) {
                render_nodes.push(rn);
            }
        }
        
        let child_ids: Vec<NodeId> = render_nodes.iter().map(|n| n.taffy_node).collect();
        let root = taffy.new_with_children(
            Style {
                size: Size { width: length(self.screen_width * self.scale_factor), height: auto() },
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &child_ids,
        ).unwrap();
        
        taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
        
        for rn in &render_nodes {
            self.draw(canvas, &taffy, rn, 0.0, 0.0);
        }
    }

    fn build_tree(&self, taffy: &mut TaffyTree, node: &WxmlNode) -> Option<RenderNode> {
        let sf = self.scale_factor;
        
        if node.node_type == WxmlNodeType::Text {
            let text = node.text_content.trim();
            if text.is_empty() { return None; }
            let fs = 14.0;
            let tw = self.measure_text(text, fs * sf);
            let tn = taffy.new_leaf(Style {
                size: Size { width: length(tw), height: length((fs + 4.0) * sf) },
                ..Default::default()
            }).unwrap();
            return Some(RenderNode {
                tag: "#text".into(), text: text.into(), taffy_node: tn,
                style: NodeStyle { font_size: fs, text_color: Some(Color::BLACK), opacity: 1.0, ..Default::default() },
                children: vec![], events: vec![],
            });
        }
        if node.node_type != WxmlNodeType::Element { return None; }

        let classes = self.get_classes(node);
        let css = self.stylesheet.get_styles(&classes, &node.tag_name);
        let (mut ts, mut ns) = self.build_styles(&css, &node.tag_name);

        let mut events = vec![];
        for attr in ["bindtap", "catchtap"] {
            if let Some(h) = node.get_attr(attr) {
                let mut d = HashMap::new();
                for (k, v) in &node.attributes {
                    if k.starts_with("data-") { d.insert(k[5..].into(), v.clone()); }
                }
                events.push(("tap".into(), h.into(), d));
            }
        }

        let btn_text = if node.tag_name == "button" {
            let t = self.get_text(node);
            let tw = self.measure_text(&t, ns.font_size * sf);
            ts.size = Size { width: length(tw + 32.0 * sf), height: length((ns.font_size + 20.0) * sf) };
            t
        } else { String::new() };

        let mut children = vec![];
        if node.tag_name != "button" {
            for c in &node.children {
                if let Some(cr) = self.build_tree(taffy, c) { children.push(cr); }
            }
        }

        let cids: Vec<NodeId> = children.iter().map(|c| c.taffy_node).collect();
        let tn = taffy.new_with_children(ts, &cids).unwrap();
        Some(RenderNode { tag: node.tag_name.clone(), text: btn_text, taffy_node: tn, style: ns, children, events })
    }

    fn build_styles(&self, props: &HashMap<String, StyleValue>, tag: &str) -> (Style, NodeStyle) {
        let sf = self.scale_factor;
        let mut ns = NodeStyle { font_size: 14.0, opacity: 1.0, ..Default::default() };
        let mut ts = Style { display: Display::Flex, flex_direction: FlexDirection::Column, ..Default::default() };

        if tag == "button" {
            ns.background_color = Some(Color::from_hex(0x07C160));
            ns.text_color = Some(Color::WHITE);
            ns.border_radius = 5.0 * sf;
            ns.font_size = 17.0;
            ts.padding = Rect { top: length(10.0 * sf), right: length(16.0 * sf), bottom: length(10.0 * sf), left: length(16.0 * sf) };
        }

        for (name, value) in props {
            match name.as_str() {
                "width" => if let Some(v) = self.to_dimension(value) { ts.size.width = v; }
                "height" => if let Some(v) = self.to_dimension(value) { ts.size.height = v; }
                "padding" => if let Some(v) = self.to_px(value) {
                    let sv = v * sf;
                    ts.padding = Rect { top: length(sv), right: length(sv), bottom: length(sv), left: length(sv) };
                }
                "padding-top" => if let Some(v) = self.to_px(value) { ts.padding.top = length(v * sf); }
                "padding-right" => if let Some(v) = self.to_px(value) { ts.padding.right = length(v * sf); }
                "padding-bottom" => if let Some(v) = self.to_px(value) { ts.padding.bottom = length(v * sf); }
                "padding-left" => if let Some(v) = self.to_px(value) { ts.padding.left = length(v * sf); }
                "margin" => if let Some(v) = self.to_px(value) {
                    let sv = v * sf;
                    ts.margin = Rect { top: length(sv), right: length(sv), bottom: length(sv), left: length(sv) };
                }
                "margin-top" => if let Some(v) = self.to_px(value) { ts.margin.top = length(v * sf); }
                "margin-bottom" => if let Some(v) = self.to_px(value) { ts.margin.bottom = length(v * sf); }
                "display" => if let StyleValue::String(s) = value {
                    ts.display = if s == "flex" { Display::Flex } else if s == "none" { Display::None } else { Display::Flex };
                }
                "flex-direction" => if let StyleValue::String(s) = value {
                    ts.flex_direction = if s == "row" { FlexDirection::Row } else { FlexDirection::Column };
                }
                "justify-content" => if let StyleValue::String(s) = value {
                    ts.justify_content = Some(match s.as_str() {
                        "center" => JustifyContent::Center,
                        "space-between" => JustifyContent::SpaceBetween,
                        "space-around" => JustifyContent::SpaceAround,
                        "flex-end" => JustifyContent::FlexEnd,
                        _ => JustifyContent::FlexStart,
                    });
                }
                "align-items" => if let StyleValue::String(s) = value {
                    ts.align_items = Some(match s.as_str() {
                        "center" => AlignItems::Center,
                        "flex-end" => AlignItems::FlexEnd,
                        "stretch" => AlignItems::Stretch,
                        _ => AlignItems::FlexStart,
                    });
                }
                "gap" => if let Some(v) = self.to_px(value) { 
                    let sv = v * sf;
                    ts.gap = Size { width: length(sv), height: length(sv) }; 
                }
                "background-color" | "background" => if let StyleValue::Color(c) = value { ns.background_color = Some(*c); }
                "color" => if let StyleValue::Color(c) = value { ns.text_color = Some(*c); }
                "border-radius" => if let Some(v) = self.to_px(value) { ns.border_radius = v * sf; }
                "font-size" => if let Some(v) = self.to_px(value) { ns.font_size = v; }
                "opacity" => if let StyleValue::Number(n) = value { ns.opacity = *n; }
                _ => {}
            }
        }
        (ts, ns)
    }

    fn draw(&mut self, canvas: &mut Canvas, taffy: &TaffyTree, node: &RenderNode, ox: f32, oy: f32) {
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);

        if let Some(bg) = node.style.background_color {
            let mut paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
            if node.style.opacity < 1.0 { paint.color.a = (paint.color.a as f32 * node.style.opacity) as u8; }
            if node.style.border_radius > 0.0 {
                let mut path = Path::new();
                path.add_round_rect(x, y, w, h, node.style.border_radius);
                canvas.draw_path(&path, &paint);
            } else {
                canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
            }
        }

        // 确定文本颜色（用于子元素继承）
        let text_color = node.style.text_color.unwrap_or(Color::BLACK);

        match node.tag.as_str() {
            "#text" => {
                // 文本节点使用自己的颜色或继承的颜色
                let c = node.style.text_color.unwrap_or(text_color);
                self.draw_text(canvas, &node.text, x, y, node.style.font_size * sf, c);
            }
            "button" => {
                let c = node.style.text_color.unwrap_or(Color::WHITE);
                let scaled_fs = node.style.font_size * sf;
                let tw = self.measure_text(&node.text, scaled_fs);
                self.draw_text(canvas, &node.text, x + (w - tw) / 2.0, y + (h - scaled_fs) / 2.0, scaled_fs, c);
            }
            _ => {
                for child in &node.children { 
                    self.draw_with_color(canvas, taffy, child, x, y, text_color); 
                }
            }
        }

        for (et, h, d) in &node.events {
            self.event_bindings.push(EventBinding { event_type: et.clone(), handler: h.clone(), data: d.clone(), bounds: logical_bounds });
        }
    }
    
    fn draw_with_color(&mut self, canvas: &mut Canvas, taffy: &TaffyTree, node: &RenderNode, ox: f32, oy: f32, inherited_color: Color) {
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);

        if let Some(bg) = node.style.background_color {
            let mut paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
            if node.style.opacity < 1.0 { paint.color.a = (paint.color.a as f32 * node.style.opacity) as u8; }
            if node.style.border_radius > 0.0 {
                let mut path = Path::new();
                path.add_round_rect(x, y, w, h, node.style.border_radius);
                canvas.draw_path(&path, &paint);
            } else {
                canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
            }
        }

        // 使用自己的颜色或继承的颜色
        let text_color = node.style.text_color.unwrap_or(inherited_color);

        match node.tag.as_str() {
            "#text" => {
                self.draw_text(canvas, &node.text, x, y, node.style.font_size * sf, text_color);
            }
            "button" => {
                let c = node.style.text_color.unwrap_or(Color::WHITE);
                let scaled_fs = node.style.font_size * sf;
                let tw = self.measure_text(&node.text, scaled_fs);
                self.draw_text(canvas, &node.text, x + (w - tw) / 2.0, y + (h - scaled_fs) / 2.0, scaled_fs, c);
            }
            _ => {
                for child in &node.children { 
                    self.draw_with_color(canvas, taffy, child, x, y, text_color); 
                }
            }
        }

        for (et, h, d) in &node.events {
            self.event_bindings.push(EventBinding { event_type: et.clone(), handler: h.clone(), data: d.clone(), bounds: logical_bounds });
        }
    }

    fn to_px(&self, v: &StyleValue) -> Option<f32> {
        match v {
            StyleValue::Length(n, u) => Some(match u {
                LengthUnit::Px => *n,
                LengthUnit::Rpx => rpx_to_px(*n, self.screen_width),
                LengthUnit::Percent => *n / 100.0 * self.screen_width,
                LengthUnit::Em | LengthUnit::Rem => *n * 16.0,
                LengthUnit::Vw => *n / 100.0 * self.screen_width,
                LengthUnit::Vh => *n / 100.0 * self.screen_height,
            }),
            StyleValue::Number(n) => Some(*n),
            _ => None,
        }
    }
    
    fn to_dimension(&self, v: &StyleValue) -> Option<Dimension> {
        match v {
            StyleValue::Auto => Some(Dimension::Auto),
            StyleValue::Length(n, LengthUnit::Percent) => Some(percent(*n / 100.0)),
            _ => self.to_px(v).map(|px| length(px * self.scale_factor)),
        }
    }

    fn draw_text(&self, canvas: &mut Canvas, text: &str, x: f32, y: f32, size: f32, color: Color) {
        if let Some(ref tr) = self.text_renderer {
            tr.draw_text(canvas, text, x, y + size, size, &Paint::new().with_color(color).with_style(PaintStyle::Fill));
        }
    }

    fn measure_text(&self, text: &str, size: f32) -> f32 {
        self.text_renderer.as_ref().map(|tr| tr.measure_text(text, size)).unwrap_or(text.chars().count() as f32 * size * 0.6)
    }

    fn get_classes<'a>(&self, node: &'a WxmlNode) -> Vec<&'a str> {
        node.get_attr("class").map(|s| s.split_whitespace().collect()).unwrap_or_default()
    }

    fn get_text(&self, node: &WxmlNode) -> String {
        let mut s = String::new();
        for c in &node.children {
            if c.node_type == WxmlNodeType::Text { s.push_str(&c.text_content); }
            else { s.push_str(&self.get_text(c)); }
        }
        s.trim().into()
    }

    pub fn get_event_bindings(&self) -> &[EventBinding] { &self.event_bindings }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<&EventBinding> {
        self.event_bindings.iter().rev().find(|b| b.bounds.contains(&crate::Point::new(x, y)))
    }
}
