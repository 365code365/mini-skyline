//! 页面加载模块

use std::collections::HashMap;
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::parser::wxml::WxmlNode;
use mini_render::parser::wxss::StyleSheet;
use super::navigation::PageInfo;

/// 自定义 TabBar 数据
pub struct CustomTabBar {
    pub wxml_nodes: Vec<WxmlNode>,
    pub stylesheet: StyleSheet,
    pub js_code: String,
}

/// 加载所有页面
pub fn load_all_pages() -> HashMap<String, PageInfo> {
    let mut pages = HashMap::new();
    
    pages.insert("pages/index/index".to_string(), PageInfo {
        path: "pages/index/index".to_string(),
        wxml: include_str!("../../../sample-app/pages/index/index.wxml").to_string(),
        wxss: include_str!("../../../sample-app/pages/index/index.wxss").to_string(),
        js: include_str!("../../../sample-app/pages/index/index.js").to_string(),
    });
    
    pages.insert("pages/category/category".to_string(), PageInfo {
        path: "pages/category/category".to_string(),
        wxml: include_str!("../../../sample-app/pages/category/category.wxml").to_string(),
        wxss: include_str!("../../../sample-app/pages/category/category.wxss").to_string(),
        js: include_str!("../../../sample-app/pages/category/category.js").to_string(),
    });
    
    pages.insert("pages/cart/cart".to_string(), PageInfo {
        path: "pages/cart/cart".to_string(),
        wxml: include_str!("../../../sample-app/pages/cart/cart.wxml").to_string(),
        wxss: include_str!("../../../sample-app/pages/cart/cart.wxss").to_string(),
        js: include_str!("../../../sample-app/pages/cart/cart.js").to_string(),
    });
    
    pages.insert("pages/profile/profile".to_string(), PageInfo {
        path: "pages/profile/profile".to_string(),
        wxml: include_str!("../../../sample-app/pages/profile/profile.wxml").to_string(),
        wxss: include_str!("../../../sample-app/pages/profile/profile.wxss").to_string(),
        js: include_str!("../../../sample-app/pages/profile/profile.js").to_string(),
    });
    
    pages.insert("pages/detail/detail".to_string(), PageInfo {
        path: "pages/detail/detail".to_string(),
        wxml: include_str!("../../../sample-app/pages/detail/detail.wxml").to_string(),
        wxss: include_str!("../../../sample-app/pages/detail/detail.wxss").to_string(),
        js: include_str!("../../../sample-app/pages/detail/detail.js").to_string(),
    });
    
    pages
}

/// 加载自定义 TabBar
pub fn load_custom_tabbar() -> Result<Option<CustomTabBar>, String> {
    let wxml = include_str!("../../../sample-app/custom-tab-bar/index.wxml");
    let wxss = include_str!("../../../sample-app/custom-tab-bar/index.wxss");
    let js = include_str!("../../../sample-app/custom-tab-bar/index.js");
    
    let mut wxml_parser = WxmlParser::new(wxml);
    let wxml_nodes = wxml_parser.parse().map_err(|e| format!("Custom TabBar WXML error: {}", e))?;
    
    let mut wxss_parser = WxssParser::new(wxss);
    let stylesheet = wxss_parser.parse().map_err(|e| format!("Custom TabBar WXSS error: {}", e))?;
    
    Ok(Some(CustomTabBar {
        wxml_nodes,
        stylesheet,
        js_code: js.to_string(),
    }))
}
