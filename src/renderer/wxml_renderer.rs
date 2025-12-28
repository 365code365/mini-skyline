//! WXML 渲染器 - 使用 Taffy 进行 Flexbox 布局，支持高 DPI 和系统字体

use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::parser::wxss::{StyleSheet, StyleValue, LengthUnit, rpx_to_px};
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path};
use crate::geometry::Rect as GeoRect;
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
}

pub struct WxmlRenderer {
    stylesheet: StyleSheet,
    screen_width: f32,
    scale_factor: f32,
    event_bindings: Vec<EventBinding>,
    text_renderer: Option<TextRenderer>,
}

impl WxmlRenderer {
    pub fn new(stylesheet: StyleSheet, screen_width: f32, _h: f32) -> Self {
        Self::new_with_scale(stylesheet, screen_width, _h, 1.0)
    }
    
    pub fn new_with_scale(stylesheet: StyleSheet, screen_width: f32, _h: f32, scale_factor: f32) -> Self {
        // 优先加载系统字体
        let text_renderer = TextRenderer::load_system_font()
            .or_else(|_| {
                println!("⚠️ System font not found, using bundled font");
                TextRenderer::from_bytes(include_bytes!("../../assets/ArialUnicode.ttf"))
            })
            .ok();
        
        if text_renderer.is_some() { 
            println!("✅ Text renderer ready (scale: {}x)", scale_factor); 
        }
        
        Self { stylesheet, screen_width, scale_factor, event_bindings: Vec::new(), text_renderer }
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
                size: Size { width: length(self.screen_width), height: auto() },
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &child_ids,
        ).unwrap();
        
        taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
        
        for rn in &render_nodes {
            self.draw_node(canvas, &taffy, rn, 0.0, 0.0);
        }
    }

    fn build_tree(&self, taffy: &mut TaffyTree, node: &WxmlNode) -> Option<RenderNode> {
        if node.node_type == WxmlNodeType::Text {
            let text = node.text_content.trim();
            if text.is_empty() { return None; }
            let fs = 14.0;
            let tw = self.measure_text(text, fs);
            let tn = taffy.new_leaf(Style {
                size: Size { width: length(tw), height: length(fs + 4.0) },
                ..Default::default()
            }).unwrap();
            return Some(RenderNode {
                tag: "#text".into(), text: text.into(), taffy_node: tn,
                style: NodeStyle { font_size: fs, text_color: Some(Color::BLACK), ..Default::default() },
                children: vec![], events: vec![],
            });
        }
        if node.node_type != WxmlNodeType::Element { return None; }

        let classes = self.get_classes(node);
        let css = self.stylesheet.get_styles(&classes, &node.tag_name);
        let (mut ts, ns) = self.build_styles(&css, &node.tag_name);

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
            let tw = self.measure_text(&t, ns.font_size);
            ts.size = Size { width: length(tw + 32.0), height: length(ns.font_size + 16.0) };
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
        let mut ns = NodeStyle { font_size: 14.0, ..Default::default() };
        let mut ts = Style { display: Display::Flex, flex_direction: FlexDirection::Column, ..Default::default() };

        if tag == "button" {
            ns.background_color = Some(Color::from_hex(0x07C160));
            ns.text_color = Some(Color::WHITE);
            ns.border_radius = 4.0;
            ts.padding = taffy::Rect { top: length(8.0), right: length(16.0), bottom: length(8.0), left: length(16.0) };
        }

        for (name, value) in props {
            match name.as_str() {
                "padding" => if let Some(v) = self.to_px(value) {
                    ts.padding = taffy::Rect { top: length(v), right: length(v), bottom: length(v), left: length(v) };
                }
                "margin" => if let Some(v) = self.to_px(value) {
                    ts.margin = taffy::Rect { top: length(v), right: length(v), bottom: length(v), left: length(v) };
                }
                "margin-bottom" => if let Some(v) = self.to_px(value) { ts.margin.bottom = length(v); }
                "display" => if let StyleValue::String(s) = value { if s == "flex" { ts.display = Display::Flex; } }
                "flex-direction" => if let StyleValue::String(s) = value {
                    ts.flex_direction = if s == "row" { FlexDirection::Row } else { FlexDirection::Column };
                }
                "gap" => if let Some(v) = self.to_px(value) { ts.gap = Size { width: length(v), height: length(v) }; }
                "background-color" | "background" => if let StyleValue::Color(c) = value { ns.background_color = Some(*c); }
                "color" => if let StyleValue::Color(c) = value { ns.text_color = Some(*c); }
                "border-radius" => if let Some(v) = self.to_px(value) { ns.border_radius = v; }
                "font-size" => if let Some(v) = self.to_px(value) { ns.font_size = v; }
                _ => {}
            }
        }
        (ts, ns)
    }

    fn draw_node(&mut self, canvas: &mut Canvas, taffy: &TaffyTree, node: &RenderNode, ox: f32, oy: f32) {
        let layout = taffy.layout(node.taffy_node).unwrap();
        let lx = ox + layout.location.x;
        let ly = oy + layout.location.y;
        let lw = layout.size.width;
        let lh = layout.size.height;
        
        let s = self.scale_factor;
        let px = lx * s;
        let py = ly * s;
        let pw = lw * s;
        let ph = lh * s;
        let bounds = GeoRect::new(px, py, pw, ph);

        // 绘制背景
        if let Some(bg) = node.style.background_color {
            let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
            let pr = node.style.border_radius * s;
            if pr > 0.0 {
                let mut path = Path::new();
                path.add_round_rect(px, py, pw, ph, pr);
                canvas.draw_path(&path, &paint);
            } else {
                canvas.draw_rect(&bounds, &paint);
            }
        }

        // 绘制内容
        let font_size = node.style.font_size * s;
        match node.tag.as_str() {
            "#text" => {
                let c = node.style.text_color.unwrap_or(Color::BLACK);
                self.draw_text_scaled(canvas, &node.text, px, py + font_size, font_size, c);
            }
            "button" => {
                let c = node.style.text_color.unwrap_or(Color::WHITE);
                let tw = self.measure_text_scaled(&node.text, font_size);
                let tx = px + (pw - tw) / 2.0;
                let ty = py + (ph + font_size * 0.7) / 2.0;
                self.draw_text_scaled(canvas, &node.text, tx, ty, font_size, c);
            }
            _ => {
                for child in &node.children { self.draw_node(canvas, taffy, child, lx, ly); }
            }
        }

        // 注册事件
        for (et, handler, data) in &node.events {
            self.event_bindings.push(EventBinding {
                event_type: et.clone(),
                handler: handler.clone(),
                data: data.clone(),
                bounds: GeoRect::new(lx, ly, lw, lh),
            });
        }
    }

    fn to_px(&self, v: &StyleValue) -> Option<f32> {
        match v {
            StyleValue::Length(n, u) => Some(match u {
                LengthUnit::Px => *n,
                LengthUnit::Rpx => rpx_to_px(*n, self.screen_width),
                LengthUnit::Percent => *n / 100.0 * self.screen_width,
                LengthUnit::Em | LengthUnit::Rem => *n * 16.0,
            }),
            StyleValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    fn draw_text_scaled(&self, canvas: &mut Canvas, text: &str, x: f32, y: f32, size: f32, color: Color) {
        if let Some(ref tr) = self.text_renderer {
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            tr.draw_text(canvas, text, x, y, size, &paint);
        }
    }

    fn measure_text_scaled(&self, text: &str, size: f32) -> f32 {
        self.text_renderer.as_ref().map(|tr| tr.measure_text(text, size)).unwrap_or(text.chars().count() as f32 * size * 0.6)
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
