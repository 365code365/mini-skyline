//! 带窗口的小程序运行器 - 支持多页面导航和原生 TabBar

mod app_window;

use app_window::*;
use app_window::events::{keyboard, mouse, ime};

use mini_render::runtime::{MiniApp, UiEvent};
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::renderer::WxmlRenderer;
use mini_render::ui::interaction::InteractionManager;
use mini_render::{Canvas, Color, Paint, PaintStyle, Rect as GeoRect};
use mini_render::text::TextRenderer;
use serde_json::json;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;
use std::collections::HashMap;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};
use mini_render::ui::ScrollController;

/// Toast 状态
#[derive(Clone)]
struct ToastState {
    title: String,
    icon: String,
    visible: bool,
    start_time: Instant,
    duration_ms: u32,
}

/// Loading 状态
#[derive(Clone)]
struct LoadingState {
    title: String,
    visible: bool,
}

/// Modal 状态
#[derive(Clone)]
struct ModalState {
    title: String,
    content: String,
    show_cancel: bool,
    cancel_text: String,
    confirm_text: String,
    visible: bool,
}

struct MiniAppWindow {
    window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    app: MiniApp,
    canvas: Option<Canvas>,
    tabbar_canvas: Option<Canvas>,
    fixed_canvas: Option<Canvas>,
    renderer: Option<WxmlRenderer>,
    tabbar_renderer: Option<WxmlRenderer>,
    text_renderer: Option<TextRenderer>,
    page_stack: Vec<PageInstance>,
    current_page_index: usize,
    pages: HashMap<String, PageInfo>,
    app_config: AppConfig,
    custom_tabbar: Option<CustomTabBar>,
    mouse_pos: (f32, f32),
    needs_redraw: bool,
    scale_factor: f64,
    scroll: ScrollController,
    last_frame: Instant,
    click_start_pos: (f32, f32),
    click_start_time: Instant,
    pending_navigation: Option<NavigationRequest>,
    interaction: InteractionManager,
    modifiers: winit::keyboard::ModifiersState,
    clipboard: Option<arboard::Clipboard>,
    // UI 状态
    toast: Option<ToastState>,
    loading: Option<LoadingState>,
    modal: Option<ModalState>,
}

impl MiniAppWindow {
    fn new() -> Result<Self, String> {
        let mut app = MiniApp::new(LOGICAL_WIDTH, LOGICAL_HEIGHT)?;
        app.init()?;
        
        // 加载 app.js（全局 App 实例）
        let app_js = include_str!("../../sample-app/app.js");
        app.load_script(app_js)?;
        println!("📱 App.js loaded");
        
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
        
        let custom_tabbar = if app_config.tab_bar.as_ref().map(|tb| tb.custom).unwrap_or(false) {
            Self::load_custom_tabbar()?
        } else {
            None
        };
        
        if custom_tabbar.is_some() {
            println!("   ✅ Custom TabBar loaded");
        }
        
        let pages = Self::load_all_pages();
        let has_tabbar = app_config.tab_bar.as_ref()
            .map(|tb| tb.list.iter().any(|item| item.page_path == "pages/index/index"))
            .unwrap_or(false);
        
        let now = Instant::now();
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
            toast: None,
            loading: None,
            modal: None,
        };
        
        window.navigate_to("pages/index/index", HashMap::new())?;
        Ok(window)
    }
    
    fn load_all_pages() -> HashMap<String, PageInfo> {
        let mut pages = HashMap::new();
        
        pages.insert("pages/index/index".to_string(), PageInfo {
            path: "pages/index/index".to_string(),
            wxml: include_str!("../../sample-app/pages/index/index.wxml").to_string(),
            wxss: include_str!("../../sample-app/pages/index/index.wxss").to_string(),
            js: include_str!("../../sample-app/pages/index/index.js").to_string(),
        });
        
        pages.insert("pages/category/category".to_string(), PageInfo {
            path: "pages/category/category".to_string(),
            wxml: include_str!("../../sample-app/pages/category/category.wxml").to_string(),
            wxss: include_str!("../../sample-app/pages/category/category.wxss").to_string(),
            js: include_str!("../../sample-app/pages/category/category.js").to_string(),
        });
        
        pages.insert("pages/cart/cart".to_string(), PageInfo {
            path: "pages/cart/cart".to_string(),
            wxml: include_str!("../../sample-app/pages/cart/cart.wxml").to_string(),
            wxss: include_str!("../../sample-app/pages/cart/cart.wxss").to_string(),
            js: include_str!("../../sample-app/pages/cart/cart.js").to_string(),
        });
        
        pages.insert("pages/profile/profile".to_string(), PageInfo {
            path: "pages/profile/profile".to_string(),
            wxml: include_str!("../../sample-app/pages/profile/profile.wxml").to_string(),
            wxss: include_str!("../../sample-app/pages/profile/profile.wxss").to_string(),
            js: include_str!("../../sample-app/pages/profile/profile.js").to_string(),
        });
        
        pages.insert("pages/detail/detail".to_string(), PageInfo {
            path: "pages/detail/detail".to_string(),
            wxml: include_str!("../../sample-app/pages/detail/detail.wxml").to_string(),
            wxss: include_str!("../../sample-app/pages/detail/detail.wxss").to_string(),
            js: include_str!("../../sample-app/pages/detail/detail.js").to_string(),
        });
        
        pages
    }
    
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
    
    fn is_tabbar_page(&self, path: &str) -> bool {
        self.app_config.tab_bar.as_ref()
            .map(|tb| tb.list.iter().any(|item| item.page_path == path))
            .unwrap_or(false)
    }
    
    fn get_tabbar_index(&self, path: &str) -> Option<usize> {
        self.app_config.tab_bar.as_ref()
            .and_then(|tb| tb.list.iter().position(|item| item.page_path == path))
    }
    
    fn is_custom_tabbar(&self) -> bool {
        self.app_config.tab_bar.as_ref().map(|tb| tb.custom).unwrap_or(false) 
            && self.custom_tabbar.is_some()
    }

    fn navigate_to(&mut self, path: &str, query: HashMap<String, String>) -> Result<(), String> {
        let path = path.trim_start_matches('/');
        println!("📄 Navigate to: {} {:?}", path, query);
        
        let page_info = self.pages.get(path)
            .ok_or_else(|| format!("Page not found: {}", path))?;
        
        let mut wxml_parser = WxmlParser::new(&page_info.wxml);
        let all_nodes = wxml_parser.parse().map_err(|e| format!("WXML parse error: {}", e))?;
        let wxml_nodes = remove_manual_tabbar(&all_nodes);
        
        let mut wxss_parser = WxssParser::new(&page_info.wxss);
        let stylesheet = wxss_parser.parse().map_err(|e| format!("WXSS parse error: {}", e))?;
        
        self.app.load_script(&page_info.js)?;
        
        // 调用 onLoad
        let query_json = serde_json::to_string(&query).unwrap_or("{}".to_string());
        let load_code = format!("if(__currentPage && __currentPage.onLoad) __currentPage.onLoad({})", query_json);
        self.app.eval(&load_code).ok();
        print_js_output(&self.app);
        
        // 调用 onShow
        let show_code = "if(__currentPage && __currentPage.onShow) __currentPage.onShow()";
        self.app.eval(show_code).ok();
        print_js_output(&self.app);
        
        let page_instance = PageInstance {
            path: path.to_string(),
            query,
            wxml_nodes,
            stylesheet,
        };
        
        self.page_stack.push(page_instance);
        self.current_page_index = self.page_stack.len() - 1;
        
        let has_tabbar = self.is_tabbar_page(path);
        self.scroll = ScrollController::new(
            CONTENT_HEIGHT as f32, 
            (LOGICAL_HEIGHT - if has_tabbar { TABBAR_HEIGHT } else { 0 }) as f32
        );
        self.needs_redraw = true;
        
        println!("✅ Page loaded: {} (stack size: {}, tabbar: {})", path, self.page_stack.len(), has_tabbar);
        Ok(())
    }
    
    fn navigate_back(&mut self) -> Result<(), String> {
        if self.page_stack.len() <= 1 {
            println!("⚠️ Already at root page");
            return Ok(());
        }
        
        self.page_stack.pop();
        self.current_page_index = self.page_stack.len() - 1;
        
        if let Some(page) = self.page_stack.last() {
            let path = page.path.clone();
            let query = page.query.clone();
            if let Some(page_info) = self.pages.get(&path) {
                self.app.load_script(&page_info.js)?;
                let query_json = serde_json::to_string(&query).unwrap_or("{}".to_string());
                let load_code = format!("if(__currentPage && __currentPage.onLoad) __currentPage.onLoad({})", query_json);
                self.app.eval(&load_code).ok();
                print_js_output(&self.app);
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
        self.page_stack.clear();
        self.interaction.clear_page_state();
        self.navigate_to(path, HashMap::new())
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
        
        self.text_renderer = TextRenderer::load_system_font()
            .or_else(|_| TextRenderer::from_bytes(include_bytes!("../../assets/ArialUnicode.ttf")))
            .ok();
    }
    
    fn update_renderers(&mut self) {
        if let Some(page) = self.page_stack.last() {
            // screen_height 应该是视口高度，用于 fixed 元素定位
            self.renderer = Some(WxmlRenderer::new_with_scale(
                page.stylesheet.clone(),
                LOGICAL_WIDTH as f32,
                LOGICAL_HEIGHT as f32,  // 使用视口高度，不是内容高度
                self.scale_factor as f32,
            ));
            
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
        // 获取页面数据
        let page_data = if let Ok(data_str) = self.app.eval("__getPageData()") {
            serde_json::from_str(&data_str).unwrap_or(json!({}))
        } else {
            json!({})
        };
        
        let page = match self.page_stack.last() {
            Some(p) => p,
            None => return,
        };
        
        let current_path = &page.path;
        let has_tabbar = self.is_tabbar_page(current_path);
        let viewport_height = (LOGICAL_HEIGHT - if has_tabbar { TABBAR_HEIGHT } else { 0 }) as f32;
        let scroll_offset = self.scroll.get_position();
        
        // 渲染主内容
        let mut content_height = 0.0f32;
        if let Some(canvas) = &mut self.canvas {
            canvas.clear(Color::from_hex(0xF5F5F5));
            if let Some(renderer) = &mut self.renderer {
                content_height = renderer.render_with_scroll_and_viewport(
                    canvas, &page.wxml_nodes, &page_data, 
                    &mut self.interaction, scroll_offset, viewport_height
                );
            }
        }
        
        let current_path = current_path.clone();
        
        if content_height > 0.0 {
            self.scroll.update_content_height(content_height, viewport_height);
            
            // canvas 高度 = 内容高度
            let required_height = (content_height * self.scale_factor as f32).ceil() as u32;
            let current_height = self.canvas.as_ref().map(|c| c.height()).unwrap_or(0);
            
            if current_height != required_height && required_height > 0 {
                let physical_width = (LOGICAL_WIDTH as f64 * self.scale_factor) as u32;
                self.canvas = Some(Canvas::new(physical_width, required_height));
                
                // 重新渲染到新 canvas
                if let Some(page) = self.page_stack.last() {
                    if let Some(canvas) = &mut self.canvas {
                        canvas.clear(Color::from_hex(0xF5F5F5));
                        if let Some(renderer) = &mut self.renderer {
                            renderer.render_with_scroll_and_viewport(
                                canvas, &page.wxml_nodes, &page_data, 
                                &mut self.interaction, scroll_offset, viewport_height
                            );
                        }
                    }
                }
            }
        }
        
        // 渲染 fixed 元素
        if let Some(page) = self.page_stack.last() {
            if let Some(fixed_canvas) = &mut self.fixed_canvas {
                fixed_canvas.clear(Color::new(0, 0, 0, 0));
                if let Some(renderer) = &mut self.renderer {
                    renderer.render_fixed_elements(fixed_canvas, &page.wxml_nodes, &page_data, &mut self.interaction, viewport_height);
                }
            }
        }
        
        // 渲染 tabbar
        if has_tabbar {
            if self.is_custom_tabbar() {
                self.render_custom_tabbar(&current_path);
            } else {
                self.render_native_tabbar(&current_path);
            }
        }
    }
    
    fn render_custom_tabbar(&mut self, current_path: &str) {
        let tab_bar_config = match &self.app_config.tab_bar {
            Some(tb) => tb.clone(),
            None => return,
        };
        
        let selected_index = self.get_tabbar_index(current_path).unwrap_or(0);
        let list: Vec<serde_json::Value> = tab_bar_config.list.iter().map(|item| {
            json!({ "pagePath": item.page_path, "text": item.text })
        }).collect();
        
        let tabbar_data = json!({ "selected": selected_index, "list": list });
        
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
        
        canvas.clear(Color::WHITE);
        renderer.render(canvas, &wxml_nodes, &tabbar_data);
    }
    
    fn render_native_tabbar(&mut self, current_path: &str) {
        let tab_bar = match &self.app_config.tab_bar {
            Some(tb) => tb.clone(),
            None => return,
        };
        
        let canvas = match &mut self.tabbar_canvas {
            Some(c) => c,
            None => return,
        };
        
        let text_renderer = match &self.text_renderer {
            Some(tr) => tr,
            None => return,
        };
        
        render_native_tabbar(canvas, text_renderer, &tab_bar, current_path, self.scale_factor);
    }
    
    fn present(&mut self) {
        let canvas = match &self.canvas { Some(c) => c, None => return };
        let page = match self.page_stack.last() { Some(p) => p, None => return };
        let has_tabbar = self.is_tabbar_page(&page.path);
        
        // 收集 UI 状态（避免借用冲突）
        let toast_state = self.toast.clone();
        let loading_state = self.loading.clone();
        let modal_state = self.modal.clone();
        let sf = self.scale_factor as f32;
        let last_frame = self.last_frame;
        
        if let (Some(window), Some(surface)) = (&self.window, &mut self.surface) {
            let size = window.inner_size();
            if let (Some(win_width), Some(win_height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
                surface.resize(win_width, win_height).ok();
                
                if let Ok(mut buffer) = surface.buffer_mut() {
                    let scroll_offset = (self.scroll.get_position() * self.scale_factor as f32) as i32;
                    let tabbar_physical_height = if has_tabbar { (TABBAR_HEIGHT as f64 * self.scale_factor) as u32 } else { 0 };
                    
                    present_to_buffer(
                        &mut buffer,
                        size.width,
                        size.height,
                        canvas,
                        self.fixed_canvas.as_ref(),
                        self.tabbar_canvas.as_ref(),
                        scroll_offset,
                        has_tabbar,
                        tabbar_physical_height,
                    );
                    
                    // 渲染 UI 覆盖层（Toast/Loading/Modal）
                    render_ui_overlay(
                        &mut buffer, size.width, size.height, sf, last_frame,
                        &toast_state, &loading_state, &modal_state, self.text_renderer.as_ref()
                    );
                    
                    buffer.present().ok();
                }
            }
        }
    }
    
    fn handle_click(&mut self, x: f32, y: f32) {
        // 如果有 Modal 显示，处理 Modal 点击
        if let Some(modal) = &self.modal {
            if modal.visible {
                self.handle_modal_click(x, y);
                return;
            }
        }
        
        // 如果有 Loading 显示，忽略点击
        if let Some(loading) = &self.loading {
            if loading.visible {
                return;
            }
        }
        
        let page = match self.page_stack.last() { Some(p) => p, None => return };
        let has_tabbar = self.is_tabbar_page(&page.path);
        let tabbar_y = if has_tabbar { (LOGICAL_HEIGHT - TABBAR_HEIGHT) as f32 } else { LOGICAL_HEIGHT as f32 };
        
        if has_tabbar && y >= tabbar_y {
            if self.is_custom_tabbar() {
                self.handle_custom_tabbar_click(x, y - tabbar_y);
            } else {
                self.handle_native_tabbar_click(x);
            }
        } else {
            let scroll_pos = self.scroll.get_position();
            if let Some(result) = mouse::handle_content_click(
                x, y, scroll_pos, has_tabbar,
                &mut self.interaction,
                self.renderer.as_ref(),
                &mut self.app,
                self.scale_factor,
                self.text_renderer.as_ref(),
            ) {
                handle_interaction_result(
                    &result,
                    self.window.as_ref(),
                    self.renderer.as_ref(),
                    &mut self.app,
                    &mut self.clipboard,
                    scroll_pos,
                    self.scale_factor,
                );
            }
            
            if let Some(nav) = check_navigation(&mut self.app) {
                self.pending_navigation = Some(nav);
            }
            print_js_output(&self.app);
            self.needs_redraw = true;
        }
    }
    
    /// 处理 Modal 点击
    fn handle_modal_click(&mut self, x: f32, y: f32) {
        let sf = self.scale_factor as f32;
        let modal_width = (280.0 * sf) as f32;
        let modal_padding = 20.0 * sf;
        let title_height = 22.0 * sf;
        let content_height = 44.0 * sf;
        let button_height = 44.0 * sf;
        let modal_height = modal_padding * 2.0 + title_height + content_height + button_height + 20.0 * sf;
        
        let modal_x = (LOGICAL_WIDTH as f32 - modal_width / sf) / 2.0;
        let modal_y = (LOGICAL_HEIGHT as f32 - modal_height / sf) / 2.0;
        let button_y = modal_y + modal_height / sf - button_height / sf;
        
        // 检查是否点击在按钮区域
        if y >= button_y && y <= button_y + button_height / sf {
            if x >= modal_x && x <= modal_x + modal_width / sf {
                let show_cancel = self.modal.as_ref().map(|m| m.show_cancel).unwrap_or(false);
                
                if show_cancel {
                    let button_width = modal_width / sf / 2.0;
                    if x < modal_x + button_width {
                        // 点击取消按钮
                        println!("Modal: 取消");
                        self.app.eval("if(__modalCallback) __modalCallback({ confirm: false, cancel: true })").ok();
                    } else {
                        // 点击确认按钮
                        println!("Modal: 确认");
                        self.app.eval("if(__modalCallback) __modalCallback({ confirm: true, cancel: false })").ok();
                    }
                } else {
                    // 只有确认按钮
                    println!("Modal: 确认");
                    self.app.eval("if(__modalCallback) __modalCallback({ confirm: true, cancel: false })").ok();
                }
                
                // 关闭 Modal
                self.modal = None;
                self.needs_redraw = true;
                if let Some(w) = &self.window { w.request_redraw(); }
            }
        }
    }
    
    fn handle_custom_tabbar_click(&mut self, x: f32, y: f32) {
        if let Some(renderer) = &self.tabbar_renderer {
            if let Some(binding) = renderer.hit_test(x, y) {
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
    
    fn handle_native_tabbar_click(&mut self, x: f32) {
        if let Some(tab_bar) = &self.app_config.tab_bar {
            let current_path = self.page_stack.last().map(|p| p.path.clone()).unwrap_or_default();
            if let Some(target_path) = handle_native_tabbar_click(tab_bar, x, &current_path) {
                self.pending_navigation = Some(NavigationRequest::SwitchTab { url: target_path });
                if let Some(w) = &self.window { w.request_redraw(); }
            }
        }
    }
    
    fn process_navigation(&mut self) {
        if let Some(nav) = self.pending_navigation.take() {
            match nav {
                NavigationRequest::NavigateTo { url } => {
                    let (path, query) = parse_url(&url);
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
                    let (path, _) = parse_url(&url);
                    if let Err(e) = self.switch_tab(&path) {
                        println!("❌ Navigation error: {}", e);
                    }
                    self.update_renderers();
                }
            }
        }
    }
    
    /// 处理 UI 事件（Toast/Loading/Modal）
    fn process_ui_events(&mut self) {
        let events = self.app.drain_ui_events();
        for event in events {
            match event {
                UiEvent::ShowToast { title, icon, duration } => {
                    self.toast = Some(ToastState {
                        title,
                        icon,
                        visible: true,
                        start_time: Instant::now(),
                        duration_ms: duration,
                    });
                    self.needs_redraw = true;
                }
                UiEvent::HideToast => {
                    if let Some(toast) = &mut self.toast {
                        toast.visible = false;
                    }
                    self.needs_redraw = true;
                }
                UiEvent::ShowLoading { title } => {
                    self.loading = Some(LoadingState {
                        title,
                        visible: true,
                    });
                    self.needs_redraw = true;
                }
                UiEvent::HideLoading => {
                    if let Some(loading) = &mut self.loading {
                        loading.visible = false;
                    }
                    self.needs_redraw = true;
                }
                UiEvent::ShowModal { title, content, show_cancel, cancel_text, confirm_text } => {
                    self.modal = Some(ModalState {
                        title,
                        content,
                        show_cancel,
                        cancel_text,
                        confirm_text,
                        visible: true,
                    });
                    self.needs_redraw = true;
                }
                UiEvent::HideModal => {
                    if let Some(modal) = &mut self.modal {
                        modal.visible = false;
                    }
                    self.needs_redraw = true;
                }
            }
        }
    }
    
    /// 更新 Toast 超时
    fn update_toast_timeout(&mut self) {
        if let Some(toast) = &self.toast {
            if toast.visible {
                let elapsed = toast.start_time.elapsed().as_millis() as u32;
                if elapsed >= toast.duration_ms {
                    self.toast = None;
                    self.needs_redraw = true;
                }
            }
        }
    }
    
    fn update_scroll(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        
        let mut scroll_changed = false;
        
        // 使用带事件检测的更新方法
        let (animating, event) = self.scroll.update_with_events(dt);
        if animating {
            scroll_changed = true;
        }
        
        // 处理页面滚动事件
        if let Some(scroll_event) = event {
            self.handle_scroll_event(scroll_event);
        }
        
        for controller in self.interaction.scroll_controllers.values_mut() {
            if controller.update(dt) {
                scroll_changed = true;
            }
        }
        
        // 滚动动画时只需要 present，不需要重新渲染
        if scroll_changed {
            if let Some(window) = &self.window { window.request_redraw(); }
        }
    }
    
    /// 处理滚动事件（触底/触顶）
    fn handle_scroll_event(&mut self, event: mini_render::ui::scroll_controller::ScrollEvent) {
        use mini_render::ui::scroll_controller::ScrollEvent;
        
        match event {
            ScrollEvent::ReachBottom => {
                println!("📜 onReachBottom triggered");
                // 调用页面的 onReachBottom 方法
                let call_code = "if(__currentPage && __currentPage.onReachBottom) __currentPage.onReachBottom()";
                self.app.eval(call_code).ok();
                print_js_output(&self.app);
            }
            ScrollEvent::ReachTop => {
                println!("📜 onPullDownRefresh triggered");
                // 调用页面的 onPullDownRefresh 方法
                let call_code = "if(__currentPage && __currentPage.onPullDownRefresh) __currentPage.onPullDownRefresh()";
                self.app.eval(call_code).ok();
                print_js_output(&self.app);
            }
        }
        
        self.needs_redraw = true;
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
                use winit::keyboard::ModifiersState;
                if event.state == ElementState::Pressed {
                    let ctrl = self.modifiers.contains(ModifiersState::CONTROL) || self.modifiers.contains(ModifiersState::SUPER);
                    
                    // 处理输入框文本输入
                    if self.interaction.has_focused_input() {
                        let (handled, result) = keyboard::handle_keyboard_input(
                            event.physical_key,
                            self.modifiers,
                            &mut self.interaction,
                            &mut self.clipboard,
                        );
                        
                        if let Some(result) = result {
                            handle_interaction_result(
                                &result,
                                self.window.as_ref(),
                                self.renderer.as_ref(),
                                &mut self.app,
                                &mut self.clipboard,
                                self.scroll.get_position(),
                                self.scale_factor,
                            );
                        }
                        
                        // 处理文本输入
                        if !ctrl {
                            if let Some(ref text) = event.text {
                                let results = keyboard::handle_text_input(text, ctrl, &mut self.interaction);
                                for result in results {
                                    handle_interaction_result(
                                        &result,
                                        self.window.as_ref(),
                                        self.renderer.as_ref(),
                                        &mut self.app,
                                        &mut self.clipboard,
                                        self.scroll.get_position(),
                                        self.scale_factor,
                                    );
                                }
                            }
                        }
                        
                        if handled {
                            self.needs_redraw = true;
                            if let Some(w) = &self.window { w.request_redraw(); }
                            return;
                        }
                    }
                    
                    // 默认键盘处理
                    if let Some(action) = keyboard::handle_default_keyboard(event.physical_key, &mut self.interaction) {
                        match action {
                            keyboard::DefaultKeyAction::Exit => event_loop.exit(),
                            keyboard::DefaultKeyAction::NavigateBack => {
                                self.pending_navigation = Some(NavigationRequest::NavigateBack);
                            }
                            keyboard::DefaultKeyAction::BlurInput => {
                                if let Some(result) = self.interaction.blur_input() {
                                    handle_interaction_result(
                                        &result,
                                        self.window.as_ref(),
                                        self.renderer.as_ref(),
                                        &mut self.app,
                                        &mut self.clipboard,
                                        self.scroll.get_position(),
                                        self.scale_factor,
                                    );
                                }
                                self.needs_redraw = true;
                            }
                            keyboard::DefaultKeyAction::ScrollUp => self.scroll.handle_scroll(8.0, false),
                            keyboard::DefaultKeyAction::ScrollDown => self.scroll.handle_scroll(-8.0, false),
                            keyboard::DefaultKeyAction::PageUp => self.scroll.handle_scroll(30.0, false),
                            keyboard::DefaultKeyAction::PageDown => self.scroll.handle_scroll(-30.0, false),
                        }
                        if let Some(w) = &self.window { w.request_redraw(); }
                    }
                }
            }
            
            WindowEvent::Ime(ime_event) => {
                let results = ime::handle_ime_event(ime_event, &mut self.interaction);
                let has_results = !results.is_empty();
                for result in results {
                    handle_interaction_result(
                        &result,
                        self.window.as_ref(),
                        self.renderer.as_ref(),
                        &mut self.app,
                        &mut self.clipboard,
                        self.scroll.get_position(),
                        self.scale_factor,
                    );
                }
                if has_results {
                    self.needs_redraw = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
            }
            
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.setup_canvas(scale_factor);
                self.update_renderers();
                self.render();
                self.needs_redraw = false;
            }
            
            WindowEvent::CursorMoved { position, .. } => {
                let x = position.x as f32 / self.scale_factor as f32;
                let y = position.y as f32 / self.scale_factor as f32;
                self.mouse_pos = (x, y);
                let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
                
                // 处理文本选择拖动
                if self.interaction.is_selecting() {
                    if let Some(focused) = &self.interaction.focused_input {
                        if let Some(tr) = &self.text_renderer {
                            let sf = self.scale_factor as f32;
                            let font_size = 16.0 * sf;
                            let padding_left = 12.0 * sf;
                            let bounds = focused.bounds;
                            let text_offset = focused.text_offset;
                            let click_x = ((x - bounds.x) * sf).max(0.0);
                            
                            let mut char_widths = Vec::new();
                            for c in focused.value.chars() {
                                let char_str = c.to_string();
                                let width = tr.measure_text(&char_str, font_size);
                                char_widths.push(width);
                            }
                            
                            let cursor_pos = mini_render::ui::interaction::calculate_cursor_position(
                                &focused.value, &char_widths, click_x, padding_left, text_offset
                            );
                            
                            self.interaction.update_text_selection(cursor_pos);
                            self.needs_redraw = true;
                            if let Some(w) = &self.window { w.request_redraw(); }
                        }
                    }
                } else if self.interaction.is_dragging_slider() {
                    if let Some(result) = self.interaction.handle_mouse_move(x, y + self.scroll.get_position()) {
                        handle_interaction_result(
                            &result,
                            self.window.as_ref(),
                            self.renderer.as_ref(),
                            &mut self.app,
                            &mut self.clipboard,
                            self.scroll.get_position(),
                            self.scale_factor,
                        );
                    }
                    self.needs_redraw = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                } else if let Some(id) = self.interaction.dragging_scroll_area.clone() {
                    if let Some(controller) = self.interaction.get_scroll_controller_mut(&id) {
                        controller.update_drag(y, timestamp);
                        if let Some(w) = &self.window { w.request_redraw(); }
                    }
                } else if self.scroll.is_dragging {
                    self.scroll.update_drag(y, timestamp);
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
            }
            
            WindowEvent::MouseWheel { delta, .. } => {
                let (delta_y, is_precise) = match delta {
                    MouseScrollDelta::LineDelta(_, y) => (-y * 20.0, false),
                    // 触控板：直接使用物理像素值，提高响应速度
                    MouseScrollDelta::PixelDelta(pos) => (-pos.y as f32 / self.scale_factor as f32, true),
                };
                
                // 忽略极小的滚动
                if delta_y.abs() < 0.1 {
                    return;
                }
                
                let x = self.mouse_pos.0;
                let y = self.mouse_pos.1;
                let actual_y = y + self.scroll.get_position();
                
                // 检查是否在 ScrollArea 内
                let mut handled_by_scrollview = false;
                
                // 首先检查 fixed 元素（使用视口坐标）
                let mut scroll_area_id = if let Some(element) = self.interaction.hit_test(x, y) {
                    if element.is_fixed && element.interaction_type == mini_render::ui::interaction::InteractionType::ScrollArea {
                        Some(element.id.clone())
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                // 如果没有找到 fixed 的滚动区域，再检查普通元素（使用滚动后的坐标）
                if scroll_area_id.is_none() {
                    if let Some(element) = self.interaction.hit_test(x, actual_y) {
                        if !element.is_fixed && element.interaction_type == mini_render::ui::interaction::InteractionType::ScrollArea {
                            scroll_area_id = Some(element.id.clone());
                        }
                    }
                }
                
                if let Some(id) = scroll_area_id {
                    if let Some(controller) = self.interaction.get_scroll_controller_mut(&id) {
                        controller.handle_scroll(delta_y, is_precise);
                        handled_by_scrollview = true;
                        // scroll-view 滚动需要重新渲染
                        self.needs_redraw = true;
                    }
                }
                
                if !handled_by_scrollview {
                    self.scroll.handle_scroll(delta_y, is_precise);
                    // 页面滚动只需要 present，不需要重新渲染
                    // needs_redraw 保持不变
                }
                
                // 滚动时请求重绘
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                match state {
                    ElementState::Pressed => {
                        self.click_start_pos = self.mouse_pos;
                        self.click_start_time = Instant::now();
                        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
                        
                        let x = self.mouse_pos.0;
                        let y = self.mouse_pos.1;
                        
                        // 首先检查是否点击在 tabbar 区域，如果是则不处理内容区域的交互
                        let page = self.page_stack.last();
                        let has_tabbar = page.map(|p| self.is_tabbar_page(&p.path)).unwrap_or(false);
                        let tabbar_y = if has_tabbar { (LOGICAL_HEIGHT - TABBAR_HEIGHT) as f32 } else { LOGICAL_HEIGHT as f32 };
                        
                        if has_tabbar && y >= tabbar_y {
                            // 点击在 tabbar 区域，不处理内容区域的交互
                            return;
                        }
                        
                        let actual_y = y + self.scroll.get_position();
                        
                        // 检查是否点击在已聚焦的输入框内（用于移动光标或开始选择）
                        if let Some(focused) = &self.interaction.focused_input {
                            let bounds = focused.bounds;
                            let text_offset = focused.text_offset;
                            // 检查点击是否在输入框内（考虑 fixed 和普通元素）
                            let click_in_input = (x >= bounds.x && x <= bounds.x + bounds.width &&
                                                  y >= bounds.y - self.scroll.get_position() && 
                                                  y <= bounds.y + bounds.height - self.scroll.get_position()) ||
                                                 (x >= bounds.x && x <= bounds.x + bounds.width &&
                                                  actual_y >= bounds.y && actual_y <= bounds.y + bounds.height);
                            
                            if click_in_input {
                                // 计算光标位置，准备可能的选择操作
                                if let Some(tr) = &self.text_renderer {
                                    let sf = self.scale_factor as f32;
                                    let font_size = 16.0 * sf;
                                    let padding_left = 12.0 * sf;
                                    let click_x = (x - bounds.x) * sf;
                                    
                                    let mut char_widths = Vec::new();
                                    for c in focused.value.chars() {
                                        let char_str = c.to_string();
                                        let width = tr.measure_text(&char_str, font_size);
                                        char_widths.push(width);
                                    }
                                    
                                    let cursor_pos = mini_render::ui::interaction::calculate_cursor_position(
                                        &focused.value, &char_widths, click_x, padding_left, text_offset
                                    );
                                    
                                    // 只准备选择，不立即开始（等待拖动）
                                    self.interaction.prepare_text_selection(cursor_pos);
                                    self.needs_redraw = true;
                                    if let Some(w) = &self.window { w.request_redraw(); }
                                    return;
                                }
                            }
                        }
                        
                        // 首先检查 fixed 元素（使用视口坐标）
                        if let Some(element) = self.interaction.hit_test(x, y) {
                            let element = element.clone();
                            match element.interaction_type {
                                mini_render::ui::interaction::InteractionType::Slider => {
                                    if !element.disabled {
                                        if let Some(result) = self.interaction.handle_click(x, y) {
                                            handle_interaction_result(
                                                &result,
                                                self.window.as_ref(),
                                                self.renderer.as_ref(),
                                                &mut self.app,
                                                &mut self.clipboard,
                                                self.scroll.get_position(),
                                                self.scale_factor,
                                            );
                                            self.needs_redraw = true;
                                            if let Some(w) = &self.window { w.request_redraw(); }
                                        }
                                    }
                                    return;
                                }
                                mini_render::ui::interaction::InteractionType::Button => {
                                    if !element.disabled {
                                        self.interaction.set_button_pressed(element.id.clone(), element.bounds);
                                        self.needs_redraw = true;
                                        if let Some(w) = &self.window { w.request_redraw(); }
                                    }
                                }
                                mini_render::ui::interaction::InteractionType::ScrollArea => {
                                    if let Some(controller) = self.interaction.get_scroll_controller_mut(&element.id) {
                                        controller.begin_drag(y, timestamp);
                                        self.interaction.dragging_scroll_area = Some(element.id.clone());
                                        return;
                                    }
                                }
                                _ => {}
                            }
                        }
                        // 然后检查普通元素（使用滚动后的坐标）
                        else if let Some(element) = self.interaction.hit_test(x, actual_y) {
                            let element = element.clone();
                            match element.interaction_type {
                                mini_render::ui::interaction::InteractionType::Slider => {
                                    if !element.disabled {
                                        if let Some(result) = self.interaction.handle_click(x, actual_y) {
                                            handle_interaction_result(
                                                &result,
                                                self.window.as_ref(),
                                                self.renderer.as_ref(),
                                                &mut self.app,
                                                &mut self.clipboard,
                                                self.scroll.get_position(),
                                                self.scale_factor,
                                            );
                                            self.needs_redraw = true;
                                            if let Some(w) = &self.window { w.request_redraw(); }
                                        }
                                    }
                                    return;
                                }
                                mini_render::ui::interaction::InteractionType::Button => {
                                    if !element.disabled {
                                        self.interaction.set_button_pressed(element.id.clone(), element.bounds);
                                        self.needs_redraw = true;
                                        if let Some(w) = &self.window { w.request_redraw(); }
                                    }
                                }
                                mini_render::ui::interaction::InteractionType::ScrollArea => {
                                    if let Some(controller) = self.interaction.get_scroll_controller_mut(&element.id) {
                                        controller.begin_drag(y, timestamp);
                                        self.interaction.dragging_scroll_area = Some(element.id.clone());
                                        return;
                                    }
                                }
                                _ => {}
                            }
                        }
                        
                        if !self.interaction.is_dragging_slider() {
                            self.scroll.begin_drag(self.mouse_pos.1, timestamp);
                        }
                    }
                    ElementState::Released => {
                        self.interaction.clear_button_pressed();
                        
                        // 结束文本选择
                        // 只有真正拖动选择时才阻止点击事件
                        let was_dragging_selection = self.interaction.is_dragging_selection();
                        self.interaction.end_text_selection();
                        
                        if was_dragging_selection {
                            self.needs_redraw = true;
                            if let Some(w) = &self.window { w.request_redraw(); }
                            // 如果是拖动选择，不触发点击事件
                            return;
                        }
                        
                        if let Some(id) = self.interaction.dragging_scroll_area.take() {
                            if let Some(controller) = self.interaction.get_scroll_controller_mut(&id) {
                                controller.end_drag();
                                self.needs_redraw = true;
                                if let Some(w) = &self.window { w.request_redraw(); }
                            }
                        }
                        
                        if let Some(result) = self.interaction.handle_mouse_release() {
                            handle_interaction_result(
                                &result,
                                self.window.as_ref(),
                                self.renderer.as_ref(),
                                &mut self.app,
                                &mut self.clipboard,
                                self.scroll.get_position(),
                                self.scale_factor,
                            );
                        }
                        
                        let needs_animation = self.scroll.end_drag();
                        let dx = (self.mouse_pos.0 - self.click_start_pos.0).abs();
                        let dy = (self.mouse_pos.1 - self.click_start_pos.1).abs();
                        let duration = self.click_start_time.elapsed().as_millis();
                        
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
                // 处理定时器
                if let Err(e) = self.app.update() {
                    eprintln!("Timer error: {}", e);
                }
                print_js_output(&self.app);
                
                // 处理 UI 事件
                self.process_ui_events();
                
                // 更新 Toast 超时
                self.update_toast_timeout();
                
                self.update_scroll();
                self.process_navigation();
                
                let has_video = mini_render::renderer::components::has_playing_video();
                let has_focused_input = self.interaction.has_focused_input();
                
                // 检查是否有 scroll-view 在滚动（需要重新渲染）
                let any_scrollview_scrolling = self.interaction.scroll_controllers.values().any(|c| c.is_animating() || c.is_dragging);
                
                // 需要重新渲染的情况：
                // 1. needs_redraw 为 true（数据变化、点击等）
                // 2. 有视频在播放
                // 3. scroll-view 内部在滚动（需要重新渲染 scroll-view 内容）
                // 4. 有输入框聚焦（光标闪烁动画）
                // 5. 页面正在滚动（需要渲染 tabbar 和 fixed 元素）
                let is_scrolling = self.scroll.is_animating() || self.scroll.is_dragging;
                if self.needs_redraw || has_video || any_scrollview_scrolling || has_focused_input || is_scrolling {
                    self.render();
                    self.needs_redraw = false;
                }
                // 页面滚动只需要 present，不需要重新渲染
                self.present();
                
                // 继续请求重绘的情况
                let has_timers = self.app.has_active_timers();
                let has_toast = self.toast.as_ref().map(|t| t.visible).unwrap_or(false);
                let has_loading = self.loading.as_ref().map(|l| l.visible).unwrap_or(false);
                let has_modal = self.modal.as_ref().map(|m| m.visible).unwrap_or(false);
                if is_scrolling || has_video || any_scrollview_scrolling || has_focused_input || has_timers || has_toast || has_loading || has_modal {
                    if let Some(window) = &self.window { window.request_redraw(); }
                }
            }
            _ => {}
        }
    }
}

/// 渲染 UI 覆盖层（Toast/Loading/Modal）
fn render_ui_overlay(
    buffer: &mut softbuffer::Buffer<Arc<Window>, Arc<Window>>,
    width: u32, height: u32, sf: f32, last_frame: Instant,
    toast: &Option<ToastState>, loading: &Option<LoadingState>, modal: &Option<ModalState>,
    text_renderer: Option<&TextRenderer>
) {
    // 渲染 Loading（优先级最高）
    if let Some(loading) = loading {
        if loading.visible {
            render_loading_to_buffer(buffer, width, height, &loading.title, sf, last_frame, text_renderer);
            return;
        }
    }
    
    // 渲染 Modal
    if let Some(modal) = modal {
        if modal.visible {
            render_modal_to_buffer(buffer, width, height, modal, sf, text_renderer);
            return;
        }
    }
    
    // 渲染 Toast
    if let Some(toast) = toast {
        if toast.visible {
            render_toast_to_buffer(buffer, width, height, &toast.title, &toast.icon, sf, text_renderer);
        }
    }
}

/// 渲染 Toast 到 buffer
fn render_toast_to_buffer(buffer: &mut softbuffer::Buffer<Arc<Window>, Arc<Window>>, width: u32, height: u32, title: &str, icon: &str, sf: f32, text_renderer: Option<&TextRenderer>) {
    let toast_padding = (16.0 * sf) as i32;
    let toast_min_width = (120.0 * sf) as i32;
    let toast_height = if icon == "none" { (44.0 * sf) as i32 } else { (100.0 * sf) as i32 };
    let icon_size = (40.0 * sf) as i32;
    let font_size = (14.0 * sf) as i32;
    
    let text_width = (title.chars().count() as f32 * font_size as f32 * 0.55) as i32;
    let toast_width = (toast_min_width).max(text_width + toast_padding * 2);
    
    let toast_x = (width as i32 - toast_width) / 2;
    let toast_y = (height as i32 - toast_height) / 2;
    
    let bg_color = 0xFF333333u32;
    let radius = (8.0 * sf) as i32;
    
    // 绘制圆角矩形背景
    for py in toast_y.max(0)..(toast_y + toast_height).min(height as i32) {
        for px in toast_x.max(0)..(toast_x + toast_width).min(width as i32) {
            let in_corner = (px < toast_x + radius || px >= toast_x + toast_width - radius) &&
                           (py < toast_y + radius || py >= toast_y + toast_height - radius);
            if in_corner {
                let cx = if px < toast_x + radius { toast_x + radius } else { toast_x + toast_width - radius };
                let cy = if py < toast_y + radius { toast_y + radius } else { toast_y + toast_height - radius };
                let dist = (((px - cx) * (px - cx) + (py - cy) * (py - cy)) as f32).sqrt();
                if dist > radius as f32 { continue; }
            }
            let idx = (py as u32 * width + px as u32) as usize;
            if idx < buffer.len() { buffer[idx] = bg_color; }
        }
    }
    
    // 绘制图标
    if icon != "none" {
        let icon_x = toast_x + (toast_width - icon_size) / 2;
        let icon_y = toast_y + toast_padding;
        let icon_color = if icon == "success" { 0xFF09BB07u32 } else { 0xFFFFFFFFu32 };
        let center_x = icon_x + icon_size / 2;
        let center_y = icon_y + icon_size / 2;
        let icon_radius = icon_size / 2 - 2;
        
        // 绘制圆环
        for py in (icon_y).max(0)..(icon_y + icon_size).min(height as i32) {
            for px in (icon_x).max(0)..(icon_x + icon_size).min(width as i32) {
                let dist = (((px - center_x) * (px - center_x) + (py - center_y) * (py - center_y)) as f32).sqrt();
                if dist <= icon_radius as f32 && dist >= (icon_radius - 3) as f32 {
                    let idx = (py as u32 * width + px as u32) as usize;
                    if idx < buffer.len() { buffer[idx] = icon_color; }
                }
            }
        }
        
        // 绘制勾号
        if icon == "success" {
            for t in 0..30 {
                let t = t as f32 / 30.0;
                let px = (center_x - icon_radius / 2) as f32 + (icon_radius / 3) as f32 * t;
                let py = center_y as f32 + (icon_radius / 3) as f32 * t;
                for dy in -2..=2 { for dx in -2..=2 {
                    let idx = ((py as i32 + dy) as u32 * width + (px as i32 + dx) as u32) as usize;
                    if idx < buffer.len() { buffer[idx] = icon_color; }
                }}
            }
            for t in 0..30 {
                let t = t as f32 / 30.0;
                let px = (center_x - icon_radius / 6) as f32 + (icon_radius * 2 / 3) as f32 * t;
                let py = (center_y + icon_radius / 3) as f32 - (icon_radius * 2 / 3) as f32 * t;
                for dy in -2..=2 { for dx in -2..=2 {
                    let idx = ((py as i32 + dy) as u32 * width + (px as i32 + dx) as u32) as usize;
                    if idx < buffer.len() { buffer[idx] = icon_color; }
                }}
            }
        }
    }
    
    // 绘制文字
    let text_y = if icon == "none" { toast_y + (toast_height - font_size) / 2 } 
                 else { toast_y + toast_padding + icon_size + (8.0 * sf) as i32 };
    let text_x = toast_x + (toast_width - text_width) / 2;
    
    if let Some(tr) = text_renderer {
        let mut temp_canvas = Canvas::new(toast_width as u32, font_size as u32 + 4);
        temp_canvas.clear(Color::TRANSPARENT);
        let paint = Paint::new().with_color(Color::WHITE);
        tr.draw_text(&mut temp_canvas, title, 0.0, 0.0, font_size as f32, &paint);
        let temp_pixels = temp_canvas.pixels();
        for py in 0..temp_canvas.height() as i32 {
            for px in 0..temp_canvas.width() as i32 {
                let src_idx = (py as u32 * temp_canvas.width() + px as u32) as usize;
                let dst_x = text_x + px;
                let dst_y = text_y + py;
                if dst_x >= 0 && dst_x < width as i32 && dst_y >= 0 && dst_y < height as i32 {
                    let dst_idx = (dst_y as u32 * width + dst_x as u32) as usize;
                    if dst_idx < buffer.len() && src_idx < temp_pixels.len() {
                        let pixel = temp_pixels[src_idx];
                        if pixel.a > 0 { buffer[dst_idx] = 0xFF000000 | ((pixel.r as u32) << 16) | ((pixel.g as u32) << 8) | (pixel.b as u32); }
                    }
                }
            }
        }
    }
}

/// 渲染 Loading 到 buffer
fn render_loading_to_buffer(buffer: &mut softbuffer::Buffer<Arc<Window>, Arc<Window>>, width: u32, height: u32, title: &str, sf: f32, last_frame: Instant, text_renderer: Option<&TextRenderer>) {
    let loading_size = (100.0 * sf) as i32;
    let loading_x = (width as i32 - loading_size) / 2;
    let loading_y = (height as i32 - loading_size) / 2;
    let radius = (8.0 * sf) as i32;
    let bg_color = 0xFF333333u32;
    
    for py in loading_y.max(0)..(loading_y + loading_size).min(height as i32) {
        for px in loading_x.max(0)..(loading_x + loading_size).min(width as i32) {
            let in_corner = (px < loading_x + radius || px >= loading_x + loading_size - radius) &&
                           (py < loading_y + radius || py >= loading_y + loading_size - radius);
            if in_corner {
                let cx = if px < loading_x + radius { loading_x + radius } else { loading_x + loading_size - radius };
                let cy = if py < loading_y + radius { loading_y + radius } else { loading_y + loading_size - radius };
                let dist = (((px - cx) * (px - cx) + (py - cy) * (py - cy)) as f32).sqrt();
                if dist > radius as f32 { continue; }
            }
            let idx = (py as u32 * width + px as u32) as usize;
            if idx < buffer.len() { buffer[idx] = bg_color; }
        }
    }
    
    // 绘制旋转加载圈
    let center_x = loading_x + loading_size / 2;
    let center_y = loading_y + (30.0 * sf) as i32;
    let spinner_radius = (16.0 * sf) as i32;
    let time = last_frame.elapsed().as_secs_f32();
    let angle = time * 5.0;
    
    for i in 0..12 {
        let seg_angle = angle + (i as f32 * std::f32::consts::PI / 6.0);
        let alpha = ((12 - i) as f32 / 12.0 * 255.0) as u32;
        let color = 0xFF000000 | (alpha << 16) | (alpha << 8) | alpha;
        let x1 = center_x + ((spinner_radius - 4) as f32 * seg_angle.cos()) as i32;
        let y1 = center_y + ((spinner_radius - 4) as f32 * seg_angle.sin()) as i32;
        let x2 = center_x + (spinner_radius as f32 * seg_angle.cos()) as i32;
        let y2 = center_y + (spinner_radius as f32 * seg_angle.sin()) as i32;
        for t in 0..10 {
            let t = t as f32 / 10.0;
            let px = (x1 as f32 + (x2 - x1) as f32 * t) as i32;
            let py = (y1 as f32 + (y2 - y1) as f32 * t) as i32;
            for dy in -1..=1 { for dx in -1..=1 {
                if px + dx >= 0 && px + dx < width as i32 && py + dy >= 0 && py + dy < height as i32 {
                    let idx = ((py + dy) as u32 * width + (px + dx) as u32) as usize;
                    if idx < buffer.len() { buffer[idx] = color; }
                }
            }}
        }
    }
    
    // 绘制文字
    let font_size = (14.0 * sf) as i32;
    let text_width = (title.chars().count() as f32 * font_size as f32 * 0.55) as i32;
    let text_x = loading_x + (loading_size - text_width) / 2;
    let text_y = loading_y + loading_size - (30.0 * sf) as i32;
    
    if let Some(tr) = text_renderer {
        let mut temp_canvas = Canvas::new(loading_size as u32, font_size as u32 + 4);
        temp_canvas.clear(Color::TRANSPARENT);
        let paint = Paint::new().with_color(Color::WHITE);
        tr.draw_text(&mut temp_canvas, title, 0.0, 0.0, font_size as f32, &paint);
        let temp_pixels = temp_canvas.pixels();
        for py in 0..temp_canvas.height() as i32 {
            for px in 0..temp_canvas.width() as i32 {
                let src_idx = (py as u32 * temp_canvas.width() + px as u32) as usize;
                let dst_x = text_x + px;
                let dst_y = text_y + py;
                if dst_x >= 0 && dst_x < width as i32 && dst_y >= 0 && dst_y < height as i32 {
                    let dst_idx = (dst_y as u32 * width + dst_x as u32) as usize;
                    if dst_idx < buffer.len() && src_idx < temp_pixels.len() {
                        let pixel = temp_pixels[src_idx];
                        if pixel.a > 0 { buffer[dst_idx] = 0xFF000000 | ((pixel.r as u32) << 16) | ((pixel.g as u32) << 8) | (pixel.b as u32); }
                    }
                }
            }
        }
    }
}

/// 渲染 Modal 到 buffer
fn render_modal_to_buffer(buffer: &mut softbuffer::Buffer<Arc<Window>, Arc<Window>>, width: u32, height: u32, modal: &ModalState, sf: f32, text_renderer: Option<&TextRenderer>) {
    // 绘制半透明遮罩
    for i in 0..buffer.len() {
        let existing = buffer[i];
        let r = ((existing >> 16) & 0xFF) / 2;
        let g = ((existing >> 8) & 0xFF) / 2;
        let b = (existing & 0xFF) / 2;
        buffer[i] = 0xFF000000 | (r << 16) | (g << 8) | b;
    }
    
    let modal_width = (280.0 * sf) as i32;
    let modal_padding = (20.0 * sf) as i32;
    let title_height = (22.0 * sf) as i32;
    let content_height = (44.0 * sf) as i32;
    let button_height = (44.0 * sf) as i32;
    let modal_height = modal_padding * 2 + title_height + content_height + button_height + (20.0 * sf) as i32;
    let modal_x = (width as i32 - modal_width) / 2;
    let modal_y = (height as i32 - modal_height) / 2;
    let radius = (12.0 * sf) as i32;
    let bg_color = 0xFFFFFFFFu32;
    
    // 绘制白色背景
    for py in modal_y.max(0)..(modal_y + modal_height).min(height as i32) {
        for px in modal_x.max(0)..(modal_x + modal_width).min(width as i32) {
            let in_corner = (px < modal_x + radius || px >= modal_x + modal_width - radius) &&
                           (py < modal_y + radius || py >= modal_y + modal_height - radius);
            if in_corner {
                let cx = if px < modal_x + radius { modal_x + radius } else { modal_x + modal_width - radius };
                let cy = if py < modal_y + radius { modal_y + radius } else { modal_y + modal_height - radius };
                let dist = (((px - cx) * (px - cx) + (py - cy) * (py - cy)) as f32).sqrt();
                if dist > radius as f32 { continue; }
            }
            let idx = (py as u32 * width + px as u32) as usize;
            if idx < buffer.len() { buffer[idx] = bg_color; }
        }
    }
    
    // 绘制标题
    let title_font_size = (17.0 * sf) as i32;
    let title_y = modal_y + modal_padding;
    if let Some(tr) = text_renderer {
        let mut temp_canvas = Canvas::new(modal_width as u32, title_font_size as u32 + 4);
        temp_canvas.clear(Color::TRANSPARENT);
        let text_w = tr.measure_text(&modal.title, title_font_size as f32);
        let text_x = (modal_width as f32 - text_w) / 2.0;
        let paint = Paint::new().with_color(Color::BLACK);
        tr.draw_text(&mut temp_canvas, &modal.title, text_x, 0.0, title_font_size as f32, &paint);
        let temp_pixels = temp_canvas.pixels();
        for py in 0..temp_canvas.height() as i32 {
            for px in 0..temp_canvas.width() as i32 {
                let src_idx = (py as u32 * temp_canvas.width() + px as u32) as usize;
                let dst_x = modal_x + px;
                let dst_y = title_y + py;
                if dst_x >= 0 && dst_x < width as i32 && dst_y >= 0 && dst_y < height as i32 {
                    let dst_idx = (dst_y as u32 * width + dst_x as u32) as usize;
                    if dst_idx < buffer.len() && src_idx < temp_pixels.len() {
                        let pixel = temp_pixels[src_idx];
                        if pixel.a > 0 { buffer[dst_idx] = 0xFF000000 | ((pixel.r as u32) << 16) | ((pixel.g as u32) << 8) | (pixel.b as u32); }
                    }
                }
            }
        }
    }
    
    // 绘制内容
    let content_font_size = (14.0 * sf) as i32;
    let content_y = title_y + title_font_size + (15.0 * sf) as i32;
    if let Some(tr) = text_renderer {
        let mut temp_canvas = Canvas::new(modal_width as u32, content_height as u32);
        temp_canvas.clear(Color::TRANSPARENT);
        let text_w = tr.measure_text(&modal.content, content_font_size as f32);
        let text_x = (modal_width as f32 - text_w) / 2.0;
        let paint = Paint::new().with_color(Color::from_hex(0x666666));
        tr.draw_text(&mut temp_canvas, &modal.content, text_x.max(modal_padding as f32), 0.0, content_font_size as f32, &paint);
        let temp_pixels = temp_canvas.pixels();
        for py in 0..temp_canvas.height() as i32 {
            for px in 0..temp_canvas.width() as i32 {
                let src_idx = (py as u32 * temp_canvas.width() + px as u32) as usize;
                let dst_x = modal_x + px;
                let dst_y = content_y + py;
                if dst_x >= 0 && dst_x < width as i32 && dst_y >= 0 && dst_y < height as i32 {
                    let dst_idx = (dst_y as u32 * width + dst_x as u32) as usize;
                    if dst_idx < buffer.len() && src_idx < temp_pixels.len() {
                        let pixel = temp_pixels[src_idx];
                        if pixel.a > 0 { buffer[dst_idx] = 0xFF000000 | ((pixel.r as u32) << 16) | ((pixel.g as u32) << 8) | (pixel.b as u32); }
                    }
                }
            }
        }
    }
    
    // 绘制分隔线
    let line_y = modal_y + modal_height - button_height - 1;
    let line_color = 0xFFE5E5E5u32;
    for px in modal_x..(modal_x + modal_width) {
        let idx = (line_y as u32 * width + px as u32) as usize;
        if idx < buffer.len() { buffer[idx] = line_color; }
    }
    
    // 绘制按钮
    let button_y = modal_y + modal_height - button_height;
    let button_font_size = (17.0 * sf) as i32;
    
    if modal.show_cancel {
        let button_width = modal_width / 2;
        // 取消按钮
        if let Some(tr) = text_renderer {
            let mut temp_canvas = Canvas::new(button_width as u32, button_font_size as u32 + 4);
            temp_canvas.clear(Color::TRANSPARENT);
            let text_w = tr.measure_text(&modal.cancel_text, button_font_size as f32);
            let text_x = (button_width as f32 - text_w) / 2.0;
            let paint = Paint::new().with_color(Color::BLACK);
            tr.draw_text(&mut temp_canvas, &modal.cancel_text, text_x, 0.0, button_font_size as f32, &paint);
            let temp_pixels = temp_canvas.pixels();
            let btn_text_y = button_y + (button_height - button_font_size) / 2;
            for py in 0..temp_canvas.height() as i32 {
                for px in 0..temp_canvas.width() as i32 {
                    let src_idx = (py as u32 * temp_canvas.width() + px as u32) as usize;
                    let dst_x = modal_x + px;
                    let dst_y = btn_text_y + py;
                    if dst_x >= 0 && dst_x < width as i32 && dst_y >= 0 && dst_y < height as i32 {
                        let dst_idx = (dst_y as u32 * width + dst_x as u32) as usize;
                        if dst_idx < buffer.len() && src_idx < temp_pixels.len() {
                            let pixel = temp_pixels[src_idx];
                            if pixel.a > 0 { buffer[dst_idx] = 0xFF000000 | ((pixel.r as u32) << 16) | ((pixel.g as u32) << 8) | (pixel.b as u32); }
                        }
                    }
                }
            }
        }
        // 垂直分隔线
        let vline_x = modal_x + button_width;
        for py in button_y..(button_y + button_height) {
            let idx = (py as u32 * width + vline_x as u32) as usize;
            if idx < buffer.len() { buffer[idx] = line_color; }
        }
        // 确认按钮
        if let Some(tr) = text_renderer {
            let mut temp_canvas = Canvas::new(button_width as u32, button_font_size as u32 + 4);
            temp_canvas.clear(Color::TRANSPARENT);
            let text_w = tr.measure_text(&modal.confirm_text, button_font_size as f32);
            let text_x = (button_width as f32 - text_w) / 2.0;
            let paint = Paint::new().with_color(Color::from_hex(0x576B95));
            tr.draw_text(&mut temp_canvas, &modal.confirm_text, text_x, 0.0, button_font_size as f32, &paint);
            let temp_pixels = temp_canvas.pixels();
            let btn_text_y = button_y + (button_height - button_font_size) / 2;
            for py in 0..temp_canvas.height() as i32 {
                for px in 0..temp_canvas.width() as i32 {
                    let src_idx = (py as u32 * temp_canvas.width() + px as u32) as usize;
                    let dst_x = modal_x + button_width + px;
                    let dst_y = btn_text_y + py;
                    if dst_x >= 0 && dst_x < width as i32 && dst_y >= 0 && dst_y < height as i32 {
                        let dst_idx = (dst_y as u32 * width + dst_x as u32) as usize;
                        if dst_idx < buffer.len() && src_idx < temp_pixels.len() {
                            let pixel = temp_pixels[src_idx];
                            if pixel.a > 0 { buffer[dst_idx] = 0xFF000000 | ((pixel.r as u32) << 16) | ((pixel.g as u32) << 8) | (pixel.b as u32); }
                        }
                    }
                }
            }
        }
    } else {
        if let Some(tr) = text_renderer {
            let mut temp_canvas = Canvas::new(modal_width as u32, button_font_size as u32 + 4);
            temp_canvas.clear(Color::TRANSPARENT);
            let text_w = tr.measure_text(&modal.confirm_text, button_font_size as f32);
            let text_x = (modal_width as f32 - text_w) / 2.0;
            let paint = Paint::new().with_color(Color::from_hex(0x576B95));
            tr.draw_text(&mut temp_canvas, &modal.confirm_text, text_x, 0.0, button_font_size as f32, &paint);
            let temp_pixels = temp_canvas.pixels();
            let btn_text_y = button_y + (button_height - button_font_size) / 2;
            for py in 0..temp_canvas.height() as i32 {
                for px in 0..temp_canvas.width() as i32 {
                    let src_idx = (py as u32 * temp_canvas.width() + px as u32) as usize;
                    let dst_x = modal_x + px;
                    let dst_y = btn_text_y + py;
                    if dst_x >= 0 && dst_x < width as i32 && dst_y >= 0 && dst_y < height as i32 {
                        let dst_idx = (dst_y as u32 * width + dst_x as u32) as usize;
                        if dst_idx < buffer.len() && src_idx < temp_pixels.len() {
                            let pixel = temp_pixels[src_idx];
                            if pixel.a > 0 { buffer[dst_idx] = 0xFF000000 | ((pixel.r as u32) << 16) | ((pixel.g as u32) << 8) | (pixel.b as u32); }
                        }
                    }
                }
            }
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
