//! WXSS 解析器 - 完整支持微信小程序样式

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
    Rpx,
    Percent,
    Em,
    Rem,
    Vw,
    Vh,
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
    
    /// 获取元素的样式（支持多选择器匹配和优先级）
    pub fn get_styles(&self, class_names: &[&str], tag_name: &str) -> HashMap<String, StyleValue> {
        let mut styles = HashMap::new();
        
        // 按选择器特异性排序应用
        let mut matched_rules: Vec<(u32, &StyleRule)> = Vec::new();
        
        for rule in &self.rules {
            if let Some(specificity) = self.selector_matches(&rule.selector, class_names, tag_name) {
                matched_rules.push((specificity, rule));
            }
        }
        
        // 按特异性排序（低到高）
        matched_rules.sort_by_key(|(s, _)| *s);
        
        for (_, rule) in matched_rules {
            for (key, value) in &rule.properties {
                styles.insert(key.clone(), value.clone());
            }
        }
        
        styles
    }
    
    /// 返回 Some(specificity) 如果匹配，None 如果不匹配
    fn selector_matches(&self, selector: &str, class_names: &[&str], tag_name: &str) -> Option<u32> {
        let selector = selector.trim();
        
        // 处理多选择器（逗号分隔）
        if selector.contains(',') {
            for part in selector.split(',') {
                if let Some(s) = self.single_selector_matches(part.trim(), class_names, tag_name) {
                    return Some(s);
                }
            }
            return None;
        }
        
        self.single_selector_matches(selector, class_names, tag_name)
    }
    
    fn single_selector_matches(&self, selector: &str, class_names: &[&str], tag_name: &str) -> Option<u32> {
        // 类选择器 .class
        if selector.starts_with('.') && !selector[1..].contains('.') {
            let class = &selector[1..];
            if class_names.contains(&class) {
                return Some(10); // 类选择器特异性
            }
            return None;
        }
        
        // 标签选择器
        if !selector.contains('.') && !selector.contains('#') {
            if selector == tag_name {
                return Some(1); // 标签选择器特异性
            }
            return None;
        }
        
        // 复合选择器 tag.class 或 .class1.class2
        let mut specificity = 0u32;
        let mut all_match = true;
        
        let parts: Vec<&str> = selector.split('.').collect();
        
        // 第一部分可能是标签
        if !parts[0].is_empty() {
            if parts[0] == tag_name {
                specificity += 1;
            } else {
                all_match = false;
            }
        }
        
        // 后续部分是类
        for class in &parts[1..] {
            if !class.is_empty() {
                if class_names.contains(class) {
                    specificity += 10;
                } else {
                    all_match = false;
                    break;
                }
            }
        }
        
        if all_match && specificity > 0 {
            Some(specificity)
        } else {
            None
        }
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
    
    /// 解析单个长度值（用于 border 简写等）
    pub fn parse_length_value(&mut self) -> Option<(f32, LengthUnit)> {
        let value: String = self.input.iter().collect();
        Self::parse_length(&value)
    }
    
    pub fn parse(&mut self) -> Result<StyleSheet, String> {
        let mut stylesheet = StyleSheet::new();
        
        while self.pos < self.input.len() {
            self.skip_whitespace_and_comments();
            
            if self.pos >= self.input.len() {
                break;
            }
            
            // 跳过 @import 等 at-rules
            if self.current_char() == '@' {
                self.skip_at_rule();
                continue;
            }
            
            if let Some(rule) = self.parse_rule()? {
                stylesheet.rules.push(rule);
            }
        }
        
        Ok(stylesheet)
    }
    
    fn skip_at_rule(&mut self) {
        while self.pos < self.input.len() && self.current_char() != ';' && self.current_char() != '{' {
            self.advance();
        }
        if self.current_char() == '{' {
            let mut depth = 1;
            self.advance();
            while self.pos < self.input.len() && depth > 0 {
                if self.current_char() == '{' { depth += 1; }
                if self.current_char() == '}' { depth -= 1; }
                self.advance();
            }
        } else {
            self.advance();
        }
    }
    
    fn parse_rule(&mut self) -> Result<Option<StyleRule>, String> {
        self.skip_whitespace_and_comments();
        
        let selector = self.parse_selector();
        if selector.is_empty() {
            return Ok(None);
        }
        
        self.skip_whitespace_and_comments();
        
        if self.current_char() != '{' {
            return Err(format!("Expected '{{' after selector '{}'", selector));
        }
        self.advance();
        
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
            
            if c == '(' { paren_depth += 1; }
            else if c == ')' { paren_depth -= 1; }
            
            if paren_depth == 0 && (c == ';' || c == '}') {
                break;
            }
            
            value.push(c);
            self.advance();
        }
        
        value.trim().to_string()
    }
    
    fn parse_value(_name: &str, value: &str) -> StyleValue {
        let value = value.trim();
        
        // 颜色值
        if value.starts_with('#') {
            if let Some(color) = Self::parse_color(value) {
                return StyleValue::Color(color);
            }
        }
        
        if value.starts_with("rgb") {
            if let Some(color) = Self::parse_rgb_color(value) {
                return StyleValue::Color(color);
            }
        }
        
        // 渐变值 - 提取第一个颜色作为 fallback
        if value.starts_with("linear-gradient") || value.starts_with("radial-gradient") {
            if let Some(color) = Self::parse_gradient_fallback(value) {
                return StyleValue::Color(color);
            }
        }
        
        // 命名颜色
        if let Some(color) = Self::parse_named_color(value) {
            return StyleValue::Color(color);
        }
        
        // 长度值
        if let Some((num, unit)) = Self::parse_length(value) {
            return StyleValue::Length(num, unit);
        }
        
        // 特殊值
        match value {
            "auto" => return StyleValue::Auto,
            "none" => return StyleValue::None,
            "inherit" | "initial" | "unset" => return StyleValue::String(value.to_string()),
            _ => {}
        }
        
        // 数字
        if let Ok(num) = value.parse::<f32>() {
            return StyleValue::Number(num);
        }
        
        StyleValue::String(value.to_string())
    }
    
    fn parse_named_color(name: &str) -> Option<Color> {
        let color = match name.to_lowercase().as_str() {
            "transparent" => Color::new(0, 0, 0, 0),
            "black" => Color::BLACK,
            "white" => Color::WHITE,
            "red" => Color::new(255, 0, 0, 255),
            "green" => Color::new(0, 128, 0, 255),
            "blue" => Color::new(0, 0, 255, 255),
            "yellow" => Color::new(255, 255, 0, 255),
            "orange" => Color::new(255, 165, 0, 255),
            "purple" => Color::new(128, 0, 128, 255),
            "pink" => Color::new(255, 192, 203, 255),
            "gray" | "grey" => Color::new(128, 128, 128, 255),
            "lightgray" | "lightgrey" => Color::new(211, 211, 211, 255),
            "darkgray" | "darkgrey" => Color::new(169, 169, 169, 255),
            "cyan" => Color::new(0, 255, 255, 255),
            "magenta" => Color::new(255, 0, 255, 255),
            "brown" => Color::new(165, 42, 42, 255),
            "navy" => Color::new(0, 0, 128, 255),
            "teal" => Color::new(0, 128, 128, 255),
            "olive" => Color::new(128, 128, 0, 255),
            "maroon" => Color::new(128, 0, 0, 255),
            "silver" => Color::new(192, 192, 192, 255),
            "lime" => Color::new(0, 255, 0, 255),
            "aqua" => Color::new(0, 255, 255, 255),
            "fuchsia" => Color::new(255, 0, 255, 255),
            _ => return None,
        };
        Some(color)
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
    
    /// 解析渐变值，提取第一个颜色作为 fallback
    /// 支持格式：linear-gradient(135deg, #ff6b35 0%, #ff8f5a 100%)
    fn parse_gradient_fallback(value: &str) -> Option<Color> {
        // 找到括号内的内容
        let start = value.find('(')?;
        let end = value.rfind(')')?;
        let inner = &value[start + 1..end];
        
        // 按逗号分割，跳过角度/方向参数
        for part in inner.split(',') {
            let part = part.trim();
            
            // 跳过角度（如 135deg, 180deg）
            if part.ends_with("deg") || part.starts_with("to ") {
                continue;
            }
            
            // 尝试提取颜色（可能带有百分比位置）
            // 例如：#ff6b35 0% 或 #fff5f0 或 rgb(255, 107, 53)
            let color_part = part.split_whitespace().next()?;
            
            // 尝试解析为颜色
            if color_part.starts_with('#') {
                if let Some(color) = Self::parse_color(color_part) {
                    return Some(color);
                }
            } else if color_part.starts_with("rgb") {
                // 需要找到完整的 rgb/rgba 表达式
                if let Some(rgb_end) = part.find(')') {
                    let rgb_part = &part[..rgb_end + 1];
                    if let Some(color) = Self::parse_rgb_color(rgb_part) {
                        return Some(color);
                    }
                }
            } else if let Some(color) = Self::parse_named_color(color_part) {
                return Some(color);
            }
        }
        
        None
    }
    
    fn parse_length(value: &str) -> Option<(f32, LengthUnit)> {
        let value = value.trim();
        
        let units = [
            ("rpx", LengthUnit::Rpx),
            ("px", LengthUnit::Px),
            ("vw", LengthUnit::Vw),
            ("vh", LengthUnit::Vh),
            ("rem", LengthUnit::Rem),
            ("em", LengthUnit::Em),
            ("%", LengthUnit::Percent),
        ];
        
        for (suffix, unit) in units {
            if value.ends_with(suffix) {
                let num = value.trim_end_matches(suffix).parse().ok()?;
                return Some((num, unit));
            }
        }
        
        // 纯数字默认为 px
        if let Ok(num) = value.parse::<f32>() {
            return Some((num, LengthUnit::Px));
        }
        
        None
    }
    
    fn current_char(&self) -> char {
        self.input.get(self.pos).copied().unwrap_or('\0')
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

/// rpx 转 px (基于 750 设计稿)
pub fn rpx_to_px(rpx: f32, screen_width: f32) -> f32 {
    rpx * screen_width / 750.0
}
