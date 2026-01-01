//! 带窗口的小程序运行器 - 支持多页面导航和原生 TabBar

mod app_window;

use app_window::*;
use app_window::events::{keyboard, mouse, ime};
use app_window::ui_overlay::{ToastState, LoadingState, ModalState, render_ui_overlay};

use mini_render::runtime::{MiniApp, UiEvent};
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
    
    /// 处理 Modal 按钮按下
    fn handle_modal_press(&mut self, x: f32, y: f32) -> bool {
        let modal = match &self.modal {
            Some(m) if m.visible => m,
            _ => return false,
        };
        
        let sf = self.scale_factor as f32;
        let modal_width = 280.0 * sf;
        let modal_padding = 24.0 * sf;
        let title_font_size = 17.0 * sf;
        let content_font_size = 14.0 * sf;
        let button_height = 50.0 * sf;
        let gap = 16.0 * sf;
        
        let title_line_height = title_font_size * 1.4;
        // 计算内容需要的行数
        let content_max_width = modal_width - modal_padding * 2.0;
        let content_lines = if let Some(tr) = &self.text_renderer {
            let text_width = tr.measure_text(&modal.content, content_font_size);
            ((text_width / content_max_width).ceil() as i32).max(1)
        } else { 1 };
        let content_line_height = content_font_size * 1.6 * content_lines as f32;
        
        let modal_height = modal_padding + title_line_height + gap + content_line_height + gap + button_height;
        
        // 转换为逻辑坐标
        let modal_x = (LOGICAL_WIDTH as f32 * sf - modal_width) / 2.0 / sf;
        let modal_y = (LOGICAL_HEIGHT as f32 * sf - modal_height) / 2.0 / sf;
        let button_y = modal_y + (modal_height - button_height) / sf;
        let button_h = button_height / sf;
        let modal_w = modal_width / sf;
        
        // 检查是否点击在按钮区域
        if y >= button_y && y <= button_y + button_h {
            if x >= modal_x && x <= modal_x + modal_w {
                let show_cancel = modal.show_cancel;
                
                if show_cancel {
                    let button_width = modal_w / 2.0;
                    if x < modal_x + button_width {
                        // 按下取消按钮
                        if let Some(m) = &mut self.modal {
                            m.pressed_button = Some("cancel".to_string());
                        }
                    } else {
                        // 按下确认按钮
                        if let Some(m) = &mut self.modal {
                            m.pressed_button = Some("confirm".to_string());
                        }
                    }
                } else {
                    // 只有确认按钮
                    if let Some(m) = &mut self.modal {
                        m.pressed_button = Some("confirm".to_string());
                    }
                }
                self.needs_redraw = true;
                if let Some(w) = &self.window { w.request_redraw(); }
                return true;
            }
        }
        false
    }
    
    /// 处理 Modal 按钮释放
    fn handle_modal_release(&mut self, x: f32, y: f32) {
        let pressed = self.modal.as_ref().and_then(|m| m.pressed_button.clone());
        
        // 清除按下状态
        if let Some(m) = &mut self.modal {
            m.pressed_button = None;
        }
        
        let modal = match &self.modal {
            Some(m) if m.visible => m,
            _ => return,
        };
        
        let sf = self.scale_factor as f32;
        let modal_width = 280.0 * sf;
        let modal_padding = 24.0 * sf;
        let title_font_size = 17.0 * sf;
        let content_font_size = 14.0 * sf;
        let button_height = 50.0 * sf;
        let gap = 16.0 * sf;
        
        let title_line_height = title_font_size * 1.4;
        let content_max_width = modal_width - modal_padding * 2.0;
        let content_lines = if let Some(tr) = &self.text_renderer {
            let text_width = tr.measure_text(&modal.content, content_font_size);
            ((text_width / content_max_width).ceil() as i32).max(1)
        } else { 1 };
        let content_line_height = content_font_size * 1.6 * content_lines as f32;
        
        let modal_height = modal_padding + title_line_height + gap + content_line_height + gap + button_height;
        
        let modal_x = (LOGICAL_WIDTH as f32 * sf - modal_width) / 2.0 / sf;
        let modal_y = (LOGICAL_HEIGHT as f32 * sf - modal_height) / 2.0 / sf;
        let button_y = modal_y + (modal_height - button_height) / sf;
        let button_h = button_height / sf;
        let modal_w = modal_width / sf;
        
        // 检查释放位置是否仍在按钮区域
        if y >= button_y && y <= button_y + button_h && x >= modal_x && x <= modal_x + modal_w {
            let show_cancel = modal.show_cancel;
            
            let clicked_button = if show_cancel {
                let button_width = modal_w / 2.0;
                if x < modal_x + button_width { "cancel" } else { "confirm" }
            } else {
                "confirm"
            };
            
            // 只有当释放位置与按下位置相同时才触发
            if pressed.as_deref() == Some(clicked_button) {
                if clicked_button == "cancel" {
                    println!("Modal: 取消");
                    self.app.eval("if(__modalCallback) __modalCallback({ confirm: false, cancel: true })").ok();
                } else {
                    println!("Modal: 确认");
                    self.app.eval("if(__modalCallback) __modalCallback({ confirm: true, cancel: false })").ok();
                }
                
                // 关闭 Modal
                self.modal = None;
            }
        }
        
        self.needs_redraw = true;
        if let Some(w) = &self.window { w.request_redraw(); }
    }
    
    /// 处理 Modal 点击（兼容旧逻辑）
    fn handle_modal_click(&mut self, x: f32, y: f32) {
        self.handle_modal_release(x, y);
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
                        pressed_button: None,
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
                        
                        // 如果有 Modal 显示，优先处理 Modal 按钮按下
                        if self.modal.as_ref().map(|m| m.visible).unwrap_or(false) {
                            self.handle_modal_press(x, y);
                            return;
                        }
                        
                        // 如果有 Loading 显示，忽略点击
                        if self.loading.as_ref().map(|l| l.visible).unwrap_or(false) {
                            return;
                        }
                        
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
                        // 如果有 Modal 按钮被按下，处理释放
                        if self.modal.as_ref().map(|m| m.visible && m.pressed_button.is_some()).unwrap_or(false) {
                            self.handle_modal_release(self.mouse_pos.0, self.mouse_pos.1);
                            return;
                        }
                        
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Mini App Engine - Multi-page Navigation\n");
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = MiniAppWindow::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
