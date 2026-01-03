//! RichText 富文本组件

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Rect as GeoRect};

/// RichText 组件 - 支持简单的 HTML 标签渲染
pub struct RichTextComponent;

/// 富文本节点
#[derive(Clone, Debug)]
pub struct RichTextNode {
    pub tag: String,
    pub text: String,
    pub attrs: std::collections::HashMap<String, String>,
    pub children: Vec<RichTextNode>,
}

impl RichTextComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        // 获取 nodes 属性（JSON 格式的富文本节点）
        let nodes_str = node.get_attr("nodes").unwrap_or("");
        let text = Self::parse_nodes_to_text(nodes_str);
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "rich-text".into(),
            text,
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    /// 解析 nodes 属性为纯文本（简化实现）
    fn parse_nodes_to_text(nodes_str: &str) -> String {
        // 尝试解析 JSON
        if let Ok(nodes) = serde_json::from_str::<Vec<serde_json::Value>>(nodes_str) {
            let mut text = String::new();
            for node in nodes {
                Self::extract_text_from_node(&node, &mut text);
            }
            return text;
        }
        
        // 如果不是 JSON，可能是 HTML 字符串
        Self::strip_html_tags(nodes_str)
    }
    
    /// 从 JSON 节点提取文本
    fn extract_text_from_node(node: &serde_json::Value, text: &mut String) {
        if let Some(obj) = node.as_object() {
            // 检查是否是文本节点
            if let Some(t) = obj.get("text").and_then(|v| v.as_str()) {
                text.push_str(t);
            }
            // 递归处理子节点
            if let Some(children) = obj.get("children").and_then(|v| v.as_array()) {
                for child in children {
                    Self::extract_text_from_node(child, text);
                }
            }
        } else if let Some(s) = node.as_str() {
            text.push_str(s);
        }
    }
    
    /// 去除 HTML 标签
    fn strip_html_tags(html: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;
        
        for c in html.chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => result.push(c),
                _ => {}
            }
        }
        
        // 处理 HTML 实体
        result
            .replace("&nbsp;", " ")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
    }
    
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, text_renderer: Option<&TextRenderer>, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        // 绘制背景
        draw_background(canvas, &node.style, x, y, w, h);
        
        // 绘制文本
        if !node.text.is_empty() {
            let font_size = node.style.font_size * sf;
            let text_color = node.style.text_color.unwrap_or(Color::BLACK);
            let padding = 4.0 * sf;
            
            if let Some(tr) = text_renderer {
                let text_x = x + padding;
                let text_y = y + padding + font_size;
                let paint = Paint::new().with_color(text_color);
                tr.draw_text(canvas, &node.text, text_x, text_y, font_size, &paint);
            }
        }
        
        // 绘制边框
        if node.style.border_width > 0.0 {
            let border_color = node.style.border_color.unwrap_or(Color::from_hex(0xE5E5E5));
            let paint = Paint::new()
                .with_color(border_color)
                .with_style(PaintStyle::Stroke)
                .with_stroke_width(node.style.border_width);
            canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
        }
    }
}
