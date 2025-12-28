//! 带窗口的小程序运行器 - 支持多页面导航

use mini_render::runtime::MiniApp;
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::renderer::WxmlRenderer;
use mini_render::{Canvas, Color};
use serde_json::json;
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
const TABBAR_HEIGHT: u32 = 60;

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
    tabbar_nodes: Vec<mini_render::parser::wxml::WxmlNode>,
    stylesheet: mini_render::parser::wxss::StyleSheet,
    has_tabbar: bool,
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
    renderer: Option<WxmlRenderer>,
    tabbar_renderer: Option<WxmlRenderer>,
    // 页面栈
    page_stack: Vec<PageInstance>,
    current_page_index: usize,
    // 页面资源
    pages: HashMap<String, PageInfo>,
    mouse_pos: (f32, f32),
    needs_redraw: bool,
    scale_factor: f64,
    scroll: ScrollController,
    last_frame: Instant,
    click_start_pos: (f32, f32),
    click_start_time: Instant,
    // 导航请求
    pending_navigation: Option<NavigationRequest>,
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
        
        // 加载所有页面资源
        let mut pages = HashMap::new();
        
        // Index 页面
        pages.insert("pages/index/index".to_string(), PageInfo {
            path: "pages/index/index".to_string(),
            wxml: include_str!("../../sample-app/pages/index/index.wxml").to_string(),
            wxss: include_str!("../../sample-app/pages/index/index.wxss").to_string(),
            js: include_str!("../../sample-app/pages/index/index.js").to_string(),
        });
        
        // List 页面
        pages.insert("pages/list/list".to_string(), PageInfo {
            path: "pages/list/list".to_string(),
            wxml: include_str!("../../sample-app/pages/list/list.wxml").to_string(),
            wxss: include_str!("../../sample-app/pages/list/list.wxss").to_string(),
            js: include_str!("../../sample-app/pages/list/list.js").to_string(),
        });
        
        // Detail 页面
        pages.insert("pages/detail/detail".to_string(), PageInfo {
            path: "pages/detail/detail".to_string(),
            wxml: include_str!("../../sample-app/pages/detail/detail.wxml").to_string(),
            wxss: include_str!("../../sample-app/pages/detail/detail.wxss").to_string(),
            js: include_str!("../../sample-app/pages/detail/detail.js").to_string(),
        });
        
        let now = Instant::now();
        let mut window = Self {
            window: None,
            surface: None,
            app,
            canvas: None,
            tabbar_canvas: None,
            renderer: None,
            tabbar_renderer: None,
            page_stack: Vec::new(),
            current_page_index: 0,
            pages,
            mouse_pos: (0.0, 0.0),
            needs_redraw: true,
            scale_factor: 1.0,
            scroll: ScrollController::new(CONTENT_HEIGHT as f32, (LOGICAL_HEIGHT - TABBAR_HEIGHT) as f32),
            last_frame: now,
            click_start_pos: (0.0, 0.0),
            click_start_time: now,
            pending_navigation: None,
        };
        
        // 加载首页
        window.navigate_to("pages/index/index", HashMap::new())?;
        
        Ok(window)
    }

    fn navigate_to(&mut self, path: &str, query: HashMap<String, String>) -> Result<(), String> {
        let path = path.trim_start_matches('/');
        println!("📄 Navigate to: {} {:?}", path, query);
        
        let page_info = self.pages.get(path)
            .ok_or_else(|| format!("Page not found: {}", path))?;
        
        // 解析 WXML
        let mut wxml_parser = WxmlParser::new(&page_info.wxml);
        let all_nodes = wxml_parser.parse().map_err(|e| format!("WXML parse error: {}", e))?;
        let (wxml_nodes, tabbar_nodes) = Self::separate_tabbar(&all_nodes);
        let has_tabbar = !tabbar_nodes.is_empty();
        
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
            tabbar_nodes,
            stylesheet,
            has_tabbar,
        };
        
        self.page_stack.push(page_instance);
        self.current_page_index = self.page_stack.len() - 1;
        self.scroll = ScrollController::new(
            CONTENT_HEIGHT as f32, 
            (LOGICAL_HEIGHT - if has_tabbar { TABBAR_HEIGHT } else { 0 }) as f32
        );
        self.needs_redraw = true;
        
        println!("✅ Page loaded: {} (stack size: {})", path, self.page_stack.len());
        Ok(())
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
            
            let has_tabbar = page.has_tabbar;
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

    fn separate_tabbar(nodes: &[mini_render::parser::wxml::WxmlNode]) -> (Vec<mini_render::parser::wxml::WxmlNode>, Vec<mini_render::parser::wxml::WxmlNode>) {
        let mut content_nodes = Vec::new();
        let mut tabbar_nodes = Vec::new();
        for node in nodes {
            let (content, tabbar) = Self::separate_node(node);
            content_nodes.extend(content);
            tabbar_nodes.extend(tabbar);
        }
        (content_nodes, tabbar_nodes)
    }
    
    fn separate_node(node: &mini_render::parser::wxml::WxmlNode) -> (Vec<mini_render::parser::wxml::WxmlNode>, Vec<mini_render::parser::wxml::WxmlNode>) {
        use mini_render::parser::wxml::{WxmlNode, WxmlNodeType};
        if node.node_type != WxmlNodeType::Element {
            return (vec![node.clone()], vec![]);
        }
        let class = node.attributes.get("class").map(|s| s.as_str()).unwrap_or("");
        if class.contains("tabbar") && !class.contains("tabbar-placeholder") {
            return (vec![], vec![node.clone()]);
        }
        if class.contains("tabbar-placeholder") {
            return (vec![], vec![]);
        }
        let mut new_children = Vec::new();
        let mut tabbar_nodes = Vec::new();
        for child in &node.children {
            let (content, tabbar) = Self::separate_node(child);
            new_children.extend(content);
            tabbar_nodes.extend(tabbar);
        }
        let mut new_node = WxmlNode::new_element(&node.tag_name);
        new_node.attributes = node.attributes.clone();
        new_node.children = new_children;
        (vec![new_node], tabbar_nodes)
    }
    
    fn setup_canvas(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
        let physical_width = (LOGICAL_WIDTH as f64 * scale_factor) as u32;
        let physical_height = (CONTENT_HEIGHT as f64 * scale_factor) as u32;
        let tabbar_physical_height = (TABBAR_HEIGHT as f64 * scale_factor) as u32;
        
        println!("📐 Scale: {}x | Content: {}x{}", scale_factor, LOGICAL_WIDTH, CONTENT_HEIGHT);
        
        self.canvas = Some(Canvas::new(physical_width, physical_height));
        self.tabbar_canvas = Some(Canvas::new(physical_width, tabbar_physical_height));
    }
    
    fn update_renderers(&mut self) {
        if let Some(page) = self.page_stack.last() {
            self.renderer = Some(WxmlRenderer::new_with_scale(
                page.stylesheet.clone(),
                LOGICAL_WIDTH as f32,
                CONTENT_HEIGHT as f32,
                self.scale_factor as f32,
            ));
            self.tabbar_renderer = Some(WxmlRenderer::new_with_scale(
                page.stylesheet.clone(),
                LOGICAL_WIDTH as f32,
                TABBAR_HEIGHT as f32,
                self.scale_factor as f32,
            ));
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
        
        // 渲染内容区域
        if let (Some(canvas), Some(renderer)) = (&mut self.canvas, &mut self.renderer) {
            canvas.clear(Color::from_hex(0xF5F5F5));
            renderer.render(canvas, &page.wxml_nodes, &page_data);
        }
        
        // 渲染 TabBar（如果有）
        if page.has_tabbar {
            if let (Some(canvas), Some(renderer)) = (&mut self.tabbar_canvas, &mut self.tabbar_renderer) {
                canvas.clear(Color::WHITE);
                renderer.render(canvas, &page.tabbar_nodes, &page_data);
            }
        }
    }
    
    fn present(&mut self) {
        let canvas = match &self.canvas { Some(c) => c, None => return };
        let page = match self.page_stack.last() { Some(p) => p, None => return };
        
        if let (Some(window), Some(surface)) = (&self.window, &mut self.surface) {
            let size = window.inner_size();
            if let (Some(win_width), Some(win_height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
                surface.resize(win_width, win_height).ok();
                
                if let Ok(mut buffer) = surface.buffer_mut() {
                    let canvas_data = canvas.to_rgba();
                    let canvas_width = canvas.width();
                    let canvas_height = canvas.height();
                    let scroll_offset = (self.scroll.get_position() * self.scale_factor as f32) as i32;
                    
                    let tabbar_physical_height = if page.has_tabbar { (TABBAR_HEIGHT as f64 * self.scale_factor) as u32 } else { 0 };
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
                    
                    // 渲染 TabBar
                    if page.has_tabbar {
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
        let tabbar_y = if page.has_tabbar { (LOGICAL_HEIGHT - TABBAR_HEIGHT) as f32 } else { LOGICAL_HEIGHT as f32 };
        
        if page.has_tabbar && y >= tabbar_y {
            // TabBar 点击
            let tabbar_local_y = y - tabbar_y;
            if let Some(renderer) = &self.tabbar_renderer {
                if let Some(binding) = renderer.hit_test(x, tabbar_local_y) {
                    println!("👆 TabBar {} -> {}", binding.event_type, binding.handler);
                    let data_json = serde_json::to_string(&binding.data).unwrap_or("{}".to_string());
                    let call_code = format!("__callPageMethod('{}', {})", binding.handler, data_json);
                    self.app.eval(&call_code).ok();
                    self.check_navigation();
                    self.print_js_output();
                    self.scroll = ScrollController::new(CONTENT_HEIGHT as f32, (LOGICAL_HEIGHT - TABBAR_HEIGHT) as f32);
                    self.needs_redraw = true;
                }
            }
        } else {
            // 内容区域点击
            let actual_y = y + self.scroll.get_position();
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
            
            WindowEvent::KeyboardInput { event, .. } => {
                use winit::keyboard::{PhysicalKey, KeyCode};
                if event.state == ElementState::Pressed {
                    if let PhysicalKey::Code(code) = event.physical_key {
                        match code {
                            KeyCode::Escape => event_loop.exit(),
                            KeyCode::Backspace => {
                                self.pending_navigation = Some(NavigationRequest::NavigateBack);
                                if let Some(w) = &self.window { w.request_redraw(); }
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
            
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.setup_canvas(scale_factor);
                self.update_renderers();
                self.needs_redraw = true;
            }
            
            WindowEvent::CursorMoved { position, .. } => {
                let x = position.x as f32 / self.scale_factor as f32;
                let y = position.y as f32 / self.scale_factor as f32;
                self.mouse_pos = (x, y);
                if self.scroll.is_dragging {
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
                        self.scroll.begin_drag(self.mouse_pos.1);
                    }
                    ElementState::Released => {
                        let needs_animation = self.scroll.end_drag();
                        let dx = (self.mouse_pos.0 - self.click_start_pos.0).abs();
                        let dy = (self.mouse_pos.1 - self.click_start_pos.1).abs();
                        let duration = self.click_start_time.elapsed().as_millis();
                        if dx < 10.0 && dy < 10.0 && duration < 200 {
                            self.handle_click(self.mouse_pos.0, self.mouse_pos.1);
                        }
                        if needs_animation {
                            if let Some(w) = &self.window { w.request_redraw(); }
                        }
                    }
                }
            }
            
            WindowEvent::RedrawRequested => {
                self.update_scroll();
                self.process_navigation();
                if self.needs_redraw {
                    self.render();
                    self.needs_redraw = false;
                }
                self.present();
                if self.scroll.is_animating() || self.scroll.is_dragging {
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
