//! 窗口模块 - 将事件处理逻辑拆分成独立模块

pub mod config;
// pub mod scroll; // Moved to mini_render::ui::scroll_controller
pub mod navigation;
pub mod tabbar;
pub mod events;
pub mod render;
pub mod interaction_handler;

pub use config::*;
// pub use scroll::ScrollController;
// pub use mini_render::ui::scroll_controller::ScrollController;
pub use navigation::*;
pub use tabbar::*;
pub use render::*;
pub use interaction_handler::*;

// 常量
pub const LOGICAL_WIDTH: u32 = 375;
pub const LOGICAL_HEIGHT: u32 = 667;
pub const CONTENT_HEIGHT: u32 = 1500;
