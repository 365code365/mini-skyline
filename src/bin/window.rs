//! 带窗口的小程序运行器 - 支持多页面导航和原生 TabBar

mod app_window;

use app_window::*;
use app_window::events::{keyboard, mouse, ime};

use mini_render::runtime::MiniApp;
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::renderer::WxmlRenderer;
use mini_render::ui::interaction::InteractionManager;
use mini_render::{Canvas, Color};
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
}

impl MiniAppWindow {
    fn new() -> Result<Self, String> {
        let mut app = MiniApp::new(LOGICAL_WIDTH, LOGICAL_HEIGHT)?;
        app.init()?;
        
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
        
        let query_json = serde_json::to_string(&query).unwrap_or("{}".to_string());
        let load_code = format!("if(__currentPage && __currentPage.onLoad) __currentPage.onLoad({})", query_json);
        self.app.eval(&load_code).ok();
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
        let scroll_offset = self.scroll.get_position();
        let has_tabbar = self.is_tabbar_page(&current_path);
        let viewport_height = (LOGICAL_HEIGHT - if has_tabbar { TABBAR_HEIGHT } else { 0 }) as f32;
        
        let mut content_height = 0.0f32;
        if let Some(canvas) = &mut self.canvas {
            canvas.clear(Color::from_hex(0xF5F5F5));
            if let Some(renderer) = &mut self.renderer {
                content_height = renderer.render_with_scroll_and_viewport(canvas, &wxml_nodes, &page_data, &mut self.interaction, scroll_offset, viewport_height);
            }
        }
        
        if content_height > 0.0 {
            self.scroll.update_content_height(content_height, viewport_height);
            
            // canvas 高度 = 内容高度
            let required_height = (content_height * self.scale_factor as f32).ceil() as u32;
            let current_height = self.canvas.as_ref().map(|c| c.height()).unwrap_or(0);
            
            if current_height != required_height && required_height > 0 {
                let physical_width = (LOGICAL_WIDTH as f64 * self.scale_factor) as u32;
                self.canvas = Some(Canvas::new(physical_width, required_height));
                
                // 重新渲染到新 canvas
                if let Some(canvas) = &mut self.canvas {
                    canvas.clear(Color::from_hex(0xF5F5F5));
                    if let Some(renderer) = &mut self.renderer {
                        renderer.render_with_scroll_and_viewport(canvas, &wxml_nodes, &page_data, &mut self.interaction, scroll_offset, viewport_height);
                    }
                }
            }
        }
        
        if let Some(fixed_canvas) = &mut self.fixed_canvas {
            fixed_canvas.clear(Color::new(0, 0, 0, 0));
            if let Some(renderer) = &mut self.renderer {
                renderer.render_fixed_elements(fixed_canvas, &wxml_nodes, &page_data, &mut self.interaction, viewport_height);
            }
        }
        
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
    
    fn update_scroll(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        
        let mut scroll_changed = false;
        if self.scroll.update(dt) {
            scroll_changed = true;
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
                
                if self.interaction.is_dragging_slider() {
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
                    // 触控板：将物理像素转换为逻辑像素，并降低灵敏度
                    MouseScrollDelta::PixelDelta(pos) => (-pos.y as f32 / self.scale_factor as f32 * 0.5, true),
                };
                
                let x = self.mouse_pos.0;
                let y = self.mouse_pos.1;
                let actual_y = y + self.scroll.get_position();
                
                // 检查是否在 ScrollArea 内
                let mut handled = false;
                
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
                        handled = true;
                    }
                }
                
                if !handled {
                    self.scroll.handle_scroll(delta_y, is_precise);
                }
                
                // 滚动时不需要重新渲染，只需要 present
                // needs_redraw 保持不变，只请求重绘
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
                        let actual_y = y + self.scroll.get_position();
                        
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
                self.update_scroll();
                self.process_navigation();
                
                let has_video = mini_render::renderer::components::has_playing_video();
                
                if self.needs_redraw || has_video {
                    self.render();
                    self.needs_redraw = false;
                }
                self.present();
                
                let any_scroll_active = self.interaction.scroll_controllers.values().any(|c| c.is_animating() || c.is_dragging);
                if self.scroll.is_animating() || self.scroll.is_dragging || has_video || any_scroll_active {
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
