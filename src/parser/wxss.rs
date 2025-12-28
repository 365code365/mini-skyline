//! WXSS 解析器

use std::collections::HashMap;
use crate::Color;

/// 样式值
#[derive(Debug, Clone)]
pub enum StyleValue {
    Length(f32, LengthUnit),
    Color(Color),
    String(String),
    Number(f32),
    Auto,
    None,
}

/// 长度单位
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LengthUnit {
    Px,
    Rpx,  // 小程序响应式像素
    Percent,
    Em,
    Rem,
}

/// 样式规则
#[derive(Debug, Clone)]
pub struct StyleRule {
    pub selector: String,
    pub properties: HashMap<String, StyleValue>,
}

/// 样式表
#[derive(Debug, Clone, Default)]
pub struct StyleSheet {
    pub rules: Vec<StyleRule>,
}

impl StyleSheet {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }
    
    /// 获取元素的样式
    pub fn get_styles(&self, class_names: &[&str], tag_name: &str) -> HashMap<String, StyleValue> {
        let mut styles = HashMap::new();
        
        for rule in &self.rules {
            let matches = self.selector_matches(&rule.selector, class_names, tag_name);
            if matches {
                for (key, value) in &rule.properties {
                    styles.insert(key.clone(), value.clone());
                }
            }
        }
        
        styles
    }
    
    fn selector_matches(&self, selector: &str, class_names: &[&str], tag_name: &str) -> bool {
        let selector = selector.trim();
        
        // 类选择器
        if selector.starts_with('.') {
            let class = &selector[1..];
            return class_names.contains(&class);
        }
        
        // 标签选择器
        if selector == tag_name {
            return true;
        }
        
        // 复合选择器 (简化处理)
        if selector.contains('.') {
            let parts: Vec<&str> = selector.split('.').collect();
            if parts.len() >= 2 {
                let tag = parts[0];
                let class = parts[1];
                return (tag.is_empty() || tag == tag_name) && class_names.contains(&class);
            }
        }
        
        false
    }
}

/// WXSS 解析器
pub struct WxssParser {
    input: Vec<char>,
    pos: usize,
}

impl WxssParser {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }
    
    pub fn parse(&mut self) -> Result<StyleSheet, String> {
        let mut stylesheet = StyleSheet::new();
        
        while self.pos < self.input.len() {
            self.skip_whitespace_and_comments();
            
            if self.pos >= self.input.len() {
                break;
            }
            
            if let Some(rule) = self.parse_rule()? {
                stylesheet.rules.push(rule);
            }
        }
        
        Ok(stylesheet)
    }
    
    fn parse_rule(&mut self) -> Result<Option<StyleRule>, String> {
        self.skip_whitespace_and_comments();
        
        // 解析选择器
        let selector = self.parse_selector();
        if selector.is_empty() {
            return Ok(None);
        }
        
        self.skip_whitespace_and_comments();
        
        if self.current_char() != '{' {
            return Err(format!("Expected '{{' after selector '{}'", selector));
        }
        self.advance();
        
        // 解析属性
        let properties = self.parse_properties()?;
        
        self.skip_whitespace_and_comments();
        if self.current_char() == '}' {
            self.advance();
        }
        
        Ok(Some(StyleRule { selector, properties }))
    }
    
    fn parse_selector(&mut self) -> String {
        let mut selector = String::new();
        
        while self.pos < self.input.len() {
            let c = self.current_char();
            if c == '{' || c == '}' {
                break;
            }
            selector.push(c);
            self.advance();
        }
        
        selector.trim().to_string()
    }
    
    fn parse_properties(&mut self) -> Result<HashMap<String, StyleValue>, String> {
        let mut properties = HashMap::new();
        
        loop {
            self.skip_whitespace_and_comments();
            
            if self.pos >= self.input.len() || self.current_char() == '}' {
                break;
            }
            
            let name = self.parse_property_name();
            if name.is_empty() {
                break;
            }
            
            self.skip_whitespace();
            
            if self.current_char() != ':' {
                continue;
            }
            self.advance();
            
            self.skip_whitespace();
            
            let value = self.parse_property_value();
            
            if self.current_char() == ';' {
                self.advance();
            }
            
            let parsed_value = Self::parse_value(&name, &value);
            properties.insert(name, parsed_value);
        }
        
        Ok(properties)
    }
    
    fn parse_property_name(&mut self) -> String {
        let mut name = String::new();
        
        while self.pos < self.input.len() {
            let c = self.current_char();
            if c.is_alphanumeric() || c == '-' || c == '_' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        
        name
    }
    
    fn parse_property_value(&mut self) -> String {
        let mut value = String::new();
        let mut paren_depth = 0;
        
        while self.pos < self.input.len() {
            let c = self.current_char();
            
            if c == '(' {
                paren_depth += 1;
            } else if c == ')' {
                paren_depth -= 1;
            }
            
            if paren_depth == 0 && (c == ';' || c == '}') {
                break;
            }
            
            value.push(c);
            self.advance();
        }
        
        value.trim().to_string()
    }
    
    fn parse_value(name: &str, value: &str) -> StyleValue {
        let value = value.trim();
        
        // 颜色值
        if value.starts_with('#') {
            if let Some(color) = Self::parse_color(value) {
                return StyleValue::Color(color);
            }
        }
        
        if value.starts_with("rgb") || value.starts_with("rgba") {
            if let Some(color) = Self::parse_rgb_color(value) {
                return StyleValue::Color(color);
            }
        }
        
        // 长度值
        if let Some((num, unit)) = Self::parse_length(value) {
            return StyleValue::Length(num, unit);
        }
        
        // 特殊值
        match value {
            "auto" => return StyleValue::Auto,
            "none" => return StyleValue::None,
            _ => {}
        }
        
        // 数字
        if let Ok(num) = value.parse::<f32>() {
            return StyleValue::Number(num);
        }
        
        StyleValue::String(value.to_string())
    }
    
    fn parse_color(value: &str) -> Option<Color> {
        let hex = value.trim_start_matches('#');
        
        let (r, g, b, a) = match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
                (r, g, b, 255)
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                (r, g, b, 255)
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                (r, g, b, a)
            }
            _ => return None,
        };
        
        Some(Color::new(r, g, b, a))
    }
    
    fn parse_rgb_color(value: &str) -> Option<Color> {
        let inner = value
            .trim_start_matches("rgba(")
            .trim_start_matches("rgb(")
            .trim_end_matches(')');
        
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() < 3 {
            return None;
        }
        
        let r = parts[0].trim().parse::<u8>().ok()?;
        let g = parts[1].trim().parse::<u8>().ok()?;
        let b = parts[2].trim().parse::<u8>().ok()?;
        let a = if parts.len() > 3 {
            (parts[3].trim().parse::<f32>().ok()? * 255.0) as u8
        } else {
            255
        };
        
        Some(Color::new(r, g, b, a))
    }
    
    fn parse_length(value: &str) -> Option<(f32, LengthUnit)> {
        let value = value.trim();
        
        if value.ends_with("rpx") {
            let num = value.trim_end_matches("rpx").parse().ok()?;
            return Some((num, LengthUnit::Rpx));
        }
        
        if value.ends_with("px") {
            let num = value.trim_end_matches("px").parse().ok()?;
            return Some((num, LengthUnit::Px));
        }
        
        if value.ends_with('%') {
            let num = value.trim_end_matches('%').parse().ok()?;
            return Some((num, LengthUnit::Percent));
        }
        
        if value.ends_with("em") {
            let num = value.trim_end_matches("em").parse().ok()?;
            return Some((num, LengthUnit::Em));
        }
        
        if value.ends_with("rem") {
            let num = value.trim_end_matches("rem").parse().ok()?;
            return Some((num, LengthUnit::Rem));
        }
        
        // 纯数字默认为 px
        if let Ok(num) = value.parse::<f32>() {
            return Some((num, LengthUnit::Px));
        }
        
        None
    }
    
    fn current_char(&self) -> char {
        if self.pos < self.input.len() {
            self.input[self.pos]
        } else {
            '\0'
        }
    }
    
    fn advance(&mut self) {
        self.pos += 1;
    }
    
    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.current_char().is_whitespace() {
            self.advance();
        }
    }
    
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            self.skip_whitespace();
            
            if self.starts_with("/*") {
                self.skip_comment();
            } else {
                break;
            }
        }
    }
    
    fn skip_comment(&mut self) {
        self.advance();
        self.advance();
        
        while self.pos < self.input.len() && !self.starts_with("*/") {
            self.advance();
        }
        
        if self.pos < self.input.len() {
            self.advance();
            self.advance();
        }
    }
    
    fn starts_with(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            if self.pos + i >= self.input.len() || self.input[self.pos + i] != *c {
                return false;
            }
        }
        true
    }
}

/// rpx 转 px (基于 375 设计稿)
pub fn rpx_to_px(rpx: f32, screen_width: f32) -> f32 {
    rpx * screen_width / 750.0
}
