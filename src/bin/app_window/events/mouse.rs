//! 鼠标事件处理

use mini_render::ui::interaction::{InteractionManager, InteractionResult, InteractionType};
use mini_render::renderer::WxmlRenderer;
use mini_render::runtime::MiniApp;
use super::super::scroll::ScrollController;
use super::super::navigation::NavigationRequest;
use super::super::tabbar::TABBAR_HEIGHT;

pub const LOGICAL_HEIGHT: u32 = 667;

/// 鼠标按下事件处理
pub fn handle_mouse_pressed(
    mouse_pos: (f32, f32),
    scroll: &mut ScrollController,
    interaction: &mut InteractionManager,
) -> bool {
    let x = mouse_pos.0;
    let y = mouse_pos.1;
    let actual_y = y + scroll.get_position();
    
    // 首先检查 fixed 元素（使用视口坐标）
    if let Some(element) = interaction.hit_test(x, y) {
        let element = element.clone();
        
        match element.interaction_type {
            InteractionType::Slider => {
                if !element.disabled {
                    if let Some(result) = interaction.handle_click(x, y) {
                        return true; // 需要处理结果
                    }
                }
                return true; // 阻止滚动
            }
            InteractionType::Button => {
                if !element.disabled {
                    interaction.set_button_pressed(element.id.clone(), element.bounds);
                    return true;
                }
            }
            _ => {}
        }
    }
    // 然后检查普通元素（使用滚动后的坐标）
    else if let Some(element) = interaction.hit_test(x, actual_y) {
        let element = element.clone();
        
        match element.interaction_type {
            InteractionType::Slider => {
                if !element.disabled {
                    if let Some(_result) = interaction.handle_click(x, actual_y) {
                        return true;
                    }
                }
                return true;
            }
            InteractionType::Button => {
                if !element.disabled {
                    interaction.set_button_pressed(element.id.clone(), element.bounds);
                    return true;
                }
            }
            _ => {}
        }
    }
    
    // 如果不是在拖动滑块，才开始滚动拖动
    if !interaction.is_dragging_slider() {
        scroll.begin_drag(mouse_pos.1);
    }
    
    false
}

/// 鼠标释放事件处理
pub fn handle_mouse_released(
    scroll: &mut ScrollController,
    interaction: &mut InteractionManager,
) -> bool {
    // 清除按钮按下状态
    interaction.clear_button_pressed();
    
    // 结束滑块拖动
    if let Some(_result) = interaction.handle_mouse_release() {
        // 结果会在外部处理
    }
    
    scroll.end_drag()
}

/// 处理内容区域点击
pub fn handle_content_click(
    x: f32,
    y: f32,
    scroll_pos: f32,
    has_tabbar: bool,
    interaction: &mut InteractionManager,
    renderer: Option<&WxmlRenderer>,
    app: &mut MiniApp,
    scale_factor: f64,
    text_renderer: Option<&mini_render::text::TextRenderer>,
) -> Option<InteractionResult> {
    let actual_y = y + scroll_pos;
    let tabbar_y = if has_tabbar { (LOGICAL_HEIGHT - TABBAR_HEIGHT) as f32 } else { LOGICAL_HEIGHT as f32 };
    
    // 首先检查 fixed 元素（使用视口坐标）
    let fixed_binding = if let Some(renderer) = renderer {
        if let Some(binding) = renderer.hit_test(x, y) {
            let viewport_height = if has_tabbar { tabbar_y } else { LOGICAL_HEIGHT as f32 };
            if binding.bounds.y >= 0.0 && binding.bounds.y + binding.bounds.height <= viewport_height + 10.0 {
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
        if let Some(result) = interaction.handle_click(x, y) {
            let should_call_js = matches!(&result, 
                InteractionResult::ButtonClick { .. } |
                InteractionResult::Toggle { .. } |
                InteractionResult::Select { .. }
            );
            
            if should_call_js {
                println!("👆 {} -> {}", event_type, handler);
                let data_json = serde_json::to_string(&data).unwrap_or("{}".to_string());
                let call_code = format!("__callPageMethod('{}', {})", handler, data_json);
                app.eval(&call_code).ok();
            }
            
            return Some(result);
        }
        
        // 如果没有交互元素，直接调用事件处理
        println!("👆 {} -> {}", event_type, handler);
        let data_json = serde_json::to_string(&data).unwrap_or("{}".to_string());
        let call_code = format!("__callPageMethod('{}', {})", handler, data_json);
        app.eval(&call_code).ok();
        return None;
    }
    
    // 使用交互管理器处理点击
    if let Some(result) = interaction.handle_click(x, actual_y) {
        // 处理输入框光标位置
        if let InteractionResult::Focus { click_x, .. } = &result {
            if let Some(focused) = &interaction.focused_input {
                if let Some(tr) = text_renderer {
                    let font_size = (16.0 * scale_factor) as f32;
                    let padding_left = (12.0 * scale_factor) as f32;
                    
                    let mut char_widths = Vec::new();
                    for c in focused.value.chars() {
                        let char_str = c.to_string();
                        let width = tr.measure_text(&char_str, font_size);
                        char_widths.push(width);
                    }
                    
                    use mini_render::ui::interaction::calculate_cursor_position;
                    let cursor_pos = calculate_cursor_position(&focused.value, &char_widths, *click_x as f32, padding_left);
                    
                    if let Some(input) = &mut interaction.focused_input {
                        input.cursor_pos = cursor_pos;
                    }
                }
            }
        }
        
        let should_call_js = matches!(&result,
            InteractionResult::ButtonClick { .. } |
            InteractionResult::Toggle { .. } |
            InteractionResult::Select { .. } |
            InteractionResult::Focus { .. }
        );
        
        if should_call_js {
            if let Some(renderer) = renderer {
                if let Some(binding) = renderer.hit_test(x, actual_y) {
                    println!("👆 {} -> {}", binding.event_type, binding.handler);
                    let data_json = serde_json::to_string(&binding.data).unwrap_or("{}".to_string());
                    let call_code = format!("__callPageMethod('{}', {})", binding.handler, data_json);
                    app.eval(&call_code).ok();
                }
            }
        }
        
        return Some(result);
    } else {
        // 点击了非交互区域，让输入框失去焦点
        if interaction.has_focused_input() {
            return interaction.blur_input();
        }
    }
    
    // 检查其他事件绑定
    if let Some(renderer) = renderer {
        if let Some(binding) = renderer.hit_test(x, actual_y) {
            println!("👆 {} -> {}", binding.event_type, binding.handler);
            let data_json = serde_json::to_string(&binding.data).unwrap_or("{}".to_string());
            let call_code = format!("__callPageMethod('{}', {})", binding.handler, data_json);
            app.eval(&call_code).ok();
        }
    }
    
    None
}
