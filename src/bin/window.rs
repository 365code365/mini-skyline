//! 带窗口的小程序运行器 - 支持多页面导航和原生 TabBar

mod app_window;

use app_window::*;
use app_window::events::{keyboard, mouse, ime};
use app_window::ui_overlay::{ToastState, LoadingState, ModalState, render_ui_overlay};
use app_window::event_handler as evt;
use app_window::click_handler as click;

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
    toast: Option<ToastState>,
    loading: Option<LoadingState>,
    modal: Option<ModalState>,
}

impl MiniAppWindow {
    fn new() -> Result<Self, String> {
        let mut app = MiniApp::new(LOGICAL_WIDTH, LOGICAL_HEIGHT)?;
        app.init()?;
        
        let app_js = include_str!("../../sample-app/app.js");
        app.load_script(app_js)?;
        println!("📱 App.js loaded");
        
        let app_json = include_str!("../../sample-app/app.json");
        let app_config: AppConfig = serde_json::from_str(app_json)
            .map_err(|e| format!("Failed to parse app.json: {}", e))?;
        
        println!("📱 App config loaded");
        if let Some(ref tab_bar) = app_config.tab_bar {
            println!("   TabBar: {} items, custom: {}", tab_bar.list.len(), tab_bar.custom);
        }
        
        let custom_tabbar = if app_config.tab_bar.as_ref().map(|tb| tb.custom).unwrap_or(false) {
            load_custom_tabbar()?
        } else {
            None
        };
        
        let pages = load_all_pages();
        let has_tabbar = app_config.tab_bar.as_ref()
            .map(|tb| tb.list.iter().any(|item| item.page_path == "pages/index/index"))
            .unwrap_or(false);
        
        let now = Instant::now();
        let clipboard = arboard::Clipboard::new().ok();
        
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
        
        self.app.eval("if(__currentPage && __currentPage.onShow) __currentPage.onShow()").ok();
        print_js_output(&self.app);
        
        self.page_stack.push(PageInstance { path: path.to_string(), query, wxml_nodes, stylesheet });
        self.current_page_index = self.page_stack.len() - 1;
        
        let has_tabbar = self.is_tabbar_page(path);
        self.scroll = ScrollController::new(
            CONTENT_HEIGHT as f32, 
            (LOGICAL_HEIGHT - if has_tabbar { TABBAR_HEIGHT } else { 0 }) as f32
        );
        self.needs_redraw = true;
        
        println!("✅ Page loaded: {} (stack: {})", path, self.page_stack.len());
        Ok(())
    }
    
    fn navigate_back(&mut self) -> Result<(), String> {
        if self.page_stack.len() <= 1 {
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
                self.app.eval(&format!("if(__currentPage && __currentPage.onLoad) __currentPage.onLoad({})", query_json)).ok();
                print_js_output(&self.app);
            }
            
            let has_tabbar = self.is_tabbar_page(&path);
            self.scroll = ScrollController::new(
                CONTENT_HEIGHT as f32,
                (LOGICAL_HEIGHT - if has_tabbar { TABBAR_HEIGHT } else { 0 }) as f32
            );
        }
        
        self.needs_redraw = true;
        println!("⬅️ Navigate back (stack: {})", self.page_stack.len());
        Ok(())
    }
    
    fn switch_tab(&mut self, path: &str) -> Result<(), String> {
        let path = path.trim_start_matches('/');
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
        
        self.canvas = Some(Canvas::new(physical_width, physical_height));
        self.tabbar_canvas = Some(Canvas::new(physical_width, tabbar_physical_height));
        self.fixed_canvas = Some(Canvas::new(physical_width, viewport_physical_height));
        
        self.text_renderer = TextRenderer::load_system_font()
            .or_else(|_| TextRenderer::from_bytes(include_bytes!("../../assets/ArialUnicode.ttf")))
            .ok();
    }
    
    fn update_renderers(&mut self) {
        if let Some(page) = self.page_stack.last() {
            self.renderer = Some(WxmlRenderer::new_with_scale(
                page.stylesheet.clone(),
                LOGICAL_WIDTH as f32,
                LOGICAL_HEIGHT as f32,
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
        let page_data = self.app.eval("__getPageData()")
            .map(|s| serde_json::from_str(&s).unwrap_or(json!({})))
            .unwrap_or(json!({}));
        
        let page = match self.page_stack.last() { Some(p) => p, None => return };
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
            
            let required_height = (content_height * self.scale_factor as f32).ceil() as u32;
            let current_height = self.canvas.as_ref().map(|c| c.height()).unwrap_or(0);
            
            if current_height != required_height && required_height > 0 {
                let physical_width = (LOGICAL_WIDTH as f64 * self.scale_factor) as u32;
                self.canvas = Some(Canvas::new(physical_width, required_height));
                
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
        let tab_bar_config = match &self.app_config.tab_bar { Some(tb) => tb.clone(), None => return };
        let selected_index = self.get_tabbar_index(current_path).unwrap_or(0);
        let list: Vec<serde_json::Value> = tab_bar_config.list.iter()
            .map(|item| json!({ "pagePath": item.page_path, "text": item.text }))
            .collect();
        let tabbar_data = json!({ "selected": selected_index, "list": list });
        
        let custom_tabbar = match &self.custom_tabbar { Some(ct) => ct, None => return };
        let wxml_nodes = custom_tabbar.wxml_nodes.clone();
        
        if let (Some(canvas), Some(renderer)) = (&mut self.tabbar_canvas, &mut self.tabbar_renderer) {
            canvas.clear(Color::WHITE);
            renderer.render(canvas, &wxml_nodes, &tabbar_data);
        }
    }
    
    fn render_native_tabbar(&mut self, current_path: &str) {
        let tab_bar = match &self.app_config.tab_bar { Some(tb) => tb.clone(), None => return };
        if let (Some(canvas), Some(text_renderer)) = (&mut self.tabbar_canvas, &self.text_renderer) {
            render_native_tabbar(canvas, text_renderer, &tab_bar, current_path, self.scale_factor);
        }
    }
    
    fn present(&mut self) {
        let canvas = match &self.canvas { Some(c) => c, None => return };
        let page = match self.page_stack.last() { Some(p) => p, None => return };
        let has_tabbar = self.is_tabbar_page(&page.path);
        
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
                        &mut buffer, size.width, size.height, canvas,
                        self.fixed_canvas.as_ref(), self.tabbar_canvas.as_ref(),
                        scroll_offset, has_tabbar, tabbar_physical_height,
                    );
                    
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
        // Modal 处理
        if let Some(modal) = &self.modal {
            if modal.visible {
                self.handle_modal_click(x, y);
                return;
            }
        }
        
        // Loading 时忽略点击
        if self.loading.as_ref().map(|l| l.visible).unwrap_or(false) {
            return;
        }
        
        let page = match self.page_stack.last() { Some(p) => p, None => return };
        let has_tabbar = self.is_tabbar_page(&page.path);
        let tabbar_y = if has_tabbar { (LOGICAL_HEIGHT - TABBAR_HEIGHT) as f32 } else { LOGICAL_HEIGHT as f32 };
        
        if has_tabbar && y >= tabbar_y {
            if self.is_custom_tabbar() {
                if let Some(nav) = click::handle_custom_tabbar_click(x, y - tabbar_y, self.tabbar_renderer.as_ref(), &page.path) {
                    self.pending_navigation = Some(nav);
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
            } else if let Some(tab_bar) = &self.app_config.tab_bar {
                if let Some(nav) = click::handle_native_tabbar_click_wrapper(x, tab_bar, &page.path) {
                    self.pending_navigation = Some(nav);
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
            }
        } else {
            if let Some(nav) = click::handle_content_click(
                x, y, &self.scroll, has_tabbar, &mut self.interaction,
                self.renderer.as_ref(), &mut self.app, self.scale_factor,
                self.text_renderer.as_ref(), self.window.as_ref(), &mut self.clipboard,
            ) {
                self.pending_navigation = Some(nav);
            }
            self.needs_redraw = true;
        }
    }
    
    fn handle_modal_press(&mut self, x: f32, y: f32) -> bool {
        let modal = match &self.modal { Some(m) if m.visible => m, _ => return false };
        let layout = click::calculate_modal_layout(modal, self.scale_factor as f32, self.text_renderer.as_ref());
        
        if let Some(button) = click::detect_modal_button(x, y, &layout, modal.show_cancel) {
            if let Some(m) = &mut self.modal {
                m.pressed_button = Some(button);
            }
            self.needs_redraw = true;
            if let Some(w) = &self.window { w.request_redraw(); }
            return true;
        }
        false
    }
    
    fn handle_modal_release(&mut self, x: f32, y: f32) {
        let pressed = self.modal.as_ref().and_then(|m| m.pressed_button.clone());
        if let Some(m) = &mut self.modal { m.pressed_button = None; }
        
        let modal = match &self.modal { Some(m) if m.visible => m, _ => return };
        let layout = click::calculate_modal_layout(modal, self.scale_factor as f32, self.text_renderer.as_ref());
        
        if let Some(clicked_button) = click::detect_modal_button(x, y, &layout, modal.show_cancel) {
            if pressed.as_deref() == Some(&clicked_button) {
                if clicked_button == "cancel" {
                    self.app.eval("if(__modalCallback) __modalCallback({ confirm: false, cancel: true })").ok();
                } else {
                    self.app.eval("if(__modalCallback) __modalCallback({ confirm: true, cancel: false })").ok();
                }
                self.modal = None;
            }
        }
        
        self.needs_redraw = true;
        if let Some(w) = &self.window { w.request_redraw(); }
    }
    
    fn handle_modal_click(&mut self, x: f32, y: f32) {
        self.handle_modal_release(x, y);
    }
    
    fn process_navigation(&mut self) {
        if let Some(nav) = self.pending_navigation.take() {
            match nav {
                NavigationRequest::NavigateTo { url } => {
                    let (path, query) = parse_url(&url);
                    self.navigate_to(&path, query).ok();
                    self.update_renderers();
                }
                NavigationRequest::NavigateBack => {
                    self.navigate_back().ok();
                    self.update_renderers();
                }
                NavigationRequest::SwitchTab { url } => {
                    let (path, _) = parse_url(&url);
                    self.switch_tab(&path).ok();
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
        let (animating, event) = self.scroll.update_with_events(dt);
        if animating { scroll_changed = true; }
        
        if let Some(scroll_event) = event {
            evt::handle_scroll_event(scroll_event, &mut self.app);
            self.needs_redraw = true;
        }
        
        for controller in self.interaction.scroll_controllers.values_mut() {
            if controller.update(dt) { scroll_changed = true; }
        }
        
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
            
            println!("\n🎮 Ready!\n");
        }
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            
            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
            }
            
            WindowEvent::KeyboardInput { event, .. } => {
                let (needs_redraw, pending_nav, exit_requested) = evt::handle_keyboard_event(
                    event, self.modifiers, &mut self.interaction, &mut self.clipboard,
                    self.window.as_ref(), self.renderer.as_ref(), &mut self.app,
                    &mut self.scroll, self.scale_factor,
                );
                
                if exit_requested { event_loop.exit(); }
                if let Some(nav) = pending_nav { self.pending_navigation = Some(nav); }
                if needs_redraw {
                    self.needs_redraw = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
            }
            
            WindowEvent::Ime(ime_event) => {
                if evt::handle_ime_event(
                    ime_event, &mut self.interaction, self.window.as_ref(),
                    self.renderer.as_ref(), &mut self.app, &mut self.clipboard,
                    self.scroll.get_position(), self.scale_factor,
                ) {
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
                
                if evt::handle_cursor_moved(
                    x, y, &mut self.interaction, &mut self.scroll,
                    self.text_renderer.as_ref(), self.window.as_ref(),
                    self.renderer.as_ref(), &mut self.app, &mut self.clipboard,
                    self.scale_factor,
                ) {
                    self.needs_redraw = true;
                }
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            
            WindowEvent::MouseWheel { delta, .. } => {
                if evt::handle_mouse_wheel(delta, self.mouse_pos, &mut self.interaction, &mut self.scroll, self.scale_factor) {
                    self.needs_redraw = true;
                }
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                match state {
                    ElementState::Pressed => self.handle_mouse_pressed(),
                    ElementState::Released => self.handle_mouse_released(),
                }
            }
            
            WindowEvent::RedrawRequested => {
                self.app.update().ok();
                print_js_output(&self.app);
                
                if evt::process_ui_events(&mut self.app, &mut self.toast, &mut self.loading, &mut self.modal) {
                    self.needs_redraw = true;
                }
                if evt::update_toast_timeout(&mut self.toast) {
                    self.needs_redraw = true;
                }
                
                self.update_scroll();
                self.process_navigation();
                
                let has_video = mini_render::renderer::components::has_playing_video();
                let has_focused_input = self.interaction.has_focused_input();
                let any_scrollview_scrolling = self.interaction.scroll_controllers.values().any(|c| c.is_animating() || c.is_dragging);
                let is_scrolling = self.scroll.is_animating() || self.scroll.is_dragging;
                
                if self.needs_redraw || has_video || any_scrollview_scrolling || has_focused_input || is_scrolling {
                    self.render();
                    self.needs_redraw = false;
                }
                self.present();
                
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

impl MiniAppWindow {
    fn handle_mouse_pressed(&mut self) {
        self.click_start_pos = self.mouse_pos;
        self.click_start_time = Instant::now();
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        let (x, y) = self.mouse_pos;
        
        // Modal 按钮按下
        if self.modal.as_ref().map(|m| m.visible).unwrap_or(false) {
            self.handle_modal_press(x, y);
            return;
        }
        
        // Loading 时忽略
        if self.loading.as_ref().map(|l| l.visible).unwrap_or(false) {
            return;
        }
        
        // TabBar 区域检查
        let page = self.page_stack.last();
        let has_tabbar = page.map(|p| self.is_tabbar_page(&p.path)).unwrap_or(false);
        let tabbar_y = if has_tabbar { (LOGICAL_HEIGHT - TABBAR_HEIGHT) as f32 } else { LOGICAL_HEIGHT as f32 };
        
        if has_tabbar && y >= tabbar_y {
            return;
        }
        
        let actual_y = y + self.scroll.get_position();
        
        // 输入框内点击处理
        if let Some(focused) = &self.interaction.focused_input {
            let bounds = focused.bounds;
            let click_in_input = (x >= bounds.x && x <= bounds.x + bounds.width &&
                                  y >= bounds.y - self.scroll.get_position() && 
                                  y <= bounds.y + bounds.height - self.scroll.get_position()) ||
                                 (x >= bounds.x && x <= bounds.x + bounds.width &&
                                  actual_y >= bounds.y && actual_y <= bounds.y + bounds.height);
            
            if click_in_input {
                if let Some(tr) = &self.text_renderer {
                    let sf = self.scale_factor as f32;
                    let font_size = 16.0 * sf;
                    let padding_left = 12.0 * sf;
                    let click_x = (x - bounds.x) * sf;
                    let text_offset = focused.text_offset;
                    
                    let char_widths: Vec<f32> = focused.value.chars()
                        .map(|c| tr.measure_text(&c.to_string(), font_size))
                        .collect();
                    
                    let cursor_pos = mini_render::ui::interaction::calculate_cursor_position(
                        &focused.value, &char_widths, click_x, padding_left, text_offset
                    );
                    
                    self.interaction.prepare_text_selection(cursor_pos);
                    self.needs_redraw = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                    return;
                }
            }
        }
        
        // 交互元素处理
        let element = self.interaction.hit_test(x, y)
            .or_else(|| self.interaction.hit_test(x, actual_y))
            .cloned();
        
        if let Some(element) = element {
            use mini_render::ui::interaction::InteractionType;
            match element.interaction_type {
                InteractionType::Slider if !element.disabled => {
                    let test_y = if element.is_fixed { y } else { actual_y };
                    if let Some(result) = self.interaction.handle_click(x, test_y) {
                        handle_interaction_result(
                            &result, self.window.as_ref(), self.renderer.as_ref(),
                            &mut self.app, &mut self.clipboard, self.scroll.get_position(), self.scale_factor,
                        );
                        self.needs_redraw = true;
                        if let Some(w) = &self.window { w.request_redraw(); }
                    }
                    return;
                }
                InteractionType::Button if !element.disabled => {
                    self.interaction.set_button_pressed(element.id.clone(), element.bounds);
                    self.needs_redraw = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
                InteractionType::ScrollArea => {
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
            self.scroll.begin_drag(y, timestamp);
        }
    }
    
    fn handle_mouse_released(&mut self) {
        // Modal 按钮释放
        if self.modal.as_ref().map(|m| m.visible && m.pressed_button.is_some()).unwrap_or(false) {
            self.handle_modal_release(self.mouse_pos.0, self.mouse_pos.1);
            return;
        }
        
        self.interaction.clear_button_pressed();
        
        let was_dragging_selection = self.interaction.is_dragging_selection();
        self.interaction.end_text_selection();
        
        if was_dragging_selection {
            self.needs_redraw = true;
            if let Some(w) = &self.window { w.request_redraw(); }
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
                &result, self.window.as_ref(), self.renderer.as_ref(),
                &mut self.app, &mut self.clipboard, self.scroll.get_position(), self.scale_factor,
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Mini App Engine\n");
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = MiniAppWindow::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
