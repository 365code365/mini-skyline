//! 样式解析器 - 将 WXSS 样式应用到组件

use crate::parser::wxss::{StyleSheet, StyleValue, LengthUnit, rpx_to_px};
use crate::ui::Style;
use crate::Color;
use std::collections::HashMap;

/// 样式解析器
pub struct StyleResolver {
    stylesheet: StyleSheet,
    screen_width: f32,
}

impl StyleResolver {
    pub fn new(stylesheet: StyleSheet, screen_width: f32) -> Self {
        Self { stylesheet, screen_width }
    }
    
    /// 解析元素样式
    pub fn resolve(&self, class_names: &[&str], tag_name: &str, inline_style: Option<&str>) -> Style {
        let mut style = Style::default();
        
        // 从样式表获取样式
        let css_styles = self.stylesheet.get_styles(class_names, tag_name);
        self.apply_styles(&mut style, &css_styles);
        
        // 应用内联样式
        if let Some(inline) = inline_style {
            let inline_styles = self.parse_inline_style(inline);
            self.apply_styles(&mut style, &inline_styles);
        }
        
        style
    }
    
    fn apply_styles(&self, style: &mut Style, properties: &HashMap<String, StyleValue>) {
        for (name, value) in properties {
            match name.as_str() {
                "width" => {
                    if let Some(px) = self.to_px(value) {
                        style.width = px;
                    }
                }
                "height" => {
                    if let Some(px) = self.to_px(value) {
                        style.height = px;
                    }
                }
                "padding" => {
                    if let Some(px) = self.to_px(value) {
                        style.padding = [px, px, px, px];
                    }
                }
                "padding-top" => {
                    if let Some(px) = self.to_px(value) {
                        style.padding[0] = px;
                    }
                }
                "padding-right" => {
                    if let Some(px) = self.to_px(value) {
                        style.padding[1] = px;
                    }
                }
                "padding-bottom" => {
                    if let Some(px) = self.to_px(value) {
                        style.padding[2] = px;
                    }
                }
                "padding-left" => {
                    if let Some(px) = self.to_px(value) {
                        style.padding[3] = px;
                    }
                }
                "margin" => {
                    if let Some(px) = self.to_px(value) {
                        style.margin = [px, px, px, px];
                    }
                }
                "margin-top" => {
                    if let Some(px) = self.to_px(value) {
                        style.margin[0] = px;
                    }
                }
                "margin-right" => {
                    if let Some(px) = self.to_px(value) {
                        style.margin[1] = px;
                    }
                }
                "margin-bottom" => {
                    if let Some(px) = self.to_px(value) {
                        style.margin[2] = px;
                    }
                }
                "margin-left" => {
                    if let Some(px) = self.to_px(value) {
                        style.margin[3] = px;
                    }
                }
                "background-color" | "background" => {
                    if let StyleValue::Color(color) = value {
                        style.background_color = Some(*color);
                    }
                }
                "color" => {
                    if let StyleValue::Color(color) = value {
                        style.text_color = Some(*color);
                    }
                }
                "border-color" => {
                    if let StyleValue::Color(color) = value {
                        style.border_color = Some(*color);
                    }
                }
                "border-width" => {
                    if let Some(px) = self.to_px(value) {
                        style.border_width = px;
                    }
                }
                "border-radius" => {
                    if let Some(px) = self.to_px(value) {
                        style.border_radius = px;
                    }
                }
                "opacity" => {
                    if let StyleValue::Number(n) = value {
                        style.opacity = *n;
                    }
                }
                
                // Flex 属性
                "display" => {
                    if let StyleValue::String(s) = value {
                        style.display_flex = s == "flex";
                    }
                }
                "flex-direction" => {
                    if let StyleValue::String(s) = value {
                        style.flex_direction = Some(s.clone());
                    }
                }
                "flex-wrap" => {
                    if let StyleValue::String(s) = value {
                        style.flex_wrap = Some(s.clone());
                    }
                }
                "justify-content" => {
                    if let StyleValue::String(s) = value {
                        style.justify_content = Some(s.clone());
                    }
                }
                "align-items" => {
                    if let StyleValue::String(s) = value {
                        style.align_items = Some(s.clone());
                    }
                }
                "align-content" => {
                    if let StyleValue::String(s) = value {
                        style.align_content = Some(s.clone());
                    }
                }
                "gap" => {
                    if let Some(px) = self.to_px(value) {
                        style.gap = Some(px);
                    }
                }
                "flex-grow" => {
                    if let StyleValue::Number(n) = value {
                        style.flex_grow = *n;
                    }
                }
                "flex-shrink" => {
                    if let StyleValue::Number(n) = value {
                        style.flex_shrink = *n;
                    }
                }
                "flex-basis" => {
                    if let Some(px) = self.to_px(value) {
                        style.flex_basis = Some(px);
                    }
                }
                
                // 文本属性
                "font-size" => {
                    if let Some(px) = self.to_px(value) {
                        style.font_size = Some(px);
                    }
                }
                "font-weight" => {
                    if let StyleValue::String(s) = value {
                        style.font_weight = Some(s.clone());
                    }
                }
                "text-align" => {
                    if let StyleValue::String(s) = value {
                        style.text_align = Some(s.clone());
                    }
                }
                
                _ => {}
            }
        }
    }
    
    fn to_px(&self, value: &StyleValue) -> Option<f32> {
        match value {
            StyleValue::Length(num, unit) => {
                Some(match unit {
                    LengthUnit::Px => *num,
                    LengthUnit::Rpx => rpx_to_px(*num, self.screen_width),
                    LengthUnit::Percent => *num / 100.0 * self.screen_width,
                    LengthUnit::Em => *num * 16.0,
                    LengthUnit::Rem => *num * 16.0,
                })
            }
            StyleValue::Number(n) => Some(*n),
            _ => None,
        }
    }
    
    fn parse_inline_style(&self, style_str: &str) -> HashMap<String, StyleValue> {
        let mut styles = HashMap::new();
        
        for part in style_str.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            
            if let Some(colon_pos) = part.find(':') {
                let name = part[..colon_pos].trim().to_string();
                let value_str = part[colon_pos + 1..].trim();
                
                let value = self.parse_value(&name, value_str);
                styles.insert(name, value);
            }
        }
        
        styles
    }
    
    fn parse_value(&self, _name: &str, value: &str) -> StyleValue {
        let value = value.trim();
        
        // 颜色
        if value.starts_with('#') {
            if let Some(color) = self.parse_color(value) {
                return StyleValue::Color(color);
            }
        }
        
        // 长度
        if value.ends_with("rpx") {
            if let Ok(num) = value.trim_end_matches("rpx").parse::<f32>() {
                return StyleValue::Length(num, LengthUnit::Rpx);
            }
        }
        
        if value.ends_with("px") {
            if let Ok(num) = value.trim_end_matches("px").parse::<f32>() {
                return StyleValue::Length(num, LengthUnit::Px);
            }
        }
        
        // 数字
        if let Ok(num) = value.parse::<f32>() {
            return StyleValue::Number(num);
        }
        
        StyleValue::String(value.to_string())
    }
    
    fn parse_color(&self, value: &str) -> Option<Color> {
        let hex = value.trim_start_matches('#');
        
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
                Some(Color::new(r, g, b, 255))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Color::new(r, g, b, 255))
            }
            _ => None,
        }
    }
}
