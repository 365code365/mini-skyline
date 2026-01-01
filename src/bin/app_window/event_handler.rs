//! 窗口事件处理模块

use std::sync::Arc;
use std::time::Instant;
use winit::event::MouseScrollDelta;
use winit::window::Window;

use mini_render::runtime::UiEvent;
use mini_render::ui::interaction::InteractionType;

use super::{NavigationRequest, ui_overlay::{ToastState, LoadingState, ModalState}};
use super::events::keyboard;
use super::interaction_handler::{handle_interaction_result, print_js_output};

/// 处理 UI 事件（Toast/Loading/Modal）
pub fn process_ui_events(
    app: &mut mini_render::runtime::MiniApp,
    toast: &mut Option<ToastState>,
    loading: &mut Option<LoadingState>,
    modal: &mut Option<ModalState>,
) -> bool {
    let events = app.drain_ui_events();
    let mut needs_redraw = false;
    
    for event in events {
        match event {
            UiEvent::ShowToast { title, icon, duration } => {
                *toast = Some(ToastState {
                    title,
                    icon,
                    visible: true,
                    start_time: Instant::now(),
                    duration_ms: duration,
                });
                needs_redraw = true;
            }
            UiEvent::HideToast => {
                if let Some(t) = toast {
                    t.visible = false;
                }
                needs_redraw = true;
            }
            UiEvent::ShowLoading { title } => {
                *loading = Some(LoadingState {
                    title,
                    visible: true,
                });
                needs_redraw = true;
            }
            UiEvent::HideLoading => {
                if let Some(l) = loading {
                    l.visible = false;
                }
                needs_redraw = true;
            }
            UiEvent::ShowModal { title, content, show_cancel, cancel_text, confirm_text } => {
                *modal = Some(ModalState {
                    title,
                    content,
                    show_cancel,
                    cancel_text,
                    confirm_text,
                    visible: true,
                    pressed_button: None,
                });
                needs_redraw = true;
            }
            UiEvent::HideModal => {
                if let Some(m) = modal {
                    m.visible = false;
                }
                needs_redraw = true;
            }
        }
    }
    needs_redraw
}

/// 更新 Toast 超时
pub fn update_toast_timeout(toast: &mut Option<ToastState>) -> bool {
    if let Some(t) = toast {
        if t.visible {
            let elapsed = t.start_time.elapsed().as_millis() as u32;
            if elapsed >= t.duration_ms {
                *toast = None;
                return true;
            }
        }
    }
    false
}

/// 处理滚动事件
pub fn handle_scroll_event(
    event: mini_render::ui::scroll_controller::ScrollEvent,
    app: &mut mini_render::runtime::MiniApp,
) {
    use mini_render::ui::scroll_controller::ScrollEvent;
    
    match event {
        ScrollEvent::ReachBottom => {
            println!("📜 onReachBottom triggered");
            let call_code = "if(__currentPage && __currentPage.onReachBottom) __currentPage.onReachBottom()";
            app.eval(call_code).ok();
            print_js_output(app);
        }
        ScrollEvent::ReachTop => {
            println!("📜 onPullDownRefresh triggered");
            let call_code = "if(__currentPage && __currentPage.onPullDownRefresh) __currentPage.onPullDownRefresh()";
            app.eval(call_code).ok();
            print_js_output(app);
        }
    }
}

/// 处理键盘输入
pub fn handle_keyboard_event(
    event: winit::event::KeyEvent,
    modifiers: winit::keyboard::ModifiersState,
    interaction: &mut mini_render::ui::interaction::InteractionManager,
    clipboard: &mut Option<arboard::Clipboard>,
    window: Option<&Arc<Window>>,
    renderer: Option<&mini_render::renderer::WxmlRenderer>,
    app: &mut mini_render::runtime::MiniApp,
    scroll: &mut mini_render::ui::ScrollController,
    scale_factor: f64,
) -> (bool, Option<NavigationRequest>, bool) {
    use winit::keyboard::ModifiersState;
    
    let mut needs_redraw = false;
    let mut pending_nav = None;
    let mut exit_requested = false;
    
    if event.state != ElementState::Pressed {
        return (needs_redraw, pending_nav, exit_requested);
    }
    
    let ctrl = modifiers.contains(ModifiersState::CONTROL) || modifiers.contains(ModifiersState::SUPER);
    
    // 处理输入框文本输入
    if interaction.has_focused_input() {
        let (handled, result) = keyboard::handle_keyboard_input(
            event.physical_key,
            modifiers,
            interaction,
            clipboard,
        );
        
        if let Some(result) = result {
            handle_interaction_result(
                &result,
                window,
                renderer,
                app,
                clipboard,
                scroll.get_position(),
                scale_factor,
            );
        }
        
        // 处理文本输入
        if !ctrl {
            if let Some(ref text) = event.text {
                let results = keyboard::handle_text_input(text, ctrl, interaction);
                for result in results {
                    handle_interaction_result(
                        &result,
                        window,
                        renderer,
                        app,
                        clipboard,
                        scroll.get_position(),
                        scale_factor,
                    );
                }
            }
        }
        
        if handled {
            needs_redraw = true;
            return (needs_redraw, pending_nav, exit_requested);
        }
    }
    
    // 默认键盘处理
    if let Some(action) = keyboard::handle_default_keyboard(event.physical_key, interaction) {
        match action {
            keyboard::DefaultKeyAction::Exit => exit_requested = true,
            keyboard::DefaultKeyAction::NavigateBack => {
                pending_nav = Some(NavigationRequest::NavigateBack);
            }
            keyboard::DefaultKeyAction::BlurInput => {
                if let Some(result) = interaction.blur_input() {
                    handle_interaction_result(
                        &result,
                        window,
                        renderer,
                        app,
                        clipboard,
                        scroll.get_position(),
                        scale_factor,
                    );
                }
                needs_redraw = true;
            }
            keyboard::DefaultKeyAction::ScrollUp => scroll.handle_scroll(8.0, false),
            keyboard::DefaultKeyAction::ScrollDown => scroll.handle_scroll(-8.0, false),
            keyboard::DefaultKeyAction::PageUp => scroll.handle_scroll(30.0, false),
            keyboard::DefaultKeyAction::PageDown => scroll.handle_scroll(-30.0, false),
        }
    }
    
    (needs_redraw, pending_nav, exit_requested)
}

/// 处理 IME 事件
pub fn handle_ime_event(
    ime_event: winit::event::Ime,
    interaction: &mut mini_render::ui::interaction::InteractionManager,
    window: Option<&Arc<Window>>,
    renderer: Option<&mini_render::renderer::WxmlRenderer>,
    app: &mut mini_render::runtime::MiniApp,
    clipboard: &mut Option<arboard::Clipboard>,
    scroll_pos: f32,
    scale_factor: f64,
) -> bool {
    let results = ime::handle_ime_event(ime_event, interaction);
    let has_results = !results.is_empty();
    
    for result in results {
        handle_interaction_result(
            &result,
            window,
            renderer,
            app,
            clipboard,
            scroll_pos,
            scale_factor,
        );
    }
    
    has_results
}

/// 处理鼠标移动
pub fn handle_cursor_moved(
    x: f32, y: f32,
    interaction: &mut mini_render::ui::interaction::InteractionManager,
    scroll: &mut mini_render::ui::ScrollController,
    text_renderer: Option<&mini_render::text::TextRenderer>,
    window: Option<&Arc<Window>>,
    renderer: Option<&mini_render::renderer::WxmlRenderer>,
    app: &mut mini_render::runtime::MiniApp,
    clipboard: &mut Option<arboard::Clipboard>,
    scale_factor: f64,
) -> bool {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    let mut needs_redraw = false;
    
    // 处理文本选择拖动
    if interaction.is_selecting() {
        if let Some(focused) = &interaction.focused_input {
            if let Some(tr) = text_renderer {
                let sf = scale_factor as f32;
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
                
                interaction.update_text_selection(cursor_pos);
                needs_redraw = true;
            }
        }
    } else if interaction.is_dragging_slider() {
        if let Some(result) = interaction.handle_mouse_move(x, y + scroll.get_position()) {
            handle_interaction_result(
                &result,
                window,
                renderer,
                app,
                clipboard,
                scroll.get_position(),
                scale_factor,
            );
        }
        needs_redraw = true;
    } else if let Some(id) = interaction.dragging_scroll_area.clone() {
        if let Some(controller) = interaction.get_scroll_controller_mut(&id) {
            controller.update_drag(y, timestamp);
        }
    } else if scroll.is_dragging {
        scroll.update_drag(y, timestamp);
    }
    
    needs_redraw
}

/// 处理鼠标滚轮
pub fn handle_mouse_wheel(
    delta: MouseScrollDelta,
    mouse_pos: (f32, f32),
    interaction: &mut mini_render::ui::interaction::InteractionManager,
    scroll: &mut mini_render::ui::ScrollController,
    scale_factor: f64,
) -> bool {
    let (delta_y, is_precise) = match delta {
        MouseScrollDelta::LineDelta(_, y) => (-y * 20.0, false),
        MouseScrollDelta::PixelDelta(pos) => (-pos.y as f32 / scale_factor as f32, true),
    };
    
    if delta_y.abs() < 0.1 {
        return false;
    }
    
    let x = mouse_pos.0;
    let y = mouse_pos.1;
    let actual_y = y + scroll.get_position();
    
    let mut handled_by_scrollview = false;
    let mut needs_redraw = false;
    
    // 首先检查 fixed 元素
    let mut scroll_area_id = if let Some(element) = interaction.hit_test(x, y) {
        if element.is_fixed && element.interaction_type == InteractionType::ScrollArea {
            Some(element.id.clone())
        } else {
            None
        }
    } else {
        None
    };
    
    // 检查普通元素
    if scroll_area_id.is_none() {
        if let Some(element) = interaction.hit_test(x, actual_y) {
            if !element.is_fixed && element.interaction_type == InteractionType::ScrollArea {
                scroll_area_id = Some(element.id.clone());
            }
        }
    }
    
    if let Some(id) = scroll_area_id {
        if let Some(controller) = interaction.get_scroll_controller_mut(&id) {
            controller.handle_scroll(delta_y, is_precise);
            handled_by_scrollview = true;
            needs_redraw = true;
        }
    }
    
    if !handled_by_scrollview {
        scroll.handle_scroll(delta_y, is_precise);
    }
    
    needs_redraw
}
