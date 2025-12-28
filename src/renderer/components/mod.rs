//! 微信小程序组件实现
//! 每个组件独立文件，便于维护

mod base;
mod view;
mod text;
mod button;
mod icon;
mod progress;
mod switch;
mod checkbox;
mod radio;
mod slider;
mod input;
mod image;

pub use base::*;
pub use view::ViewComponent;
pub use text::TextComponent;
pub use button::ButtonComponent;
pub use icon::IconComponent;
pub use progress::ProgressComponent;
pub use switch::SwitchComponent;
pub use checkbox::CheckboxComponent;
pub use radio::RadioComponent;
pub use slider::SliderComponent;
pub use input::InputComponent;
pub use image::ImageComponent;

use crate::parser::wxml::WxmlNode;
use crate::parser::wxss::StyleSheet;
use taffy::prelude::*;

/// 组件注册表
pub struct ComponentRegistry {
    scale_factor: f32,
    screen_width: f32,
    screen_height: f32,
}

impl ComponentRegistry {
    pub fn new(scale_factor: f32, screen_width: f32, screen_height: f32) -> Self {
        Self { scale_factor, screen_width, screen_height }
    }
    
    /// 根据标签名构建组件
    pub fn build_component(
        &self,
        node: &WxmlNode,
        stylesheet: &StyleSheet,
        taffy: &mut TaffyTree,
    ) -> Option<RenderNode> {
        let tag = node.tag_name.as_str();
        let mut ctx = ComponentContext {
            scale_factor: self.scale_factor,
            screen_width: self.screen_width,
            screen_height: self.screen_height,
            stylesheet,
            taffy,
        };
        
        match tag {
            "view" | "block" | "scroll-view" => ViewComponent::build(node, &mut ctx),
            "text" => TextComponent::build(node, &mut ctx),
            "button" => ButtonComponent::build(node, &mut ctx),
            "icon" => IconComponent::build(node, &mut ctx),
            "progress" => ProgressComponent::build(node, &mut ctx),
            "switch" => SwitchComponent::build(node, &mut ctx),
            "checkbox" => CheckboxComponent::build(node, &mut ctx),
            "radio" => RadioComponent::build(node, &mut ctx),
            "slider" => SliderComponent::build(node, &mut ctx),
            "input" | "textarea" => InputComponent::build(node, &mut ctx),
            "image" => ImageComponent::build(node, &mut ctx),
            // 默认作为 view 处理
            _ => ViewComponent::build(node, &mut ctx),
        }
    }
}
