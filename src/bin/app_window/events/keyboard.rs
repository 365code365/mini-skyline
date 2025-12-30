//! 键盘事件处理

use mini_render::ui::interaction::{InteractionManager, InteractionResult, KeyInput};
use winit::keyboard::{PhysicalKey, KeyCode, ModifiersState};

/// 处理键盘输入，返回是否已处理和可能的交互结果
pub fn handle_keyboard_input(
    physical_key: PhysicalKey,
    modifiers: ModifiersState,
    interaction: &mut InteractionManager,
    clipboard: &mut Option<arboard::Clipboard>,
) -> (bool, Option<InteractionResult>) {
    let ctrl = modifiers.contains(ModifiersState::CONTROL) || modifiers.contains(ModifiersState::SUPER);
    let shift = modifiers.contains(ModifiersState::SHIFT);
    
    if !interaction.has_focused_input() {
        return (false, None);
    }
    
    let key_input = if let PhysicalKey::Code(code) = physical_key {
        match code {
            KeyCode::Backspace => Some(KeyInput::Backspace),
            KeyCode::Delete => Some(KeyInput::Delete),
            KeyCode::ArrowLeft if shift => Some(KeyInput::ShiftLeft),
            KeyCode::ArrowRight if shift => Some(KeyInput::ShiftRight),
            KeyCode::ArrowLeft => Some(KeyInput::Left),
            KeyCode::ArrowRight => Some(KeyInput::Right),
            KeyCode::Home if shift => Some(KeyInput::ShiftHome),
            KeyCode::End if shift => Some(KeyInput::ShiftEnd),
            KeyCode::Home => Some(KeyInput::Home),
            KeyCode::End => Some(KeyInput::End),
            KeyCode::Enter => Some(KeyInput::Enter),
            KeyCode::Escape => Some(KeyInput::Escape),
            KeyCode::KeyA if ctrl => Some(KeyInput::SelectAll),
            KeyCode::KeyC if ctrl => Some(KeyInput::Copy),
            KeyCode::KeyX if ctrl => Some(KeyInput::Cut),
            KeyCode::KeyV if ctrl => {
                let text = clipboard.as_mut()
                    .and_then(|cb| cb.get_text().ok())
                    .unwrap_or_default();
                Some(KeyInput::Paste(text))
            }
            _ => None
        }
    } else {
        None
    };
    
    if let Some(ki) = key_input {
        let result = interaction.handle_key_input(ki);
        return (true, result);
    }
    
    (false, None)
}

/// 处理文本输入字符
pub fn handle_text_input(
    text: &str,
    ctrl: bool,
    interaction: &mut InteractionManager,
) -> Vec<InteractionResult> {
    let mut results = Vec::new();
    
    if ctrl || !interaction.has_focused_input() {
        return results;
    }
    
    for c in text.chars() {
        if c.is_control() { continue; }
        if let Some(result) = interaction.handle_key_input(KeyInput::Char(c)) {
            results.push(result);
        }
    }
    
    results
}

/// 处理默认键盘事件（非输入框聚焦时）
pub fn handle_default_keyboard(
    physical_key: PhysicalKey,
    interaction: &mut InteractionManager,
) -> Option<DefaultKeyAction> {
    if let PhysicalKey::Code(code) = physical_key {
        match code {
            KeyCode::Escape => {
                if interaction.has_focused_input() {
                    return Some(DefaultKeyAction::BlurInput);
                } else {
                    return Some(DefaultKeyAction::Exit);
                }
            }
            KeyCode::Backspace => {
                if !interaction.has_focused_input() {
                    return Some(DefaultKeyAction::NavigateBack);
                }
            }
            KeyCode::ArrowUp => return Some(DefaultKeyAction::ScrollUp),
            KeyCode::ArrowDown => return Some(DefaultKeyAction::ScrollDown),
            KeyCode::PageUp => return Some(DefaultKeyAction::PageUp),
            KeyCode::PageDown => return Some(DefaultKeyAction::PageDown),
            _ => {}
        }
    }
    None
}

/// 默认键盘动作
pub enum DefaultKeyAction {
    Exit,
    NavigateBack,
    BlurInput,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
}
