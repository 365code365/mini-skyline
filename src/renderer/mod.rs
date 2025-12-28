//! UI 渲染器 - 将 WXML/WXSS 渲染为 UI

mod wxml_renderer;
mod style_resolver;
pub mod components;

pub use wxml_renderer::{WxmlRenderer, EventBinding};
pub use style_resolver::StyleResolver;
pub use components::{RenderNode, NodeStyle, ComponentRegistry};
