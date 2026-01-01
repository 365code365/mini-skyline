//! 鼠标事件处理

use mini_render::ui::interaction::{InteractionManager, InteractionResult, InteractionType};
use mini_render::renderer::WxmlRenderer;
use mini_render::runtime::MiniApp;
use mini_render::ui::scroll_controller::ScrollController;
use super::super::tabbar::TABBAR_HEIGHT;

pub const LOGICAL_HEIGHT: u32 = 667;

/// 鼠标按下事件处理
pub fn handle_mouse_pressed(
    x: f32,
    y: f32,
    scroll: &mut ScrollController,
    interaction: &mut InteractionManager,
    timestamp: u64,
) -> bool {
    let mouse_pos = (x, y);
    // 考虑滚动偏移
    let actual_y = y + scroll.get_position();
    
    // 首先检查固定元素（使用原始坐标）
    if let Some(element) = interaction.hit_test(x, y) {
        let element = element.clone();
        if element.is_fixed {
            match element.interaction_type {
                InteractionType::Button => {
                    if !element.disabled {
                        interaction.set_button_pressed(element.id.clone(), element.bounds);
                        return true;
                    }
                }
                InteractionType::Switch | InteractionType::Checkbox | InteractionType::Radio => {
                    if !element.disabled {
                        if let Some(_result) = interaction.handle_click(x, y) { // Fixed elements use screen coords
                            return true;
                        }
                    }
                }
                _ => {}
            }
            return true; // Fixed element consumed click
        }
    }

    // 然后检查普通元素（使用滚动后的坐标）
    if let Some(element) = interaction.hit_test(x, actual_y) {
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
            InteractionType::ScrollArea => {
                if !element.is_fixed {
                    if let Some(controller) = interaction.get_scroll_controller_mut(&element.id) {
                        controller.begin_drag(y, timestamp);
                        interaction.dragging_scroll_area = Some(element.id.clone());
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    
    // 如果不是在拖动滑块或 ScrollArea，才开始滚动拖动
    if !interaction.is_dragging_slider() && interaction.dragging_scroll_area.is_none() {
        scroll.begin_drag(mouse_pos.1, timestamp);
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
    
    // 结束 ScrollArea 拖动
    if let Some(id) = &interaction.dragging_scroll_area.clone() {
        if let Some(controller) = interaction.get_scroll_controller_mut(id) {
            controller.end_drag();
        }
        interaction.dragging_scroll_area = None;
        return true; // 触发重绘
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
    
    // 检查是否点击在 scroll-view 内部，如果是，需要调整坐标
    let mut adjusted_y = actual_y;
    if let Some(element) = interaction.hit_test(x, actual_y) {
        if element.interaction_type == InteractionType::ScrollArea {
            // 点击在 scroll-view 上，需要加上 scroll-view 的滚动偏移
            if let Some(controller) = interaction.get_scroll_controller(&element.id) {
                let scroll_offset = controller.get_position();
                // 计算相对于 scroll-view 内部的坐标
                adjusted_y = actual_y + scroll_offset;
            }
        }
    }
    
    // 使用交互管理器处理点击
    if let Some(result) = interaction.handle_click(x, adjusted_y) {
        // 处理输入框光标位置
        if let InteractionResult::Focus { click_x, .. } = &result {
            if let Some(focused) = &interaction.focused_input {
                if let Some(tr) = text_renderer {
                    // click_x 是逻辑坐标（相对于输入框左边缘）
                    // 需要转换为物理坐标来匹配 measure_text 的结果
                    let sf = scale_factor as f32;
                    let font_size = 16.0 * sf;
                    let padding_left = 12.0 * sf;
                    let click_x_physical = *click_x * sf;
                    
                    let mut char_widths = Vec::new();
                    for c in focused.value.chars() {
                        let char_str = c.to_string();
                        let width = tr.measure_text(&char_str, font_size);
                        char_widths.push(width);
                    }
                    
                    use mini_render::ui::interaction::calculate_cursor_position;
                    let cursor_pos = calculate_cursor_position(&focused.value, &char_widths, click_x_physical, padding_left);
                    
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
                if let Some(binding) = renderer.hit_test(x, adjusted_y) {
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
        if let Some(binding) = renderer.hit_test(x, adjusted_y) {
            println!("👆 {} -> {}", binding.event_type, binding.handler);
            let data_json = serde_json::to_string(&binding.data).unwrap_or("{}".to_string());
            let call_code = format!("__callPageMethod('{}', {})", binding.handler, data_json);
            app.eval(&call_code).ok();
        }
    }
    
    None
}
