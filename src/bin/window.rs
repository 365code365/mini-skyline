//! 带窗口的小程序运行器 - 支持多页面导航和原生 TabBar

use mini_render::runtime::MiniApp;
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::renderer::WxmlRenderer;
use mini_render::ui::interaction::{InteractionManager, InteractionResult, KeyInput};
use mini_render::{Canvas, Color, Paint, PaintStyle};
use mini_render::text::TextRenderer;
use serde_json::json;
use serde::Deserialize;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;
use std::collections::HashMap;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

const LOGICAL_WIDTH: u32 = 375;
const LOGICAL_HEIGHT: u32 = 667;
const CONTENT_HEIGHT: u32 = 1500;
const TABBAR_HEIGHT: u32 = 56;  // 自定义 TabBar 高度 (100rpx + padding)

/// app.json 配置结构
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AppConfig {
    pages: Vec<String>,
    #[serde(default)]
    window: WindowConfig,
    #[serde(default)]
    tab_bar: Option<TabBarConfig>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct WindowConfig {
    #[serde(default = "default_nav_title")]
    navigation_bar_title_text: String,
    #[serde(default = "default_nav_bg")]
    navigation_bar_background_color: String,
    #[serde(default)]
    navigation_bar_text_style: String,
    #[serde(default = "default_bg")]
    background_color: String,
}

fn default_nav_title() -> String { "Mini App".to_string() }
fn default_nav_bg() -> String { "#000000".to_string() }
fn default_bg() -> String { "#FFFFFF".to_string() }

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct TabBarConfig {
    #[serde(default)]
    custom: bool,
    #[serde(default = "default_tab_color")]
    color: String,
    #[serde(default = "default_tab_selected")]
    selected_color: String,
    #[serde(default = "default_tab_bg")]
    background_color: String,
    #[serde(default)]
    list: Vec<TabBarItem>,
}

fn default_tab_color() -> String { "#999999".to_string() }
fn default_tab_selected() -> String { "#007AFF".to_string() }
fn default_tab_bg() -> String { "#FFFFFF".to_string() }

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct TabBarItem {
    page_path: String,
    text: String,
    #[serde(default)]
    icon_path: String,
    #[serde(default)]
    selected_icon_path: String,
}

/// 页面信息
struct PageInfo {
    path: String,
    wxml: String,
    wxss: String,
    js: String,
}

/// 页面栈中的页面
struct PageInstance {
    path: String,
    query: HashMap<String, String>,
    wxml_nodes: Vec<mini_render::parser::wxml::WxmlNode>,
    stylesheet: mini_render::parser::wxss::StyleSheet,
}

/// 自定义 TabBar 组件
struct CustomTabBar {
    wxml_nodes: Vec<mini_render::parser::wxml::WxmlNode>,
    stylesheet: mini_render::parser::wxss::StyleSheet,
    js_code: String,
}

/// 微信小程序风格滚动控制器
struct ScrollController {
    position: f32,
    velocity: f32,
    min_scroll: f32,
    max_scroll: f32,
    is_dragging: bool,
    drag_start_pos: f32,
    drag_start_scroll: f32,
    velocity_samples: Vec<(f32, Instant)>,
    is_decelerating: bool,
    is_bouncing: bool,
    bounce_start_time: Instant,
    bounce_start_pos: f32,
    bounce_target_pos: f32,
}

impl ScrollController {
    fn new(content_height: f32, viewport_height: f32) -> Self {
        Self {
            position: 0.0,
            velocity: 0.0,
            min_scroll: 0.0,
            max_scroll: (content_height - viewport_height).max(0.0),
            is_dragging: false,
            drag_start_pos: 0.0,
            drag_start_scroll: 0.0,
            velocity_samples: Vec::with_capacity(10),
            is_decelerating: false,
            is_bouncing: false,
            bounce_start_time: Instant::now(),
            bounce_start_pos: 0.0,
            bounce_target_pos: 0.0,
        }
    }
    
    fn begin_drag(&mut self, y: f32) {
        self.is_dragging = true;
        self.is_decelerating = false;
        self.is_bouncing = false;
        self.drag_start_pos = y;
        self.drag_start_scroll = self.position;
        self.velocity = 0.0;
        self.velocity_samples.clear();
        self.velocity_samples.push((y, Instant::now()));
    }
    
    fn update_drag(&mut self, y: f32) {
        if !self.is_dragging { return; }
        let delta = self.drag_start_pos - y;
        let mut new_pos = self.drag_start_scroll + delta;
        if new_pos < self.min_scroll {
            let overshoot = self.min_scroll - new_pos;
            new_pos = self.min_scroll - Self::rubber_band(overshoot, LOGICAL_HEIGHT as f32);
        } else if new_pos > self.max_scroll {
            let overshoot = new_pos - self.max_scroll;
            new_pos = self.max_scroll + Self::rubber_band(overshoot, LOGICAL_HEIGHT as f32);
        }
        self.position = new_pos;
        let now = Instant::now();
        self.velocity_samples.push((y, now));
        self.velocity_samples.retain(|(_, t)| now.duration_since(*t).as_millis() < 100);
    }
    
    fn end_drag(&mut self) -> bool {
        if !self.is_dragging { return false; }
        self.is_dragging = false;
        self.velocity = self.calculate_release_velocity();
        if self.position < self.min_scroll || self.position > self.max_scroll {
            self.start_bounce();
        } else if self.velocity.abs() > 50.0 {
            self.is_decelerating = true;
        }
        self.is_decelerating || self.is_bouncing
    }

    fn calculate_release_velocity(&self) -> f32 {
        if self.velocity_samples.len() < 2 { return 0.0; }
        let first = self.velocity_samples.first().unwrap();
        let last = self.velocity_samples.last().unwrap();
        let dt = last.1.duration_since(first.1).as_secs_f32();
        if dt < 0.001 { return 0.0; }
        (first.0 - last.0) / dt * 0.8
    }
    
    fn rubber_band(offset: f32, dimension: f32) -> f32 {
        let c = 0.55;
        let x = offset.abs() / dimension;
        let result = (1.0 - (1.0 / (x * c + 1.0))) * dimension;
        if offset < 0.0 { -result } else { result }
    }
    
    fn start_bounce(&mut self) {
        self.is_bouncing = true;
        self.is_decelerating = false;
        self.bounce_start_time = Instant::now();
        self.bounce_start_pos = self.position;
        self.bounce_target_pos = self.position.clamp(self.min_scroll, self.max_scroll);
        self.velocity = 0.0;
    }
    
    fn update(&mut self, dt: f32) -> bool {
        if self.is_dragging { return false; }
        if self.is_bouncing {
            let elapsed = self.bounce_start_time.elapsed().as_secs_f32();
            let duration = 0.4;
            if elapsed >= duration {
                self.position = self.bounce_target_pos;
                self.is_bouncing = false;
                return false;
            }
            let t = elapsed / duration;
            let ease = 1.0 - (1.0 - t).powi(3);
            self.position = self.bounce_start_pos + (self.bounce_target_pos - self.bounce_start_pos) * ease;
            return true;
        }
        if self.is_decelerating {
            let deceleration = 0.998_f32.powf(dt * 1000.0);
            self.velocity *= deceleration;
            self.position += self.velocity * dt;
            if self.position < self.min_scroll {
                self.position = self.min_scroll;
                self.start_bounce();
                return true;
            } else if self.position > self.max_scroll {
                self.position = self.max_scroll;
                self.start_bounce();
                return true;
            }
            if self.velocity.abs() < 10.0 {
                self.velocity = 0.0;
                self.is_decelerating = false;
                return false;
            }
            return true;
        }
        false
    }
    
    fn handle_wheel(&mut self, delta: f32) {
        self.velocity += delta * 15.0;
        self.is_decelerating = true;
        self.is_bouncing = false;
    }
    
    fn get_position(&self) -> f32 { self.position }
    fn is_animating(&self) -> bool { self.is_decelerating || self.is_bouncing }
}


struct MiniAppWindow {
    window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    app: MiniApp,
    canvas: Option<Canvas>,
    tabbar_canvas: Option<Canvas>,
    fixed_canvas: Option<Canvas>,  // 用于渲染 fixed 元素
    renderer: Option<WxmlRenderer>,
    tabbar_renderer: Option<WxmlRenderer>,
    text_renderer: Option<TextRenderer>,
    // 页面栈
    page_stack: Vec<PageInstance>,
    current_page_index: usize,
    // 页面资源
    pages: HashMap<String, PageInfo>,
    // app.json 配置
    app_config: AppConfig,
    // 自定义 TabBar
    custom_tabbar: Option<CustomTabBar>,
    mouse_pos: (f32, f32),
    needs_redraw: bool,
    scale_factor: f64,
    scroll: ScrollController,
    last_frame: Instant,
    click_start_pos: (f32, f32),
    click_start_time: Instant,
    // 导航请求
    pending_navigation: Option<NavigationRequest>,
    // 交互管理器
    interaction: InteractionManager,
    // 键盘修饰键状态
    modifiers: winit::keyboard::ModifiersState,
    // 跨平台剪贴板
    clipboard: Option<arboard::Clipboard>,
}

#[derive(Clone)]
enum NavigationRequest {
    NavigateTo { url: String },
    NavigateBack,
    SwitchTab { url: String },
}

impl MiniAppWindow {
    fn new() -> Result<Self, String> {
        let mut app = MiniApp::new(LOGICAL_WIDTH, LOGICAL_HEIGHT)?;
        app.init()?;
        
        // 加载 app.json 配置
        let app_json = include_str!("../../sample-app/app.json");
        let app_config: AppConfig = serde_json::from_str(app_json)
            .map_err(|e| format!("Failed to parse app.json: {}", e))?;
        
        println!("📱 App config loaded");
        if let Some(ref tab_bar) = app_config.tab_bar {
            println!("   TabBar: {} items, custom: {}", tab_bar.list.len(), tab_bar.custom);
            for item in &tab_bar.list {
                println!("     - {} ({})", item.text, item.page_path);
            }
        }
        
        // 加载自定义 TabBar（如果启用）
        let custom_tabbar = if app_config.tab_bar.as_ref().map(|tb| tb.custom).unwrap_or(false) {
            Self::load_custom_tabbar()?
        } else {
            None
        };
        
        if custom_tabbar.is_some() {
            println!("   ✅ Custom TabBar loaded");
        }
        
        // 加载所有页面资源
        let mut pages = HashMap::new();
        
        // Index 页面 (首页)
        pages.insert("pages/index/index".to_string(), PageInfo {
            path: "pages/index/index".to_string(),
            wxml: include_str!("../../sample-app/pages/index/index.wxml").to_string(),
            wxss: include_str!("../../sample-app/pages/index/index.wxss").to_string(),
            js: include_str!("../../sample-app/pages/index/index.js").to_string(),
        });
        
        // Category 页面 (分类)
        pages.insert("pages/category/category".to_string(), PageInfo {
            path: "pages/category/category".to_string(),
            wxml: include_str!("../../sample-app/pages/category/category.wxml").to_string(),
            wxss: include_str!("../../sample-app/pages/category/category.wxss").to_string(),
            js: include_str!("../../sample-app/pages/category/category.js").to_string(),
        });
        
        // Cart 页面 (购物车)
        pages.insert("pages/cart/cart".to_string(), PageInfo {
            path: "pages/cart/cart".to_string(),
            wxml: include_str!("../../sample-app/pages/cart/cart.wxml").to_string(),
            wxss: include_str!("../../sample-app/pages/cart/cart.wxss").to_string(),
            js: include_str!("../../sample-app/pages/cart/cart.js").to_string(),
        });
        
        // Profile 页面 (我的)
        pages.insert("pages/profile/profile".to_string(), PageInfo {
            path: "pages/profile/profile".to_string(),
            wxml: include_str!("../../sample-app/pages/profile/profile.wxml").to_string(),
            wxss: include_str!("../../sample-app/pages/profile/profile.wxss").to_string(),
            js: include_str!("../../sample-app/pages/profile/profile.js").to_string(),
        });
        
        // Detail 页面 (商品详情)
        pages.insert("pages/detail/detail".to_string(), PageInfo {
            path: "pages/detail/detail".to_string(),
            wxml: include_str!("../../sample-app/pages/detail/detail.wxml").to_string(),
            wxss: include_str!("../../sample-app/pages/detail/detail.wxss").to_string(),
            js: include_str!("../../sample-app/pages/detail/detail.js").to_string(),
        });
        
        // 判断首页是否有 TabBar
        let has_tabbar = app_config.tab_bar.as_ref()
            .map(|tb| tb.list.iter().any(|item| item.page_path == "pages/index/index"))
            .unwrap_or(false);
        
        let now = Instant::now();
        
        // 初始化跨平台剪贴板
        let clipboard = arboard::Clipboard::new().ok();
        if clipboard.is_some() {
            println!("📋 Clipboard initialized");
        }
        
        let mut window = Self {
            window: None,
            surface: None,
            app,
            canvas: None,
            tabbar_canvas: None,
            fixed_canvas: None,
            renderer: None,
            tabbar_renderer: None,
            text_renderer: None,
            page_stack: Vec::new(),
            current_page_index: 0,
            pages,
            app_config,
            custom_tabbar,
            mouse_pos: (0.0, 0.0),
            needs_redraw: true,
            scale_factor: 1.0,
            scroll: ScrollController::new(
                CONTENT_HEIGHT as f32, 
                (LOGICAL_HEIGHT - if has_tabbar { TABBAR_HEIGHT } else { 0 }) as f32
            ),
            last_frame: now,
            click_start_pos: (0.0, 0.0),
            click_start_time: now,
            pending_navigation: None,
            interaction: InteractionManager::new(),
            modifiers: winit::keyboard::ModifiersState::empty(),
            clipboard,
        };
        
        // 加载首页
        window.navigate_to("pages/index/index", HashMap::new())?;
        
        Ok(window)
    }
    
    /// 加载自定义 TabBar 组件
    fn load_custom_tabbar() -> Result<Option<CustomTabBar>, String> {
        let wxml = include_str!("../../sample-app/custom-tab-bar/index.wxml");
        let wxss = include_str!("../../sample-app/custom-tab-bar/index.wxss");
        let js = include_str!("../../sample-app/custom-tab-bar/index.js");
        
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
    
    /// 检查当前页面是否在 TabBar 中
    fn is_tabbar_page(&self, path: &str) -> bool {
        self.app_config.tab_bar.as_ref()
            .map(|tb| tb.list.iter().any(|item| item.page_path == path))
            .unwrap_or(false)
    }
    
    /// 获取当前页面在 TabBar 中的索引
    fn get_tabbar_index(&self, path: &str) -> Option<usize> {
        self.app_config.tab_bar.as_ref()
            .and_then(|tb| tb.list.iter().position(|item| item.page_path == path))
    }
    
    /// 是否使用自定义 TabBar
    fn is_custom_tabbar(&self) -> bool {
        self.app_config.tab_bar.as_ref().map(|tb| tb.custom).unwrap_or(false) 
            && self.custom_tabbar.is_some()
    }

    fn navigate_to(&mut self, path: &str, query: HashMap<String, String>) -> Result<(), String> {
        let path = path.trim_start_matches('/');
        println!("📄 Navigate to: {} {:?}", path, query);
        
        let page_info = self.pages.get(path)
            .ok_or_else(|| format!("Page not found: {}", path))?;
        
        // 解析 WXML - 移除手动写的 tabbar
        let mut wxml_parser = WxmlParser::new(&page_info.wxml);
        let all_nodes = wxml_parser.parse().map_err(|e| format!("WXML parse error: {}", e))?;
        let wxml_nodes = Self::remove_manual_tabbar(&all_nodes);
        
        // 解析 WXSS
        let mut wxss_parser = WxssParser::new(&page_info.wxss);
        let stylesheet = wxss_parser.parse().map_err(|e| format!("WXSS parse error: {}", e))?;
        
        // 加载 JS
        self.app.load_script(&page_info.js)?;
        
        // 调用 onLoad
        let query_json = serde_json::to_string(&query).unwrap_or("{}".to_string());
        let load_code = format!("if(__currentPage && __currentPage.onLoad) __currentPage.onLoad({})", query_json);
        self.app.eval(&load_code).ok();
        self.print_js_output();
        
        // 创建页面实例
        let page_instance = PageInstance {
            path: path.to_string(),
            query,
            wxml_nodes,
            stylesheet,
        };
        
        self.page_stack.push(page_instance);
        self.current_page_index = self.page_stack.len() - 1;
        
        // 根据是否有原生 TabBar 调整滚动区域
        let has_tabbar = self.is_tabbar_page(path);
        self.scroll = ScrollController::new(
            CONTENT_HEIGHT as f32, 
            (LOGICAL_HEIGHT - if has_tabbar { TABBAR_HEIGHT } else { 0 }) as f32
        );
        self.needs_redraw = true;
        
        println!("✅ Page loaded: {} (stack size: {}, tabbar: {})", path, self.page_stack.len(), has_tabbar);
        Ok(())
    }
    
    /// 移除 WXML 中手动写的 tabbar 元素
    fn remove_manual_tabbar(nodes: &[mini_render::parser::wxml::WxmlNode]) -> Vec<mini_render::parser::wxml::WxmlNode> {
        use mini_render::parser::wxml::{WxmlNode, WxmlNodeType};
        
        fn filter_node(node: &WxmlNode) -> Option<WxmlNode> {
            if node.node_type != WxmlNodeType::Element {
                return Some(node.clone());
            }
            
            let class = node.attributes.get("class").map(|s| s.as_str()).unwrap_or("");
            // 移除 tabbar 和 tabbar-placeholder
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
    
    fn navigate_back(&mut self) -> Result<(), String> {
        if self.page_stack.len() <= 1 {
            println!("⚠️ Already at root page");
            return Ok(());
        }
        
        self.page_stack.pop();
        self.current_page_index = self.page_stack.len() - 1;
        
        // 重新加载当前页面的 JS
        if let Some(page) = self.page_stack.last() {
            let path = page.path.clone();
            let query = page.query.clone();
            if let Some(page_info) = self.pages.get(&path) {
                self.app.load_script(&page_info.js)?;
                let query_json = serde_json::to_string(&query).unwrap_or("{}".to_string());
                let load_code = format!("if(__currentPage && __currentPage.onLoad) __currentPage.onLoad({})", query_json);
                self.app.eval(&load_code).ok();
                self.print_js_output();
            }
            
            let has_tabbar = self.is_tabbar_page(&path);
            self.scroll = ScrollController::new(
                CONTENT_HEIGHT as f32,
                (LOGICAL_HEIGHT - if has_tabbar { TABBAR_HEIGHT } else { 0 }) as f32
            );
        }
        
        self.needs_redraw = true;
        println!("⬅️ Navigate back (stack size: {})", self.page_stack.len());
        Ok(())
    }
    
    fn switch_tab(&mut self, path: &str) -> Result<(), String> {
        let path = path.trim_start_matches('/');
        println!("🔄 Switch tab to: {}", path);
        
        // 清空页面栈，只保留目标页面
        self.page_stack.clear();
        // 清除交互状态
        self.interaction.clear_page_state();
        self.navigate_to(path, HashMap::new())
    }
    
    fn print_js_output(&self) {
        if let Ok(output) = self.app.eval("__print_buffer.splice(0).join('\\n')") {
            if !output.is_empty() && output != "undefined" {
                for line in output.lines() {
                    println!("   {}", line);
                }
            }
        }
    }
    
    fn setup_canvas(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
        let physical_width = (LOGICAL_WIDTH as f64 * scale_factor) as u32;
        let physical_height = (CONTENT_HEIGHT as f64 * scale_factor) as u32;
        let tabbar_physical_height = (TABBAR_HEIGHT as f64 * scale_factor) as u32;
        let viewport_physical_height = (LOGICAL_HEIGHT as f64 * scale_factor) as u32;
        
        println!("📐 Scale: {}x | Content: {}x{}", scale_factor, LOGICAL_WIDTH, CONTENT_HEIGHT);
        
        self.canvas = Some(Canvas::new(physical_width, physical_height));
        self.tabbar_canvas = Some(Canvas::new(physical_width, tabbar_physical_height));
        self.fixed_canvas = Some(Canvas::new(physical_width, viewport_physical_height));
        
        // 初始化文本渲染器
        self.text_renderer = TextRenderer::load_system_font()
            .or_else(|_| TextRenderer::from_bytes(include_bytes!("../../assets/ArialUnicode.ttf")))
            .ok();
    }
    
    fn update_renderers(&mut self) {
        if let Some(page) = self.page_stack.last() {
            self.renderer = Some(WxmlRenderer::new_with_scale(
                page.stylesheet.clone(),
                LOGICAL_WIDTH as f32,
                CONTENT_HEIGHT as f32,
                self.scale_factor as f32,
            ));
            
            // 自定义 TabBar 渲染器
            if let Some(ref custom_tabbar) = self.custom_tabbar {
                self.tabbar_renderer = Some(WxmlRenderer::new_with_scale(
                    custom_tabbar.stylesheet.clone(),
                    LOGICAL_WIDTH as f32,
                    TABBAR_HEIGHT as f32,
                    self.scale_factor as f32,
                ));
            }
        }
    }

    fn render(&mut self) {
        let page_data = if let Ok(data_str) = self.app.eval("__getPageData()") {
            serde_json::from_str(&data_str).unwrap_or(json!({}))
        } else {
            json!({})
        };
        
        let page = match self.page_stack.last() {
            Some(p) => p,
            None => return,
        };
        
        let current_path = page.path.clone();
        let wxml_nodes = page.wxml_nodes.clone();
        
        // 获取滚动偏移
        let scroll_offset = self.scroll.get_position();
        
        // 计算视口高度（不包括 TabBar）
        let has_tabbar = self.is_tabbar_page(&current_path);
        let viewport_height = (LOGICAL_HEIGHT - if has_tabbar { TABBAR_HEIGHT } else { 0 }) as f32;
        
        // 渲染内容区域（不包括 fixed 元素）
        if let Some(canvas) = &mut self.canvas {
            canvas.clear(Color::from_hex(0xF5F5F5));
            
            if let Some(renderer) = &mut self.renderer {
                renderer.render_with_scroll_and_viewport(canvas, &wxml_nodes, &page_data, &mut self.interaction, scroll_offset, viewport_height);
            }
        }
        
        // 渲染 fixed 元素到单独的 canvas
        if let Some(fixed_canvas) = &mut self.fixed_canvas {
            fixed_canvas.clear(Color::new(0, 0, 0, 0)); // 透明背景
            
            if let Some(renderer) = &mut self.renderer {
                renderer.render_fixed_elements(fixed_canvas, &wxml_nodes, &page_data, &mut self.interaction, viewport_height);
            }
        }
        
        // 渲染 TabBar（如果当前页面在 TabBar 中）
        if has_tabbar {
            if self.is_custom_tabbar() {
                self.render_custom_tabbar(&current_path);
            } else {
                self.render_native_tabbar(&current_path);
            }
        }
    }
    
    /// 渲染自定义 TabBar
    fn render_custom_tabbar(&mut self, current_path: &str) {
        let tab_bar_config = match &self.app_config.tab_bar {
            Some(tb) => tb.clone(),
            None => return,
        };
        
        // 构建自定义 TabBar 的数据
        let selected_index = self.get_tabbar_index(current_path).unwrap_or(0);
        let list: Vec<serde_json::Value> = tab_bar_config.list.iter().map(|item| {
            json!({
                "pagePath": item.page_path,
                "text": item.text
            })
        }).collect();
        
        let tabbar_data = json!({
            "selected": selected_index,
            "list": list
        });
        
        let custom_tabbar = match &self.custom_tabbar {
            Some(ct) => ct,
            None => return,
        };
        let wxml_nodes = custom_tabbar.wxml_nodes.clone();
        
        let canvas = match &mut self.tabbar_canvas {
            Some(c) => c,
            None => return,
        };
        
        let renderer = match &mut self.tabbar_renderer {
            Some(r) => r,
            None => return,
        };
        
        // 渲染自定义 TabBar
        canvas.clear(Color::WHITE);
        renderer.render(canvas, &wxml_nodes, &tabbar_data);
    }
    
    /// 渲染原生 TabBar
    fn render_native_tabbar(&mut self, current_path: &str) {
        let tab_bar = match &self.app_config.tab_bar {
            Some(tb) => tb.clone(),
            None => return,
        };
        
        let canvas = match &mut self.tabbar_canvas {
            Some(c) => c,
            None => return,
        };
        
        let text_renderer: &TextRenderer = match &self.text_renderer {
            Some(tr) => tr,
            None => return,
        };
        
        let sf = self.scale_factor as f32;
        let width = LOGICAL_WIDTH as f32 * sf;
        let height = TABBAR_HEIGHT as f32 * sf;
        
        // 背景色
        let bg_color = Self::parse_color(&tab_bar.background_color).unwrap_or(Color::WHITE);
        canvas.clear(bg_color);
        
        // 顶部分割线
        let line_paint = Paint::new().with_color(Color::from_hex(0xE5E5E5)).with_style(PaintStyle::Fill);
        canvas.draw_rect(&mini_render::Rect::new(0.0, 0.0, width, 1.0 * sf), &line_paint);
        
        let normal_color = Self::parse_color(&tab_bar.color).unwrap_or(Color::from_hex(0x999999));
        let selected_color = Self::parse_color(&tab_bar.selected_color).unwrap_or(Color::from_hex(0x007AFF));
        
        let item_count = tab_bar.list.len();
        if item_count == 0 { return; }
        
        let item_width = width / item_count as f32;
        let icon_size = 24.0 * sf;
        let label_size = 10.0 * sf;
        let icon_y = 8.0 * sf;
        let label_y = icon_y + icon_size + 4.0 * sf;
        
        for (i, item) in tab_bar.list.iter().enumerate() {
            let is_selected = item.page_path == current_path;
            let color = if is_selected { selected_color } else { normal_color };
            let x = i as f32 * item_width;
            
            // 绘制图标占位符（用文字代替）
            let icon_text = item.text.chars().next().unwrap_or('?').to_string();
            let icon_paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            let icon_text_width = text_renderer.measure_text(&icon_text, icon_size);
            let icon_x = x + (item_width - icon_text_width) / 2.0;
            text_renderer.draw_text(canvas, &icon_text, icon_x, icon_y + icon_size, icon_size, &icon_paint);
            
            // 绘制标签
            let label_paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            let label_width = text_renderer.measure_text(&item.text, label_size);
            let label_x = x + (item_width - label_width) / 2.0;
            text_renderer.draw_text(canvas, &item.text, label_x, label_y + label_size, label_size, &label_paint);
        }
    }
    
    /// 解析颜色字符串
    fn parse_color(s: &str) -> Option<Color> {
        let s = s.trim().trim_start_matches('#');
        if s.len() == 6 {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some(Color::new(r, g, b, 255))
        } else {
            None
        }
    }
    
    fn present(&mut self) {
        let canvas = match &self.canvas { Some(c) => c, None => return };
        let page = match self.page_stack.last() { Some(p) => p, None => return };
        let has_tabbar = self.is_tabbar_page(&page.path);
        
        if let (Some(window), Some(surface)) = (&self.window, &mut self.surface) {
            let size = window.inner_size();
            if let (Some(win_width), Some(win_height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
                surface.resize(win_width, win_height).ok();
                
                if let Ok(mut buffer) = surface.buffer_mut() {
                    let canvas_data = canvas.to_rgba();
                    let canvas_width = canvas.width();
                    let canvas_height = canvas.height();
                    let scroll_offset = (self.scroll.get_position() * self.scale_factor as f32) as i32;
                    
                    let tabbar_physical_height = if has_tabbar { (TABBAR_HEIGHT as f64 * self.scale_factor) as u32 } else { 0 };
                    let content_area_height = size.height - tabbar_physical_height;
                    
                    // 渲染内容区域
                    for y in 0..content_area_height {
                        let src_y = (y as i32 + scroll_offset).clamp(0, canvas_height as i32 - 1) as u32;
                        for x in 0..size.width.min(canvas_width) {
                            let src_idx = ((src_y * canvas_width + x) * 4) as usize;
                            let dst_idx = (y * size.width + x) as usize;
                            if src_idx + 3 < canvas_data.len() && dst_idx < buffer.len() {
                                let r = canvas_data[src_idx] as u32;
                                let g = canvas_data[src_idx + 1] as u32;
                                let b = canvas_data[src_idx + 2] as u32;
                                buffer[dst_idx] = (r << 16) | (g << 8) | b;
                            }
                        }
                    }
                    
                    // 渲染 fixed 元素（覆盖在内容区域上）
                    if let Some(fixed_canvas) = &self.fixed_canvas {
                        let fixed_data = fixed_canvas.to_rgba();
                        let fixed_width = fixed_canvas.width();
                        let fixed_height = fixed_canvas.height();
                        for y in 0..content_area_height.min(fixed_height) {
                            for x in 0..size.width.min(fixed_width) {
                                let src_idx = ((y * fixed_width + x) * 4) as usize;
                                let dst_idx = (y * size.width + x) as usize;
                                if src_idx + 3 < fixed_data.len() && dst_idx < buffer.len() {
                                    let a = fixed_data[src_idx + 3];
                                    if a > 0 {
                                        // Alpha 混合
                                        let r = fixed_data[src_idx] as u32;
                                        let g = fixed_data[src_idx + 1] as u32;
                                        let b = fixed_data[src_idx + 2] as u32;
                                        if a == 255 {
                                            buffer[dst_idx] = (r << 16) | (g << 8) | b;
                                        } else {
                                            // 简单的 alpha 混合
                                            let dst = buffer[dst_idx];
                                            let dst_r = (dst >> 16) & 0xFF;
                                            let dst_g = (dst >> 8) & 0xFF;
                                            let dst_b = dst & 0xFF;
                                            let alpha = a as u32;
                                            let inv_alpha = 255 - alpha;
                                            let new_r = (r * alpha + dst_r * inv_alpha) / 255;
                                            let new_g = (g * alpha + dst_g * inv_alpha) / 255;
                                            let new_b = (b * alpha + dst_b * inv_alpha) / 255;
                                            buffer[dst_idx] = (new_r << 16) | (new_g << 8) | new_b;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // 渲染原生 TabBar
                    if has_tabbar {
                        if let Some(tabbar_canvas) = &self.tabbar_canvas {
                            let tabbar_data = tabbar_canvas.to_rgba();
                            let tabbar_width = tabbar_canvas.width();
                            let tabbar_height = tabbar_canvas.height();
                            for y in 0..tabbar_physical_height.min(tabbar_height) {
                                let dst_y = content_area_height + y;
                                for x in 0..size.width.min(tabbar_width) {
                                    let src_idx = ((y * tabbar_width + x) * 4) as usize;
                                    let dst_idx = (dst_y * size.width + x) as usize;
                                    if src_idx + 3 < tabbar_data.len() && dst_idx < buffer.len() {
                                        let r = tabbar_data[src_idx] as u32;
                                        let g = tabbar_data[src_idx + 1] as u32;
                                        let b = tabbar_data[src_idx + 2] as u32;
                                        buffer[dst_idx] = (r << 16) | (g << 8) | b;
                                    }
                                }
                            }
                        }
                    }
                    buffer.present().ok();
                }
            }
        }
    }

    fn handle_click(&mut self, x: f32, y: f32) {
        let page = match self.page_stack.last() { Some(p) => p, None => return };
        let has_tabbar = self.is_tabbar_page(&page.path);
        let tabbar_y = if has_tabbar { (LOGICAL_HEIGHT - TABBAR_HEIGHT) as f32 } else { LOGICAL_HEIGHT as f32 };
        
        if has_tabbar && y >= tabbar_y {
            // TabBar 点击
            if self.is_custom_tabbar() {
                self.handle_custom_tabbar_click(x, y - tabbar_y);
            } else {
                self.handle_tabbar_click(x);
            }
        } else {
            // 内容区域点击
            let scroll_pos = self.scroll.get_position();
            let actual_y = y + scroll_pos;
            
            // 首先检查 fixed 元素（使用视口坐标，不加滚动偏移）
            // fixed 元素的事件绑定是相对于视口的
            let fixed_binding = if let Some(renderer) = &self.renderer {
                if let Some(binding) = renderer.hit_test(x, y) {
                    // 检查这个绑定是否在视口范围内（可能是 fixed 元素）
                    if binding.bounds.y >= 0.0 && binding.bounds.y < tabbar_y {
                        Some((binding.event_type.clone(), binding.handler.clone(), binding.data.clone(), binding.bounds))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };
            
            if let Some((event_type, handler, data, _bounds)) = fixed_binding {
                // 检查交互元素（使用视口坐标）
                if let Some(result) = self.interaction.handle_click(x, y) {
                    self.handle_interaction_result(result.clone());
                    
                    let should_call_js = match &result {
                        InteractionResult::ButtonClick { .. } => true,
                        InteractionResult::Toggle { .. } => true,
                        InteractionResult::Select { .. } => true,
                        _ => false,
                    };
                    
                    if should_call_js {
                        println!("👆 {} -> {}", event_type, handler);
                        let data_json = serde_json::to_string(&data).unwrap_or("{}".to_string());
                        let call_code = format!("__callPageMethod('{}', {})", handler, data_json);
                        self.app.eval(&call_code).ok();
                        self.check_navigation();
                        self.print_js_output();
                    }
                    
                    self.needs_redraw = true;
                    return;
                }
                
                // 如果没有交互元素，直接调用事件处理
                println!("👆 {} -> {}", event_type, handler);
                let data_json = serde_json::to_string(&data).unwrap_or("{}".to_string());
                let call_code = format!("__callPageMethod('{}', {})", handler, data_json);
                self.app.eval(&call_code).ok();
                self.check_navigation();
                self.print_js_output();
                self.needs_redraw = true;
                return;
            }
            
            // 使用交互管理器处理点击（按钮点击也在这里处理）
            if let Some(result) = self.interaction.handle_click(x, actual_y) {
                self.handle_interaction_result(result.clone());
                
                // 检查事件绑定并调用 JS 处理函数
                let should_call_js = match &result {
                    InteractionResult::ButtonClick { .. } => true,
                    InteractionResult::Toggle { .. } => true,  // switch/checkbox 的 bindchange
                    InteractionResult::Select { .. } => true,  // radio 的 bindchange
                    _ => false,
                };
                
                if should_call_js {
                    if let Some(renderer) = &self.renderer {
                        if let Some(binding) = renderer.hit_test(x, actual_y) {
                            println!("👆 {} -> {}", binding.event_type, binding.handler);
                            let data_json = serde_json::to_string(&binding.data).unwrap_or("{}".to_string());
                            let call_code = format!("__callPageMethod('{}', {})", binding.handler, data_json);
                            self.app.eval(&call_code).ok();
                            self.check_navigation();
                            self.print_js_output();
                        }
                    }
                }
                
                self.needs_redraw = true;
                return;
            }
            
            // 检查其他事件绑定
            if let Some(renderer) = &self.renderer {
                if let Some(binding) = renderer.hit_test(x, actual_y) {
                    println!("👆 {} -> {}", binding.event_type, binding.handler);
                    let data_json = serde_json::to_string(&binding.data).unwrap_or("{}".to_string());
                    let call_code = format!("__callPageMethod('{}', {})", binding.handler, data_json);
                    self.app.eval(&call_code).ok();
                    self.check_navigation();
                    self.print_js_output();
                    self.needs_redraw = true;
                }
            }
        }
    }
    
    /// 处理交互结果
    fn handle_interaction_result(&mut self, result: InteractionResult) {
        match result {
            InteractionResult::Toggle { id, checked } => {
                println!("🔘 Toggle {}: {}", id, checked);
            }
            InteractionResult::Select { id, value } => {
                println!("🔘 Select {}: {}", id, value);
            }
            InteractionResult::SliderChange { id, value } => {
                println!("🎚️ Slider {}: {}", id, value);
            }
            InteractionResult::SliderEnd { id } => {
                println!("🎚️ Slider {} released", id);
            }
            InteractionResult::Focus { id, bounds } => {
                println!("📝 Focus: {} at ({:.0}, {:.0})", id, bounds.x, bounds.y);
                // 启用 IME 并设置位置到输入框下方
                if let Some(window) = &self.window {
                    window.set_ime_allowed(true);
                    // 计算物理像素位置（考虑滚动偏移）
                    let sf = self.scale_factor;
                    let scroll_offset = self.scroll.get_position();
                    let ime_x = (bounds.x * sf as f32) as f64;
                    let ime_y = ((bounds.y - scroll_offset + bounds.height + 5.0) * sf as f32) as f64;
                    let ime_w = (bounds.width * sf as f32) as f64;
                    let ime_h = (bounds.height * sf as f32) as f64;
                    
                    window.set_ime_cursor_area(
                        winit::dpi::PhysicalPosition::new(ime_x, ime_y),
                        winit::dpi::PhysicalSize::new(ime_w, ime_h),
                    );
                }
            }
            InteractionResult::InputChange { id, value } => {
                println!("📝 Input {}: {}", id, value);
            }
            InteractionResult::InputBlur { id, value } => {
                println!("📝 Blur {}: {}", id, value);
            }
            InteractionResult::ButtonClick { id, bounds: _ } => {
                println!("🔘 Button clicked: {}", id);
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            InteractionResult::CopyText { text } => {
                println!("📋 Copy: {}", text);
                // 复制到系统剪贴板
                if let Some(ref mut clipboard) = self.clipboard {
                    if let Err(e) = clipboard.set_text(&text) {
                        println!("❌ Clipboard copy failed: {}", e);
                    } else {
                        println!("✅ Copied to clipboard");
                    }
                }
            }
            InteractionResult::CutText { text, id, value } => {
                println!("✂️ Cut from {}: {} (remaining: {})", id, text, value);
                // 复制到系统剪贴板
                if let Some(ref mut clipboard) = self.clipboard {
                    if let Err(e) = clipboard.set_text(&text) {
                        println!("❌ Clipboard cut failed: {}", e);
                    } else {
                        println!("✅ Cut to clipboard");
                    }
                }
            }
        }
    }
    
    /// 处理自定义 TabBar 点击
    fn handle_custom_tabbar_click(&mut self, x: f32, y: f32) {
        if let Some(renderer) = &self.tabbar_renderer {
            if let Some(binding) = renderer.hit_test(x, y) {
                // 获取点击的 tab 索引和路径
                if let (Some(index_str), Some(path)) = (binding.data.get("index"), binding.data.get("path")) {
                    if let Ok(index) = index_str.parse::<usize>() {
                        let current_path = self.page_stack.last().map(|p| p.path.clone()).unwrap_or_default();
                        
                        if path != &current_path {
                            println!("👆 TabBar -> {} ({})", index, path);
                            self.pending_navigation = Some(NavigationRequest::SwitchTab { url: path.clone() });
                            if let Some(w) = &self.window { w.request_redraw(); }
                        }
                    }
                }
            }
        }
    }
    
    /// 处理原生 TabBar 点击
    fn handle_tabbar_click(&mut self, x: f32) {
        let tab_bar = match &self.app_config.tab_bar {
            Some(tb) => tb,
            None => return,
        };
        
        let item_count = tab_bar.list.len();
        if item_count == 0 { return; }
        
        let item_width = LOGICAL_WIDTH as f32 / item_count as f32;
        let clicked_index = (x / item_width) as usize;
        
        if clicked_index < item_count {
            let target_path = tab_bar.list[clicked_index].page_path.clone();
            let current_path = self.page_stack.last().map(|p| p.path.clone()).unwrap_or_default();
            
            if target_path != current_path {
                println!("👆 TabBar -> {} ({})", tab_bar.list[clicked_index].text, target_path);
                self.pending_navigation = Some(NavigationRequest::SwitchTab { url: target_path });
                if let Some(w) = &self.window { w.request_redraw(); }
            }
        }
    }
    
    fn check_navigation(&mut self) {
        // 检查是否有导航请求
        if let Ok(nav_str) = self.app.eval("JSON.stringify(__pendingNavigation || null)") {
            if nav_str != "null" && !nav_str.is_empty() {
                if let Ok(nav) = serde_json::from_str::<serde_json::Value>(&nav_str) {
                    if let Some(nav_type) = nav.get("type").and_then(|v| v.as_str()) {
                        let url = nav.get("url").and_then(|v| v.as_str()).unwrap_or("");
                        match nav_type {
                            "navigateTo" => {
                                self.pending_navigation = Some(NavigationRequest::NavigateTo { url: url.to_string() });
                            }
                            "navigateBack" => {
                                self.pending_navigation = Some(NavigationRequest::NavigateBack);
                            }
                            "switchTab" => {
                                self.pending_navigation = Some(NavigationRequest::SwitchTab { url: url.to_string() });
                            }
                            _ => {}
                        }
                    }
                }
                // 清除导航请求
                self.app.eval("__pendingNavigation = null").ok();
            }
        }
    }
    
    fn process_navigation(&mut self) {
        if let Some(nav) = self.pending_navigation.take() {
            match nav {
                NavigationRequest::NavigateTo { url } => {
                    let (path, query) = Self::parse_url(&url);
                    if let Err(e) = self.navigate_to(&path, query) {
                        println!("❌ Navigation error: {}", e);
                    }
                    self.update_renderers();
                }
                NavigationRequest::NavigateBack => {
                    if let Err(e) = self.navigate_back() {
                        println!("❌ Navigation error: {}", e);
                    }
                    self.update_renderers();
                }
                NavigationRequest::SwitchTab { url } => {
                    let (path, _) = Self::parse_url(&url);
                    if let Err(e) = self.switch_tab(&path) {
                        println!("❌ Navigation error: {}", e);
                    }
                    self.update_renderers();
                }
            }
        }
    }
    
    fn parse_url(url: &str) -> (String, HashMap<String, String>) {
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
    
    fn update_scroll(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        if self.scroll.update(dt) {
            if let Some(window) = &self.window { window.request_redraw(); }
        }
    }
}


impl ApplicationHandler for MiniAppWindow {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attrs = WindowAttributes::default()
                .with_title("Mini App")
                .with_inner_size(winit::dpi::LogicalSize::new(LOGICAL_WIDTH, LOGICAL_HEIGHT))
                .with_resizable(false);
            
            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
            
            // 启用 IME 输入（中文等）
            window.set_ime_allowed(true);
            
            let scale_factor = window.scale_factor();
            self.setup_canvas(scale_factor);
            self.update_renderers();
            
            let context = softbuffer::Context::new(window.clone()).unwrap();
            let surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
            
            self.window = Some(window);
            self.surface = Some(surface);
            
            self.render();
            self.present();
            
            println!("\n🎮 Ready! 点击导航到其他页面\n");
        }
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            
            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
            }
            
            WindowEvent::KeyboardInput { event, .. } => {
                use winit::keyboard::{PhysicalKey, KeyCode, ModifiersState};
                if event.state == ElementState::Pressed {
                    // 获取修饰键状态
                    let ctrl = self.modifiers.contains(ModifiersState::CONTROL) || self.modifiers.contains(ModifiersState::SUPER);
                    let shift = self.modifiers.contains(ModifiersState::SHIFT);
                    
                    // 处理输入框文本输入
                    if self.interaction.has_focused_input() {
                        let mut handled = true;
                        let key_input = if let PhysicalKey::Code(code) = event.physical_key {
                            match code {
                                KeyCode::Backspace => Some(KeyInput::Backspace),
                                KeyCode::Delete => Some(KeyInput::Delete),
                                KeyCode::ArrowLeft if shift => Some(KeyInput::ShiftLeft),
                                KeyCode::ArrowRight if shift => Some(KeyInput::ShiftRight),
                                KeyCode::ArrowLeft => Some(KeyInput::Left),
                                KeyCode::ArrowRight => Some(KeyInput::Right),
                                KeyCode::Home if shift => Some(KeyInput::ShiftHome),
                                KeyCode::End if shift => Some(KeyInput::ShiftEnd),
                                KeyCode::Home => Some(KeyInput::Home),
                                KeyCode::End => Some(KeyInput::End),
                                KeyCode::Enter => Some(KeyInput::Enter),
                                KeyCode::Escape => Some(KeyInput::Escape),
                                KeyCode::KeyA if ctrl => Some(KeyInput::SelectAll),
                                KeyCode::KeyC if ctrl => Some(KeyInput::Copy),
                                KeyCode::KeyX if ctrl => Some(KeyInput::Cut),
                                KeyCode::KeyV if ctrl => {
                                    // 从剪贴板获取文本
                                    let text = self.clipboard.as_mut()
                                        .and_then(|cb| cb.get_text().ok())
                                        .unwrap_or_default();
                                    Some(KeyInput::Paste(text))
                                }
                                _ => { handled = false; None }
                            }
                        } else {
                            handled = false;
                            None
                        };
                        
                        if let Some(ki) = key_input {
                            if let Some(result) = self.interaction.handle_key_input(ki) {
                                self.handle_interaction_result(result);
                            }
                            handled = true;
                        }
                        
                        // 处理文本输入（包括 ASCII 字符）- 但不处理 Ctrl 组合键
                        if !ctrl {
                            if let Some(ref text) = event.text {
                                for c in text.chars() {
                                    if c.is_control() { continue; }
                                    if let Some(result) = self.interaction.handle_key_input(KeyInput::Char(c)) {
                                        self.handle_interaction_result(result);
                                    }
                                }
                                handled = true;
                            }
                        }
                        
                        if handled {
                            self.needs_redraw = true;
                            if let Some(w) = &self.window { w.request_redraw(); }
                            return;
                        }
                    }
                    
                    // 默认键盘处理
                    if let PhysicalKey::Code(code) = event.physical_key {
                        match code {
                            KeyCode::Escape => {
                                if self.interaction.has_focused_input() {
                                    if let Some(result) = self.interaction.blur_input() {
                                        self.handle_interaction_result(result);
                                    }
                                    self.needs_redraw = true;
                                } else {
                                    event_loop.exit();
                                }
                            }
                            KeyCode::Backspace => {
                                if !self.interaction.has_focused_input() {
                                    self.pending_navigation = Some(NavigationRequest::NavigateBack);
                                }
                            }
                            KeyCode::ArrowUp => self.scroll.handle_wheel(8.0),
                            KeyCode::ArrowDown => self.scroll.handle_wheel(-8.0),
                            KeyCode::PageUp => self.scroll.handle_wheel(30.0),
                            KeyCode::PageDown => self.scroll.handle_wheel(-30.0),
                            _ => {}
                        }
                        if let Some(w) = &self.window { w.request_redraw(); }
                    }
                }
            }
            
            // IME 输入支持（中文等）
            WindowEvent::Ime(ime) => {
                use winit::event::Ime;
                match ime {
                    Ime::Commit(text) => {
                        if self.interaction.has_focused_input() {
                            for c in text.chars() {
                                if let Some(result) = self.interaction.handle_key_input(KeyInput::Char(c)) {
                                    self.handle_interaction_result(result);
                                }
                            }
                            self.needs_redraw = true;
                            if let Some(w) = &self.window { w.request_redraw(); }
                        }
                    }
                    Ime::Preedit(text, cursor) => {
                        // 预编辑文本（输入法候选）- 可以显示但暂不处理
                        if !text.is_empty() {
                            println!("📝 IME Preedit: {} {:?}", text, cursor);
                        }
                    }
                    Ime::Enabled => {
                        println!("📝 IME Enabled");
                    }
                    Ime::Disabled => {
                        println!("📝 IME Disabled");
                    }
                }
            }
            
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.setup_canvas(scale_factor);
                self.update_renderers();
                self.render(); // 立即渲染以填充 event_bindings
                self.needs_redraw = false;
            }
            
            WindowEvent::CursorMoved { position, .. } => {
                let x = position.x as f32 / self.scale_factor as f32;
                let y = position.y as f32 / self.scale_factor as f32;
                self.mouse_pos = (x, y);
                
                // 处理滑块拖动
                if self.interaction.is_dragging_slider() {
                    if let Some(result) = self.interaction.handle_mouse_move(x, y + self.scroll.get_position()) {
                        self.handle_interaction_result(result);
                    }
                    self.needs_redraw = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                } else if self.scroll.is_dragging {
                    self.scroll.update_drag(y);
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
            }
            
            WindowEvent::MouseWheel { delta, .. } => {
                let delta_y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * 20.0,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.5,
                };
                self.scroll.handle_wheel(delta_y);
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                match state {
                    ElementState::Pressed => {
                        self.click_start_pos = self.mouse_pos;
                        self.click_start_time = Instant::now();
                        
                        let x = self.mouse_pos.0;
                        let y = self.mouse_pos.1;
                        let actual_y = y + self.scroll.get_position();
                        
                        // 检查是否点击了交互元素
                        if let Some(element) = self.interaction.hit_test(x, actual_y) {
                            let element = element.clone();
                            
                            match element.interaction_type {
                                mini_render::ui::interaction::InteractionType::Slider => {
                                    // 开始滑块拖动
                                    if !element.disabled {
                                        if let Some(result) = self.interaction.handle_click(x, actual_y) {
                                            self.handle_interaction_result(result);
                                            self.needs_redraw = true;
                                            if let Some(w) = &self.window { w.request_redraw(); }
                                        }
                                    }
                                    return;
                                }
                                mini_render::ui::interaction::InteractionType::Button => {
                                    // 设置按钮按下状态
                                    if !element.disabled {
                                        self.interaction.set_button_pressed(element.id.clone(), element.bounds);
                                        self.needs_redraw = true;
                                        if let Some(w) = &self.window { w.request_redraw(); }
                                    }
                                }
                                _ => {}
                            }
                        }
                        
                        // 如果不是在拖动滑块，才开始滚动拖动
                        if !self.interaction.is_dragging_slider() {
                            self.scroll.begin_drag(self.mouse_pos.1);
                        }
                    }
                    ElementState::Released => {
                        // 清除按钮按下状态
                        self.interaction.clear_button_pressed();
                        
                        // 结束滑块拖动
                        if let Some(result) = self.interaction.handle_mouse_release() {
                            self.handle_interaction_result(result);
                        }
                        
                        let needs_animation = self.scroll.end_drag();
                        let dx = (self.mouse_pos.0 - self.click_start_pos.0).abs();
                        let dy = (self.mouse_pos.1 - self.click_start_pos.1).abs();
                        let duration = self.click_start_time.elapsed().as_millis();
                        
                        // 如果是短点击且移动距离小，则处理点击事件
                        if dx < 10.0 && dy < 10.0 && duration < 300 {
                            self.handle_click(self.mouse_pos.0, self.mouse_pos.1);
                        }
                        
                        self.needs_redraw = true;
                        if let Some(w) = &self.window { w.request_redraw(); }
                        
                        if needs_animation {
                            if let Some(w) = &self.window { w.request_redraw(); }
                        }
                    }
                }
            }
            
            WindowEvent::RedrawRequested => {
                self.update_scroll();
                self.process_navigation();
                
                // 检查是否有视频正在播放
                let has_video = mini_render::renderer::components::has_playing_video();
                
                if self.needs_redraw || has_video {
                    self.render();
                    self.needs_redraw = false;
                }
                self.present();
                
                // 如果有滚动动画或视频播放，继续请求重绘
                if self.scroll.is_animating() || self.scroll.is_dragging || has_video {
                    if let Some(window) = &self.window { window.request_redraw(); }
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Mini App Engine - Multi-page Navigation\n");
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = MiniAppWindow::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
