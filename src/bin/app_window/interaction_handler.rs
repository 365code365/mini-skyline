//! 交互结果处理

use mini_render::ui::interaction::InteractionResult;
use mini_render::renderer::WxmlRenderer;
use mini_render::runtime::MiniApp;
use winit::window::Window;
use std::sync::Arc;

/// 处理交互结果
pub fn handle_interaction_result(
    result: &InteractionResult,
    window: Option<&Arc<Window>>,
    renderer: Option<&WxmlRenderer>,
    app: &mut MiniApp,
    clipboard: &mut Option<arboard::Clipboard>,
    scroll_position: f32,
    scale_factor: f64,
) {
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
        InteractionResult::Focus { id, bounds, click_x: _ } => {
            println!("📝 Focus: {} at ({:.0}, {:.0})", id, bounds.x, bounds.y);
            if let Some(window) = window {
                window.set_ime_allowed(true);
                let sf = scale_factor;
                let ime_x = (bounds.x * sf as f32) as f64;
                let ime_y = ((bounds.y - scroll_position + bounds.height + 5.0) * sf as f32) as f64;
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
            if let Some(renderer) = renderer {
                for binding in renderer.get_event_bindings() {
                    if binding.event_type == "input" {
                        let mut event_data = binding.data.clone();
                        event_data.insert("value".to_string(), value.clone());
                        let data_json = serde_json::to_string(&event_data).unwrap_or("{}".to_string());
                        let call_code = format!("__callPageMethod('{}', {})", binding.handler, data_json);
                        app.eval(&call_code).ok();
                        break;
                    }
                }
            }
        }
        InteractionResult::InputBlur { id, value } => {
            println!("📝 Blur {}: {}", id, value);
            if let Some(window) = window {
                window.set_ime_allowed(false);
            }
            if let Some(renderer) = renderer {
                for binding in renderer.get_event_bindings() {
                    if binding.event_type == "blur" {
                        let mut event_data = binding.data.clone();
                        event_data.insert("value".to_string(), value.clone());
                        let data_json = serde_json::to_string(&event_data).unwrap_or("{}".to_string());
                        let call_code = format!("__callPageMethod('{}', {})", binding.handler, data_json);
                        app.eval(&call_code).ok();
                        break;
                    }
                }
            }
        }
        InteractionResult::ButtonClick { id, bounds: _ } => {
            println!("🔘 Button clicked: {}", id);
        }
        InteractionResult::CopyText { text } => {
            println!("📋 Copy: {}", text);
            if let Some(ref mut cb) = clipboard {
                if let Err(e) = cb.set_text(text) {
                    println!("❌ Clipboard copy failed: {}", e);
                } else {
                    println!("✅ Copied to clipboard");
                }
            }
        }
        InteractionResult::CutText { text, id, value } => {
            println!("✂️ Cut from {}: {} (remaining: {})", id, text, value);
            if let Some(ref mut cb) = clipboard {
                if let Err(e) = cb.set_text(text) {
                    println!("❌ Clipboard cut failed: {}", e);
                } else {
                    println!("✅ Cut to clipboard");
                }
            }
        }
    }
}

/// 检查并获取导航请求
pub fn check_navigation(app: &mut MiniApp) -> Option<super::navigation::NavigationRequest> {
    use super::navigation::NavigationRequest;
    
    if let Ok(nav_str) = app.eval("JSON.stringify(__pendingNavigation || null)") {
        if nav_str != "null" && !nav_str.is_empty() {
            if let Ok(nav) = serde_json::from_str::<serde_json::Value>(&nav_str) {
                if let Some(nav_type) = nav.get("type").and_then(|v| v.as_str()) {
                    let url = nav.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    let result = match nav_type {
                        "navigateTo" => Some(NavigationRequest::NavigateTo { url: url.to_string() }),
                        "navigateBack" => Some(NavigationRequest::NavigateBack),
                        "switchTab" => Some(NavigationRequest::SwitchTab { url: url.to_string() }),
                        _ => None,
                    };
                    // 清除导航请求
                    app.eval("__pendingNavigation = null").ok();
                    return result;
                }
            }
        }
    }
    None
}

/// 打印 JS 输出
pub fn print_js_output(app: &MiniApp) {
    if let Ok(output) = app.eval("__print_buffer.splice(0).join('\\n')") {
        if !output.is_empty() && output != "undefined" {
            for line in output.lines() {
                println!("   {}", line);
            }
        }
    }
}
