//! WXML 渲染器 - 使用组件系统渲染微信小程序

use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::parser::wxss::StyleSheet;
use crate::text::TextRenderer;
use crate::ui::interaction::{InteractionManager, InteractiveElement, InteractionType};
use crate::{Canvas, Color, Rect as GeoRect};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use taffy::prelude::*;

use super::components::{
    RenderNode, NodeStyle, ComponentContext,
    ViewComponent, TextComponent, ButtonComponent, IconComponent,
    ProgressComponent, SwitchComponent, CheckboxComponent, RadioComponent,
    SliderComponent, InputComponent, ImageComponent,
    build_base_style,
};

#[derive(Debug, Clone)]
pub struct EventBinding {
    pub event_type: String,
    pub handler: String,
    pub data: HashMap<String, String>,
    pub bounds: GeoRect,
}

pub struct WxmlRenderer {
    stylesheet: StyleSheet,
    screen_width: f32,
    screen_height: f32,
    event_bindings: Vec<EventBinding>,
    text_renderer: Option<TextRenderer>,
    scale_factor: f32,
}

impl WxmlRenderer {
    pub fn new(stylesheet: StyleSheet, screen_width: f32, screen_height: f32) -> Self {
        Self::new_with_scale(stylesheet, screen_width, screen_height, 1.0)
    }
    
    pub fn new_with_scale(stylesheet: StyleSheet, screen_width: f32, screen_height: f32, scale_factor: f32) -> Self {
        let text_renderer = TextRenderer::load_system_font()
            .or_else(|_| TextRenderer::from_bytes(include_bytes!("../../assets/ArialUnicode.ttf")))
            .ok();
        
        Self { 
            stylesheet, 
            screen_width,
            screen_height,
            event_bindings: Vec::new(),
            text_renderer,
            scale_factor,
        }
    }

    /// 渲染 WXML 节点，使用交互管理器处理状态
    pub fn render_with_interaction(
        &mut self, 
        canvas: &mut Canvas, 
        nodes: &[WxmlNode], 
        data: &JsonValue,
        interaction: &mut InteractionManager,
    ) {
        self.event_bindings.clear();
        interaction.clear_elements();
        
        let rendered = crate::parser::TemplateEngine::render(nodes, data);
        let mut taffy = TaffyTree::new();
        
        let mut render_nodes = Vec::new();
        for node in &rendered {
            if let Some(rn) = self.build_tree(&mut taffy, node) {
                render_nodes.push(rn);
            }
        }
        
        let child_ids: Vec<NodeId> = render_nodes.iter().map(|n| n.taffy_node).collect();
        let root = taffy.new_with_children(
            Style {
                size: Size { width: length(self.screen_width * self.scale_factor), height: auto() },
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &child_ids,
        ).unwrap();
        
        taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
        
        for rn in &render_nodes {
            self.draw_with_interaction(canvas, &taffy, rn, 0.0, 0.0, interaction);
        }
    }
    
    /// 兼容旧接口
    pub fn render(&mut self, canvas: &mut Canvas, nodes: &[WxmlNode], data: &JsonValue) {
        self.event_bindings.clear();
        let rendered = crate::parser::TemplateEngine::render(nodes, data);
        let mut taffy = TaffyTree::new();
        
        let mut render_nodes = Vec::new();
        for node in &rendered {
            if let Some(rn) = self.build_tree(&mut taffy, node) {
                render_nodes.push(rn);
            }
        }
        
        let child_ids: Vec<NodeId> = render_nodes.iter().map(|n| n.taffy_node).collect();
        let root = taffy.new_with_children(
            Style {
                size: Size { width: length(self.screen_width * self.scale_factor), height: auto() },
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &child_ids,
        ).unwrap();
        
        taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
        
        for rn in &render_nodes {
            self.draw(canvas, &taffy, rn, 0.0, 0.0);
        }
    }

    fn build_tree(&self, taffy: &mut TaffyTree, node: &WxmlNode) -> Option<RenderNode> {
        let sf = self.scale_factor;
        
        if node.node_type == WxmlNodeType::Text {
            let text = node.text_content.trim();
            if text.is_empty() { return None; }
            let fs = 14.0;
            let tw = self.measure_text(text, fs * sf);
            let tn = taffy.new_leaf(Style {
                size: Size { width: length(tw), height: length((fs + 4.0) * sf) },
                ..Default::default()
            }).unwrap();
            return Some(RenderNode {
                tag: "#text".into(), 
                text: text.into(), 
                attrs: HashMap::new(),
                taffy_node: tn,
                style: NodeStyle { font_size: fs, text_color: Some(Color::BLACK), opacity: 1.0, ..Default::default() },
                children: vec![], 
                events: vec![],
            });
        }
        
        if node.node_type != WxmlNodeType::Element { return None; }

        let tag = node.tag_name.as_str();
        let mut ctx = ComponentContext {
            scale_factor: sf,
            screen_width: self.screen_width,
            screen_height: self.screen_height,
            stylesheet: &self.stylesheet,
            taffy,
        };
        
        let mut render_node = match tag {
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
            _ => ViewComponent::build(node, &mut ctx),
        };
        
        if let Some(ref mut rn) = render_node {
            if !Self::is_leaf_component(tag) {
                let mut children = vec![];
                for c in &node.children {
                    if let Some(cr) = self.build_tree(ctx.taffy, c) { 
                        children.push(cr); 
                    }
                }
                
                if !children.is_empty() {
                    let child_ids: Vec<NodeId> = children.iter().map(|c| c.taffy_node).collect();
                    let (ts, _) = build_base_style(node, &mut ctx);
                    let new_tn = ctx.taffy.new_with_children(ts, &child_ids).unwrap();
                    rn.taffy_node = new_tn;
                    rn.children = children;
                }
            }
        }
        
        render_node
    }
    
    fn is_leaf_component(tag: &str) -> bool {
        matches!(tag, 
            "text" | "button" | "icon" | "progress" | "switch" | 
            "checkbox" | "radio" | "slider" | "input" | "textarea" | "image"
        )
    }
    
    fn get_component_id(node: &RenderNode, bounds: &GeoRect) -> String {
        if let Some(id) = node.attrs.get("id") {
            if !id.is_empty() {
                return id.clone();
            }
        }
        format!("{}_{:.0}_{:.0}", node.tag, bounds.x, bounds.y)
    }

    fn draw_with_interaction(
        &mut self, 
        canvas: &mut Canvas, 
        taffy: &TaffyTree, 
        node: &RenderNode, 
        ox: f32, 
        oy: f32,
        interaction: &mut InteractionManager,
    ) {
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);
        
        let component_id = Self::get_component_id(node, &logical_bounds);
        
        // 应用交互状态
        let mut node_to_draw = node.clone();
        if let Some(state) = interaction.get_state(&component_id) {
            match node.tag.as_str() {
                "checkbox" | "switch" => {
                    node_to_draw.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    // 更新颜色
                    let checkbox_color = node.attrs.get("color")
                        .and_then(|c| super::components::parse_color_str(c))
                        .unwrap_or(Color::from_hex(0x09BB07));
                    if state.checked {
                        node_to_draw.style.background_color = Some(checkbox_color);
                        node_to_draw.style.border_color = Some(checkbox_color);
                    } else {
                        node_to_draw.style.background_color = Some(Color::WHITE);
                        node_to_draw.style.border_color = Some(Color::from_hex(0xD1D1D1));
                    }
                }
                "radio" => {
                    node_to_draw.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    // 更新颜色
                    let radio_color = node.attrs.get("color")
                        .and_then(|c| super::components::parse_color_str(c))
                        .unwrap_or(Color::from_hex(0x09BB07));
                    if state.checked {
                        node_to_draw.style.background_color = Some(radio_color);
                        node_to_draw.style.border_color = Some(radio_color);
                    } else {
                        node_to_draw.style.background_color = Some(Color::WHITE);
                        node_to_draw.style.border_color = Some(Color::from_hex(0xD1D1D1));
                    }
                }
                "slider" => {
                    if let Ok(v) = state.value.parse::<f32>() {
                        node_to_draw.style.custom_data = v / 100.0;
                        if !node_to_draw.text.is_empty() {
                            node_to_draw.text = format!("{}", v as i32);
                        }
                    }
                }
                "input" | "textarea" => {
                    // 获取 placeholder
                    let placeholder = node.attrs.get("placeholder").cloned().unwrap_or_default();
                    
                    // 检查是否聚焦
                    let is_focused = interaction.focused_input.as_ref()
                        .map(|f| f.id == component_id)
                        .unwrap_or(false);
                    
                    if state.value.is_empty() && !is_focused {
                        // 没有输入值且未聚焦时显示 placeholder
                        node_to_draw.text = placeholder;
                        node_to_draw.style.text_color = Some(Color::from_hex(0xBFBFBF));
                    } else {
                        // 有输入值或聚焦时显示实际值（聚焦时即使为空也不显示 placeholder）
                        node_to_draw.text = state.value.clone();
                        node_to_draw.style.text_color = Some(Color::BLACK);
                    }
                }
                _ => {}
            }
        }
        
        // 绘制组件 - 特殊处理 input 和 button 组件
        match node.tag.as_str() {
            "input" | "textarea" => {
                let focused = interaction.focused_input.as_ref()
                    .map(|f| f.id == component_id)
                    .unwrap_or(false);
                let (cursor_pos, selection) = if focused {
                    let f = interaction.focused_input.as_ref().unwrap();
                    (f.cursor_pos, f.get_selection_range())
                } else {
                    (0, None)
                };
                InputComponent::draw_with_selection(
                    &node_to_draw, canvas, self.text_renderer.as_ref(), 
                    x, y, w, h, sf, focused, cursor_pos, selection
                );
            }
            "button" => {
                let pressed = interaction.is_button_pressed(&component_id);
                ButtonComponent::draw_with_state(
                    &node_to_draw, canvas, self.text_renderer.as_ref(),
                    x, y, w, h, sf, pressed
                );
            }
            _ => {
                self.draw_component(canvas, &node_to_draw, x, y, w, h, sf);
            }
        }
        
        // 绘制子节点
        if !Self::is_leaf_component(&node.tag) {
            let text_color = node.style.text_color.unwrap_or(Color::BLACK);
            for child in &node.children { 
                self.draw_child_with_interaction(canvas, taffy, child, x, y, text_color, interaction); 
            }
        }

        // 记录事件绑定
        for (et, h, d) in &node.events {
            self.event_bindings.push(EventBinding { 
                event_type: et.clone(), 
                handler: h.clone(), 
                data: d.clone(), 
                bounds: logical_bounds 
            });
        }
        
        // 注册交互元素
        self.register_interactive_element(node, &node_to_draw, &logical_bounds, interaction);
    }
    
    fn draw_child_with_interaction(
        &mut self, 
        canvas: &mut Canvas, 
        taffy: &TaffyTree, 
        node: &RenderNode, 
        ox: f32, 
        oy: f32, 
        inherited_color: Color,
        interaction: &mut InteractionManager,
    ) {
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);

        let text_color = node.style.text_color.unwrap_or(inherited_color);
        let component_id = Self::get_component_id(node, &logical_bounds);
        
        let mut node_to_draw = node.clone();
        if node_to_draw.style.text_color.is_none() {
            node_to_draw.style.text_color = Some(text_color);
        }
        
        // 应用交互状态
        if let Some(state) = interaction.get_state(&component_id) {
            match node.tag.as_str() {
                "checkbox" | "switch" => {
                    node_to_draw.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    let checkbox_color = node.attrs.get("color")
                        .and_then(|c| super::components::parse_color_str(c))
                        .unwrap_or(Color::from_hex(0x09BB07));
                    if state.checked {
                        node_to_draw.style.background_color = Some(checkbox_color);
                        node_to_draw.style.border_color = Some(checkbox_color);
                    } else {
                        node_to_draw.style.background_color = Some(Color::WHITE);
                        node_to_draw.style.border_color = Some(Color::from_hex(0xD1D1D1));
                    }
                }
                "radio" => {
                    node_to_draw.style.custom_data = if state.checked { 1.0 } else { 0.0 };
                    let radio_color = node.attrs.get("color")
                        .and_then(|c| super::components::parse_color_str(c))
                        .unwrap_or(Color::from_hex(0x09BB07));
                    if state.checked {
                        node_to_draw.style.background_color = Some(radio_color);
                        node_to_draw.style.border_color = Some(radio_color);
                    } else {
                        node_to_draw.style.background_color = Some(Color::WHITE);
                        node_to_draw.style.border_color = Some(Color::from_hex(0xD1D1D1));
                    }
                }
                "slider" => {
                    if let Ok(v) = state.value.parse::<f32>() {
                        node_to_draw.style.custom_data = v / 100.0;
                        if !node_to_draw.text.is_empty() {
                            node_to_draw.text = format!("{}", v as i32);
                        }
                    }
                }
                "input" | "textarea" => {
                    let placeholder = node.attrs.get("placeholder").cloned().unwrap_or_default();
                    
                    // 检查是否聚焦
                    let is_focused = interaction.focused_input.as_ref()
                        .map(|f| f.id == component_id)
                        .unwrap_or(false);
                    
                    if state.value.is_empty() && !is_focused {
                        // 没有输入值且未聚焦时显示 placeholder
                        node_to_draw.text = placeholder;
                        node_to_draw.style.text_color = Some(Color::from_hex(0xBFBFBF));
                    } else {
                        // 有输入值或聚焦时显示实际值
                        node_to_draw.text = state.value.clone();
                        node_to_draw.style.text_color = Some(Color::BLACK);
                    }
                }
                _ => {}
            }
        }

        // 绘制组件 - 特殊处理 input 和 button 组件
        match node.tag.as_str() {
            "input" | "textarea" => {
                let focused = interaction.focused_input.as_ref()
                    .map(|f| f.id == component_id)
                    .unwrap_or(false);
                let (cursor_pos, selection) = if focused {
                    let f = interaction.focused_input.as_ref().unwrap();
                    (f.cursor_pos, f.get_selection_range())
                } else {
                    (0, None)
                };
                InputComponent::draw_with_selection(
                    &node_to_draw, canvas, self.text_renderer.as_ref(), 
                    x, y, w, h, sf, focused, cursor_pos, selection
                );
            }
            "button" => {
                let pressed = interaction.is_button_pressed(&component_id);
                ButtonComponent::draw_with_state(
                    &node_to_draw, canvas, self.text_renderer.as_ref(),
                    x, y, w, h, sf, pressed
                );
            }
            _ => {
                self.draw_component(canvas, &node_to_draw, x, y, w, h, sf);
            }
        }
        
        if !Self::is_leaf_component(&node.tag) {
            for child in &node.children { 
                self.draw_child_with_interaction(canvas, taffy, child, x, y, text_color, interaction); 
            }
        }

        for (et, h, d) in &node.events {
            self.event_bindings.push(EventBinding { 
                event_type: et.clone(), 
                handler: h.clone(), 
                data: d.clone(), 
                bounds: logical_bounds 
            });
        }
        
        self.register_interactive_element(node, &node_to_draw, &logical_bounds, interaction);
    }
    
    fn draw_component(&self, canvas: &mut Canvas, node: &RenderNode, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        match node.tag.as_str() {
            "#text" | "text" => TextComponent::draw(node, canvas, self.text_renderer.as_ref(), x, y, w, h, sf),
            "button" => ButtonComponent::draw(node, canvas, self.text_renderer.as_ref(), x, y, w, h, sf),
            "icon" => IconComponent::draw(node, canvas, x, y, w, h, sf),
            "progress" => ProgressComponent::draw(node, canvas, self.text_renderer.as_ref(), x, y, w, h, sf),
            "switch" => SwitchComponent::draw(node, canvas, x, y, w, h, sf),
            "checkbox" => CheckboxComponent::draw(node, canvas, x, y, w, h, sf),
            "radio" => RadioComponent::draw(node, canvas, x, y, w, h, sf),
            "slider" => SliderComponent::draw(node, canvas, self.text_renderer.as_ref(), x, y, w, h, sf),
            "input" | "textarea" => InputComponent::draw(node, canvas, self.text_renderer.as_ref(), x, y, w, h, sf),
            "image" => ImageComponent::draw(node, canvas, self.text_renderer.as_ref(), x, y, w, h, sf),
            _ => ViewComponent::draw(node, canvas, x, y, w, h, sf),
        }
    }
    
    fn register_interactive_element(
        &self, 
        original_node: &RenderNode, 
        drawn_node: &RenderNode,
        bounds: &GeoRect, 
        interaction: &mut InteractionManager
    ) {
        let disabled = original_node.attrs.get("disabled")
            .map(|s| s == "true" || s == "{{true}}")
            .unwrap_or(false);
        
        let id = Self::get_component_id(original_node, bounds);
        
        match original_node.tag.as_str() {
            "checkbox" => {
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Checkbox,
                    id,
                    bounds: *bounds,
                    checked: drawn_node.style.custom_data > 0.5,
                    value: original_node.attrs.get("value").cloned().unwrap_or_default(),
                    disabled,
                    min: 0.0,
                    max: 1.0,
                });
            }
            "radio" => {
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Radio,
                    id,
                    bounds: *bounds,
                    checked: drawn_node.style.custom_data > 0.5,
                    value: original_node.attrs.get("value").cloned().unwrap_or_default(),
                    disabled,
                    min: 0.0,
                    max: 1.0,
                });
            }
            "switch" => {
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Switch,
                    id,
                    bounds: *bounds,
                    checked: drawn_node.style.custom_data > 0.5,
                    value: original_node.text.clone(),
                    disabled,
                    min: 0.0,
                    max: 1.0,
                });
            }
            "slider" => {
                let min = original_node.attrs.get("min").and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let max = original_node.attrs.get("max").and_then(|s| s.parse().ok()).unwrap_or(100.0);
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Slider,
                    id,
                    bounds: *bounds,
                    checked: false,
                    value: format!("{}", (drawn_node.style.custom_data * 100.0) as i32),
                    disabled,
                    min,
                    max,
                });
            }
            "input" | "textarea" => {
                // 只使用原始 value 属性，不使用 placeholder
                let actual_value = original_node.attrs.get("value").cloned().unwrap_or_default();
                // 如果已有状态，使用状态中的值
                let current_value = interaction.get_state(&id)
                    .map(|s| s.value.clone())
                    .unwrap_or(actual_value);
                
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Input,
                    id,
                    bounds: *bounds,
                    checked: false,
                    value: current_value,
                    disabled,
                    min: 0.0,
                    max: 0.0,
                });
            }
            "button" => {
                interaction.register_element(InteractiveElement {
                    interaction_type: InteractionType::Button,
                    id,
                    bounds: *bounds,
                    checked: false,
                    value: original_node.text.clone(),
                    disabled,
                    min: 0.0,
                    max: 0.0,
                });
            }
            _ => {}
        }
    }

    
    fn draw(&mut self, canvas: &mut Canvas, taffy: &TaffyTree, node: &RenderNode, ox: f32, oy: f32) {
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);

        self.draw_component(canvas, node, x, y, w, h, sf);
        
        if !Self::is_leaf_component(&node.tag) {
            let text_color = node.style.text_color.unwrap_or(Color::BLACK);
            for child in &node.children { 
                self.draw_with_color(canvas, taffy, child, x, y, text_color); 
            }
        }

        for (et, h, d) in &node.events {
            self.event_bindings.push(EventBinding { 
                event_type: et.clone(), 
                handler: h.clone(), 
                data: d.clone(), 
                bounds: logical_bounds 
            });
        }
    }
    
    fn draw_with_color(&mut self, canvas: &mut Canvas, taffy: &TaffyTree, node: &RenderNode, ox: f32, oy: f32, inherited_color: Color) {
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);

        let text_color = node.style.text_color.unwrap_or(inherited_color);
        let mut node_with_color = node.clone();
        if node_with_color.style.text_color.is_none() {
            node_with_color.style.text_color = Some(text_color);
        }

        self.draw_component(canvas, &node_with_color, x, y, w, h, sf);
        
        if !Self::is_leaf_component(&node.tag) {
            for child in &node.children { 
                self.draw_with_color(canvas, taffy, child, x, y, text_color); 
            }
        }

        for (et, h, d) in &node.events {
            self.event_bindings.push(EventBinding { 
                event_type: et.clone(), 
                handler: h.clone(), 
                data: d.clone(), 
                bounds: logical_bounds 
            });
        }
    }

    fn measure_text(&self, text: &str, size: f32) -> f32 {
        self.text_renderer.as_ref()
            .map(|tr| tr.measure_text(text, size))
            .unwrap_or(text.chars().count() as f32 * size * 0.6)
    }

    pub fn get_event_bindings(&self) -> &[EventBinding] { 
        &self.event_bindings 
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<&EventBinding> {
        self.event_bindings.iter().rev().find(|b| b.bounds.contains(&crate::Point::new(x, y)))
    }
    
    /// 获取事件绑定数量
    pub fn event_count(&self) -> usize {
        self.event_bindings.len()
    }
    
    /// 打印所有事件绑定（调试用）
    pub fn debug_events(&self) {
        for (i, binding) in self.event_bindings.iter().enumerate() {
            println!("   [{}] {} -> {} bounds=({:.1},{:.1},{:.1},{:.1}) data={:?}", 
                i, binding.event_type, binding.handler,
                binding.bounds.x, binding.bounds.y, binding.bounds.width, binding.bounds.height,
                binding.data);
        }
    }
}
