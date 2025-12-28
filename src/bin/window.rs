//! 带窗口的小程序运行器 - 支持高清渲染

use mini_render::runtime::MiniApp;
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::renderer::WxmlRenderer;
use mini_render::{Canvas, Color};
use serde_json::json;
use std::num::NonZeroU32;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

// 逻辑尺寸 (CSS 像素)
const LOGICAL_WIDTH: u32 = 375;
const LOGICAL_HEIGHT: u32 = 667;

struct MiniAppWindow {
    window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    app: MiniApp,
    canvas: Option<Canvas>,
    renderer: Option<WxmlRenderer>,
    wxml_nodes: Vec<mini_render::parser::wxml::WxmlNode>,
    stylesheet: mini_render::parser::wxss::StyleSheet,
    mouse_pos: (f32, f32),
    needs_redraw: bool,
    scale_factor: f64,
}

impl MiniAppWindow {
    fn new() -> Result<Self, String> {
        // 创建应用
        let mut app = MiniApp::new(LOGICAL_WIDTH, LOGICAL_HEIGHT)?;
        app.init()?;
        
        // 加载页面 JS
        let page_js = include_str!("../../sample-app/pages/index/index.js");
        app.load_script(page_js)?;
        
        // 解析 WXML
        let wxml_content = include_str!("../../sample-app/pages/index/index.wxml");
        let mut wxml_parser = WxmlParser::new(wxml_content);
        let wxml_nodes = wxml_parser.parse().map_err(|e| format!("WXML parse error: {}", e))?;
        println!("✅ WXML parsed: {} root nodes", wxml_nodes.len());
        
        // 解析 WXSS
        let wxss_content = include_str!("../../sample-app/pages/index/index.wxss");
        let mut wxss_parser = WxssParser::new(wxss_content);
        let stylesheet = wxss_parser.parse().map_err(|e| format!("WXSS parse error: {}", e))?;
        println!("✅ WXSS parsed: {} rules", stylesheet.rules.len());
        
        Ok(Self {
            window: None,
            surface: None,
            app,
            canvas: None,
            renderer: None,
            wxml_nodes,
            stylesheet,
            mouse_pos: (0.0, 0.0),
            needs_redraw: true,
            scale_factor: 1.0,
        })
    }
    
    fn setup_canvas(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
        
        // 物理像素尺寸 = 逻辑尺寸 * 缩放因子
        let physical_width = (LOGICAL_WIDTH as f64 * scale_factor) as u32;
        let physical_height = (LOGICAL_HEIGHT as f64 * scale_factor) as u32;
        
        println!("📐 Scale factor: {}", scale_factor);
        println!("   Logical size: {}x{}", LOGICAL_WIDTH, LOGICAL_HEIGHT);
        println!("   Physical size: {}x{}", physical_width, physical_height);
        
        // 创建高分辨率画布
        self.canvas = Some(Canvas::new(physical_width, physical_height));
        
        // 创建渲染器 - 使用物理像素尺寸和缩放因子
        self.renderer = Some(WxmlRenderer::new_with_scale(
            self.stylesheet.clone(),
            LOGICAL_WIDTH as f32,
            LOGICAL_HEIGHT as f32,
            scale_factor as f32,
        ));
    }
    
    fn render(&mut self) {
        let (canvas, renderer) = match (&mut self.canvas, &mut self.renderer) {
            (Some(c), Some(r)) => (c, r),
            _ => return,
        };
        
        // 获取页面数据
        let page_data = if let Ok(data_str) = self.app.eval("__getPageData()") {
            serde_json::from_str(&data_str).unwrap_or(json!({}))
        } else {
            json!({ "count": 0 })
        };
        
        // 清空画布
        canvas.clear(Color::from_hex(0xF5F5F5));
        
        // 渲染 WXML
        renderer.render(canvas, &self.wxml_nodes, &page_data);
    }
    
    fn present(&mut self) {
        let canvas = match &self.canvas {
            Some(c) => c,
            None => return,
        };
        
        if let (Some(window), Some(surface)) = (&self.window, &mut self.surface) {
            let size = window.inner_size();
            if let (Some(win_width), Some(win_height)) = (
                NonZeroU32::new(size.width),
                NonZeroU32::new(size.height),
            ) {
                surface.resize(win_width, win_height).ok();
                
                if let Ok(mut buffer) = surface.buffer_mut() {
                    let canvas_data = canvas.to_rgba();
                    let canvas_width = canvas.width();
                    let canvas_height = canvas.height();
                    
                    // 直接 1:1 复制（Canvas 已经是物理像素大小）
                    for y in 0..size.height.min(canvas_height) {
                        for x in 0..size.width.min(canvas_width) {
                            let src_idx = ((y * canvas_width + x) * 4) as usize;
                            let dst_idx = (y * size.width + x) as usize;
                            
                            if src_idx + 3 < canvas_data.len() && dst_idx < buffer.len() {
                                let r = canvas_data[src_idx] as u32;
                                let g = canvas_data[src_idx + 1] as u32;
                                let b = canvas_data[src_idx + 2] as u32;
                                buffer[dst_idx] = (r << 16) | (g << 8) | b;
                            }
                        }
                    }
                    
                    buffer.present().ok();
                }
            }
        }
    }
    
    fn handle_click(&mut self, logical_x: f32, logical_y: f32) {
        println!("🖱️ Click at ({:.0}, {:.0})", logical_x, logical_y);
        
        let renderer = match &self.renderer {
            Some(r) => r,
            None => return,
        };
        
        if let Some(binding) = renderer.hit_test(logical_x, logical_y) {
            println!("   → Event: {} -> {}", binding.event_type, binding.handler);
            
            let data_json = serde_json::to_string(&binding.data).unwrap_or("{}".to_string());
            let call_code = format!("__callPageMethod('{}', {})", binding.handler, data_json);
            
            if let Err(e) = self.app.eval(&call_code) {
                println!("   ⚠️ Handler error: {}", e);
            }
            
            if let Ok(output) = self.app.eval("__print_buffer.splice(0).join('\\n')") {
                if !output.is_empty() && output != "undefined" {
                    for line in output.lines() {
                        println!("   📝 {}", line);
                    }
                }
            }
            
            self.needs_redraw = true;
        }
    }
}

impl ApplicationHandler for MiniAppWindow {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attrs = WindowAttributes::default()
                .with_title("Mini App - 小程序引擎")
                .with_inner_size(winit::dpi::LogicalSize::new(LOGICAL_WIDTH, LOGICAL_HEIGHT))
                .with_resizable(false);
            
            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
            
            // 获取设备像素比
            let scale_factor = window.scale_factor();
            self.setup_canvas(scale_factor);
            
            let context = softbuffer::Context::new(window.clone()).unwrap();
            let surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
            
            self.window = Some(window);
            self.surface = Some(surface);
            
            // 初始渲染
            self.render();
            self.present();
            
            println!("\n🎮 Mini App Window Ready!");
            println!("   点击按钮进行交互");
            println!("   按 ESC 或关闭窗口退出\n");
        }
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("\n👋 Goodbye!");
                event_loop.exit();
            }
            
            WindowEvent::KeyboardInput { event, .. } => {
                if event.physical_key == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape) {
                    println!("\n👋 Goodbye!");
                    event_loop.exit();
                }
            }
            
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                println!("📐 Scale factor changed to {}", scale_factor);
                self.setup_canvas(scale_factor);
                self.needs_redraw = true;
            }
            
            WindowEvent::CursorMoved { position, .. } => {
                // position 是物理像素，转换为逻辑像素
                self.mouse_pos = (
                    position.x as f32 / self.scale_factor as f32,
                    position.y as f32 / self.scale_factor as f32,
                );
            }
            
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                self.handle_click(self.mouse_pos.0, self.mouse_pos.1);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            
            WindowEvent::RedrawRequested => {
                if self.needs_redraw {
                    self.render();
                    self.needs_redraw = false;
                }
                self.present();
            }
            
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Mini App Engine Starting...\n");
    
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    
    let mut app = MiniAppWindow::new()?;
    
    println!("✅ Engine initialized");
    println!("✅ Page loaded\n");
    
    event_loop.run_app(&mut app)?;
    
    Ok(())
}
