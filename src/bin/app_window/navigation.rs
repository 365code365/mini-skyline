//! 页面导航逻辑

use std::collections::HashMap;
use mini_render::parser::wxml::WxmlNode;
use mini_render::parser::wxss::StyleSheet;

/// 页面信息
pub struct PageInfo {
    pub path: String,
    pub wxml: String,
    pub wxss: String,
    pub js: String,
}

/// 页面栈中的页面实例
pub struct PageInstance {
    pub path: String,
    pub query: HashMap<String, String>,
    pub wxml_nodes: Vec<WxmlNode>,
    pub stylesheet: StyleSheet,
}

/// 导航请求类型
#[derive(Clone)]
pub enum NavigationRequest {
    NavigateTo { url: String },
    NavigateBack,
    SwitchTab { url: String },
}

/// 解析 URL，返回路径和查询参数
pub fn parse_url(url: &str) -> (String, HashMap<String, String>) {
    let url = url.trim_start_matches('/');
    let mut query = HashMap::new();
    let (path, query_str) = if let Some(pos) = url.find('?') {
        (&url[..pos], Some(&url[pos+1..]))
    } else {
        (url, None)
    };
    if let Some(qs) = query_str {
        for pair in qs.split('&') {
            if let Some(eq_pos) = pair.find('=') {
                let key = &pair[..eq_pos];
                let value = &pair[eq_pos+1..];
                query.insert(key.to_string(), value.to_string());
            }
        }
    }
    (path.to_string(), query)
}

/// 移除 WXML 中手动写的 tabbar 元素
pub fn remove_manual_tabbar(nodes: &[WxmlNode]) -> Vec<WxmlNode> {
    use mini_render::parser::wxml::WxmlNodeType;
    
    fn filter_node(node: &WxmlNode) -> Option<WxmlNode> {
        if node.node_type != WxmlNodeType::Element {
            return Some(node.clone());
        }
        
        let class = node.attributes.get("class").map(|s| s.as_str()).unwrap_or("");
        if class.contains("tabbar") {
            return None;
        }
        
        let mut new_node = WxmlNode::new_element(&node.tag_name);
        new_node.attributes = node.attributes.clone();
        new_node.children = node.children.iter()
            .filter_map(|c| filter_node(c))
            .collect();
        Some(new_node)
    }
    
    nodes.iter().filter_map(|n| filter_node(n)).collect()
}
