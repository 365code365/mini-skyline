//! IME 输入处理

use mini_render::ui::interaction::{InteractionManager, InteractionResult, KeyInput};
use winit::event::Ime;

/// 处理 IME 事件
pub fn handle_ime_event(
    ime: Ime,
    interaction: &mut InteractionManager,
) -> Vec<InteractionResult> {
    let mut results = Vec::new();
    
    match ime {
        Ime::Commit(text) => {
            if interaction.has_focused_input() {
                for c in text.chars() {
                    if let Some(result) = interaction.handle_key_input(KeyInput::Char(c)) {
                        results.push(result);
                    }
                }
            }
        }
        Ime::Preedit(text, cursor) => {
            if !text.is_empty() {
                println!("📝 IME Preedit: {} {:?}", text, cursor);
            }
        }
        Ime::Enabled => {
            println!("📝 IME Enabled");
        }
        Ime::Disabled => {
            println!("📝 IME Disabled");
        }
    }
    
    results
}
