//! 模板引擎 - 处理数据绑定和条件渲染

use super::wxml::WxmlNode;
use std::collections::HashMap;
use serde_json::Value as JsonValue;

/// 模板引擎
pub struct TemplateEngine;

impl TemplateEngine {
    /// 渲染模板，替换 {{}} 表达式
    pub fn render(nodes: &[WxmlNode], data: &JsonValue) -> Vec<WxmlNode> {
        let mut result = Vec::new();
        
        for node in nodes {
            if let Some(rendered) = Self::render_node(node, data) {
                result.extend(rendered);
            }
        }
        
        result
    }
    
    fn render_node(node: &WxmlNode, data: &JsonValue) -> Option<Vec<WxmlNode>> {
        match node.node_type {
            super::wxml::WxmlNodeType::Text => {
                let text = Self::interpolate(&node.text_content, data);
                Some(vec![WxmlNode::new_text(&text)])
            }
            super::wxml::WxmlNodeType::Element => {
                // 处理 wx:if
                if let Some(condition) = node.attributes.get("wx:if") {
                    let expr = Self::extract_expression(condition);
                    if !Self::evaluate_condition(&expr, data) {
                        return None;
                    }
                }
                
                // 处理 wx:for
                if let Some(for_expr) = node.attributes.get("wx:for") {
                    return Some(Self::render_for_loop(node, for_expr, data));
                }
                
                // 普通元素
                let mut new_node = WxmlNode::new_element(&node.tag_name);
                
                // 处理属性
                for (key, value) in &node.attributes {
                    if key.starts_with("wx:") {
                        continue;
                    }
                    let new_value = Self::interpolate(value, data);
                    new_node.attributes.insert(key.clone(), new_value);
                }
                
                // 处理子节点
                new_node.children = Self::render(&node.children, data);
                
                Some(vec![new_node])
            }
            _ => Some(vec![node.clone()]),
        }
    }
    
    fn render_for_loop(node: &WxmlNode, for_expr: &str, data: &JsonValue) -> Vec<WxmlNode> {
        let mut result = Vec::new();
        
        let array_name = Self::extract_expression(for_expr);
        let item_name = node.attributes.get("wx:for-item")
            .map(|s| s.as_str())
            .unwrap_or("item");
        let index_name = node.attributes.get("wx:for-index")
            .map(|s| s.as_str())
            .unwrap_or("index");
        
        if let Some(array) = Self::get_value(&array_name, data) {
            if let Some(arr) = array.as_array() {
                for (index, item) in arr.iter().enumerate() {
                    // 创建循环上下文
                    let mut loop_data = data.clone();
                    if let Some(obj) = loop_data.as_object_mut() {
                        obj.insert(item_name.to_string(), item.clone());
                        obj.insert(index_name.to_string(), JsonValue::Number(index.into()));
                    }
                    
                    // 渲染节点（不包含 wx:for 属性）
                    let mut new_node = WxmlNode::new_element(&node.tag_name);
                    
                    for (key, value) in &node.attributes {
                        if key.starts_with("wx:") {
                            continue;
                        }
                        let new_value = Self::interpolate(value, &loop_data);
                        new_node.attributes.insert(key.clone(), new_value);
                    }
                    
                    new_node.children = Self::render(&node.children, &loop_data);
                    result.push(new_node);
                }
            }
        }
        
        result
    }
    
    /// 插值替换 {{expression}}
    fn interpolate(template: &str, data: &JsonValue) -> String {
        let mut result = template.to_string();
        let mut start = 0;
        
        while let Some(open) = result[start..].find("{{") {
            let open = start + open;
            if let Some(close) = result[open..].find("}}") {
                let close = open + close;
                let expr = &result[open + 2..close].trim();
                let value = Self::evaluate_expression(expr, data);
                result = format!("{}{}{}", &result[..open], value, &result[close + 2..]);
                start = open + value.len();
            } else {
                break;
            }
        }
        
        result
    }
    
    /// 提取 {{}} 中的表达式
    fn extract_expression(s: &str) -> String {
        let s = s.trim();
        if s.starts_with("{{") && s.ends_with("}}") {
            s[2..s.len() - 2].trim().to_string()
        } else {
            s.to_string()
        }
    }
    
    /// 计算表达式
    fn evaluate_expression(expr: &str, data: &JsonValue) -> String {
        let expr = expr.trim();
        
        // 三元表达式: condition ? true_val : false_val
        if let Some(q_pos) = expr.find('?') {
            if let Some(c_pos) = expr[q_pos..].find(':') {
                let condition = &expr[..q_pos].trim();
                let true_val = &expr[q_pos + 1..q_pos + c_pos].trim();
                let false_val = &expr[q_pos + c_pos + 1..].trim();
                
                if Self::evaluate_condition(condition, data) {
                    return Self::evaluate_expression(true_val, data);
                } else {
                    return Self::evaluate_expression(false_val, data);
                }
            }
        }
        
        // 字符串字面量
        if (expr.starts_with('\'') && expr.ends_with('\'')) ||
           (expr.starts_with('"') && expr.ends_with('"')) {
            return expr[1..expr.len() - 1].to_string();
        }
        
        // 数字字面量
        if let Ok(_) = expr.parse::<f64>() {
            return expr.to_string();
        }
        
        // 变量访问
        if let Some(value) = Self::get_value(expr, data) {
            return Self::json_to_string(value);
        }
        
        expr.to_string()
    }
    
    /// 计算条件表达式
    fn evaluate_condition(expr: &str, data: &JsonValue) -> bool {
        let expr = expr.trim();
        
        // 否定
        if expr.starts_with('!') {
            return !Self::evaluate_condition(&expr[1..], data);
        }
        
        // 比较运算
        for op in &["===", "!==", "==", "!=", ">=", "<=", ">", "<"] {
            if let Some(pos) = expr.find(op) {
                let left = Self::evaluate_expression(&expr[..pos], data);
                let right = Self::evaluate_expression(&expr[pos + op.len()..], data);
                
                return match *op {
                    "===" | "==" => left == right,
                    "!==" | "!=" => left != right,
                    ">" => left.parse::<f64>().unwrap_or(0.0) > right.parse::<f64>().unwrap_or(0.0),
                    "<" => left.parse::<f64>().unwrap_or(0.0) < right.parse::<f64>().unwrap_or(0.0),
                    ">=" => left.parse::<f64>().unwrap_or(0.0) >= right.parse::<f64>().unwrap_or(0.0),
                    "<=" => left.parse::<f64>().unwrap_or(0.0) <= right.parse::<f64>().unwrap_or(0.0),
                    _ => false,
                };
            }
        }
        
        // 布尔值
        if let Some(value) = Self::get_value(expr, data) {
            return Self::is_truthy(value);
        }
        
        !expr.is_empty() && expr != "false" && expr != "0"
    }
    
    /// 获取数据值
    fn get_value<'a>(path: &str, data: &'a JsonValue) -> Option<&'a JsonValue> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = data;
        
        for part in parts {
            // 处理数组索引 item[0]
            if let Some(bracket_pos) = part.find('[') {
                let name = &part[..bracket_pos];
                let index_str = &part[bracket_pos + 1..part.len() - 1];
                
                if !name.is_empty() {
                    current = current.get(name)?;
                }
                
                if let Ok(index) = index_str.parse::<usize>() {
                    current = current.get(index)?;
                }
            } else {
                current = current.get(part)?;
            }
        }
        
        Some(current)
    }
    
    fn json_to_string(value: &JsonValue) -> String {
        match value {
            JsonValue::String(s) => s.clone(),
            JsonValue::Number(n) => n.to_string(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Null => "".to_string(),
            _ => value.to_string(),
        }
    }
    
    fn is_truthy(value: &JsonValue) -> bool {
        match value {
            JsonValue::Null => false,
            JsonValue::Bool(b) => *b,
            JsonValue::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
            JsonValue::String(s) => !s.is_empty(),
            JsonValue::Array(a) => !a.is_empty(),
            JsonValue::Object(_) => true,
        }
    }
}
