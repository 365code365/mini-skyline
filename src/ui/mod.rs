//! UI 组件系统

mod component;
mod view;
mod text;
mod button;
mod image;
mod scroll_view;
mod layout;
pub mod interaction;

pub use component::{Component, ComponentId, ComponentTree, Style};
pub use view::View;
pub use text::Text;
pub use button::Button;
pub use image::Image;
pub use scroll_view::ScrollView;
pub use layout::{Layout, FlexDirection, FlexAlign};
pub use interaction::{InteractionManager, InteractiveElement, InteractionType, InteractionResult, KeyInput, ComponentState};
