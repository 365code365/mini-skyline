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
    SliderComponent, InputComponent, ImageComponent, VideoComponent,
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
        self.render_with_scroll_and_viewport(canvas, nodes, data, interaction, 0.0, self.screen_height);
    }
    
    /// 渲染 WXML 节点，支持滚动偏移（用于 fixed 定位）
    pub fn render_with_scroll(
        &mut self, 
        canvas: &mut Canvas, 
        nodes: &[WxmlNode], 
        data: &JsonValue,
        interaction: &mut InteractionManager,
        scroll_offset: f32,
    ) {
        self.render_with_scroll_and_viewport(canvas, nodes, data, interaction, scroll_offset, self.screen_height);
    }
    
    /// 渲染 WXML 节点，支持滚动偏移和自定义视口高度（用于 fixed 定位）
    /// 返回实际内容高度
    pub fn render_with_scroll_and_viewport(
        &mut self, 
        canvas: &mut Canvas, 
        nodes: &[WxmlNode], 
        data: &JsonValue,
        interaction: &mut InteractionManager,
        _scroll_offset: f32,
        _viewport_height: f32,
    ) -> f32 {
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
        
        // 构建正常布局树（包含所有节点，fixed 元素也参与布局计算）
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
        
        // 获取实际内容高度
        let root_layout = taffy.layout(root).unwrap();
        let content_height = root_layout.size.height / self.scale_factor;
        
        // 渲染所有元素（fixed 元素会在 draw_with_interaction 中被跳过）
        for rn in &render_nodes {
            self.draw_with_interaction(canvas, &taffy, rn, 0.0, 0.0, interaction, 0.0);
        }
        
        // Fixed 元素不在这里渲染，而是通过 render_fixed_elements 单独渲染
        
        content_height
    }
    
    /// 单独渲染 fixed 元素到指定的 canvas
    /// 这个方法应该在主内容渲染后调用，fixed_canvas 是一个覆盖在主内容上的透明层
    pub fn render_fixed_elements(
        &mut self,
        canvas: &mut Canvas,
        nodes: &[WxmlNode],
        data: &JsonValue,
        interaction: &mut InteractionManager,
        viewport_height: f32,
    ) {
        let rendered = crate::parser::TemplateEngine::render(nodes, data);
        let mut taffy = TaffyTree::new();
        
        let mut render_nodes = Vec::new();
        for node in &rendered {
            if let Some(rn) = self.build_tree(&mut taffy, node) {
                render_nodes.push(rn);
            }
        }
        
        // 收集 fixed 元素
        struct FixedNodeInfo {
            node: RenderNode,
        }
        
        fn collect_fixed(nodes: &[RenderNode], fixed_list: &mut Vec<FixedNodeInfo>) {
            for node in nodes {
                if node.style.is_fixed {
                    fixed_list.push(FixedNodeInfo { node: node.clone() });
                }
                collect_fixed(&node.children, fixed_list);
            }
        }
        
        // 构建布局树
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
        
        let mut fixed_nodes = Vec::new();
        collect_fixed(&render_nodes, &mut fixed_nodes);
        
        if fixed_nodes.is_empty() {
            return;
        }
        
        let sf = self.scale_factor;
        let vp_width = self.screen_width * sf;
        let vp_height = viewport_height * sf;
        
        for info in &fixed_nodes {
            let rn = &info.node;
            let layout = taffy.layout(rn.taffy_node).unwrap();
            let w = layout.size.width;
            let h = layout.size.height;
            
            // 计算 fixed 元素的位置（相对于视口）
            let actual_w = if rn.style.fixed_left.is_some() && rn.style.fixed_right.is_some() {
                let left = rn.style.fixed_left.unwrap_or(0.0);
                let right = rn.style.fixed_right.unwrap_or(0.0);
                vp_width - left - right
            } else {
                w
            };
            
            let x = rn.style.fixed_left.unwrap_or(0.0);
            let y = if let Some(bottom) = rn.style.fixed_bottom {
                // bottom 定位：从视口底部计算
                vp_height - bottom - h
            } else if let Some(top) = rn.style.fixed_top {
                // top 定位：从视口顶部计算
                top
            } else {
                0.0
            };
            
            // 渲染 fixed 元素
            self.draw_fixed_element_original(&taffy, canvas, rn, x, y, actual_w, h, interaction);
        }
    }
    
    /// 使用原始 taffy 布局绘制 fixed 元素
    fn draw_fixed_element_original(
        &mut self,
        taffy: &TaffyTree,
        canvas: &mut Canvas,
        node: &RenderNode,
        fixed_x: f32,
        fixed_y: f32,
        fixed_w: f32,
        fixed_h: f32,
        interaction: &mut InteractionManager,
    ) {
        let sf = self.scale_factor;
        let logical_bounds = GeoRect::new(fixed_x / sf, fixed_y / sf, fixed_w / sf, fixed_h / sf);
        
        // 绘制 fixed 元素的背景
        self.draw_component(canvas, node, fixed_x, fixed_y, fixed_w, fixed_h, sf);
        
        // 绘制子节点 - 子节点位置相对于 fixed 元素
        if !Self::is_leaf_component(&node.tag) {
            let text_color = node.style.text_color.unwrap_or(Color::BLACK);
            for child in &node.children {
                // 获取子节点在原始布局中相对于父节点的位置
                let child_layout = taffy.layout(child.taffy_node).unwrap();
                let child_x = fixed_x + child_layout.location.x;
                let child_y = fixed_y + child_layout.location.y;
                let child_w = child_layout.size.width;
                let child_h = child_layout.size.height;
                
                self.draw_fixed_child_recursive(taffy, canvas, child, child_x, child_y, child_w, child_h, text_color, interaction);
            }
        }
        
        // 记录事件绑定
        for (et, handler, data) in &node.events {
            self.event_bindings.push(EventBinding {
                event_type: et.clone(),
                handler: handler.clone(),
                data: data.clone(),
                bounds: logical_bounds,
            });
        }
        
        // 注册交互元素
        self.register_interactive_element(node, node, &logical_bounds, interaction);
    }
    
    /// 递归绘制 fixed 元素的子节点
    fn draw_fixed_child_recursive(
        &mut self,
        taffy: &TaffyTree,
        canvas: &mut Canvas,
        node: &RenderNode,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        inherited_color: Color,
        interaction: &mut InteractionManager,
    ) {
        let sf = self.scale_factor;
        let logical_bounds = GeoRect::new(x / sf, y / sf, w / sf, h / sf);
        
        let text_color = node.style.text_color.unwrap_or(inherited_color);
        let mut node_to_draw = node.clone();
        if node_to_draw.style.text_color.is_none() {
            node_to_draw.style.text_color = Some(text_color);
        }
        
        // 绘制组件
        self.draw_component(canvas, &node_to_draw, x, y, w, h, sf);
        
        // 递归绘制子节点
        if !Self::is_leaf_component(&node.tag) {
            for child in &node.children {
                let child_layout = taffy.layout(child.taffy_node).unwrap();
                let child_x = x + child_layout.location.x;
                let child_y = y + child_layout.location.y;
                let child_w = child_layout.size.width;
                let child_h = child_layout.size.height;
                
                self.draw_fixed_child_recursive(taffy, canvas, child, child_x, child_y, child_w, child_h, text_color, interaction);
            }
        }
        
        // 记录事件绑定
        for (et, handler, data) in &node.events {
            self.event_bindings.push(EventBinding {
                event_type: et.clone(),
                handler: handler.clone(),
                data: data.clone(),
                bounds: logical_bounds,
            });
        }
        
        // 注册交互元素
        self.register_interactive_element(node, &node_to_draw, &logical_bounds, interaction);
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
            "video" => VideoComponent::build(node, &mut ctx),
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
                    
                    // Debug: 打印 list-item 和 list-info 的布局信息
                    let classes: Vec<&str> = node.get_attr("class")
                        .map(|s| s.split_whitespace().collect())
                        .unwrap_or_default();
                    if classes.contains(&"list-item") {
                        eprintln!("[BUILD] list-item: flex_direction={:?} align_items={:?} children={}", 
                            ts.flex_direction, ts.align_items, child_ids.len());
                        eprintln!("  rn.attrs.class = {:?}", rn.attrs.get("class"));
                    }
                    if classes.contains(&"list-info") {
                        eprintln!("[BUILD] list-info: flex_direction={:?} flex_grow={} children={}", 
                            ts.flex_direction, ts.flex_grow, child_ids.len());
                        eprintln!("  size={:?} min_size={:?}", ts.size, ts.min_size);
                        eprintln!("  align_items={:?} align_self={:?}", ts.align_items, ts.align_self);
                        eprintln!("  child_ids={:?}", child_ids);
                        for (i, child) in children.iter().enumerate() {
                            let cc = child.attrs.get("class").map(|s| s.as_str()).unwrap_or("");
                            // 获取子节点的 taffy 布局信息
                            let child_style = ctx.taffy.style(child.taffy_node).unwrap();
                            eprintln!("  child[{}] class={} tag={} taffy_node={:?}", 
                                i, cc, child.tag, child.taffy_node);
                            eprintln!("    size={:?} min_size={:?}", child_style.size, child_style.min_size);
                            eprintln!("    margin={:?}", child_style.margin);
                        }
                    }
                    
                    let new_tn = ctx.taffy.new_with_children(ts, &child_ids).unwrap();
                    
                    // Debug: 验证子节点是否正确关联
                    if classes.contains(&"list-info") {
                        eprintln!("[BUILD] list-info: new_tn={:?}", new_tn);
                        let children_of_new_tn = ctx.taffy.children(new_tn).unwrap();
                        eprintln!("  children_of_new_tn={:?}", children_of_new_tn);
                    }
                    
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
            "checkbox" | "radio" | "slider" | "input" | "textarea" | "image" | "video"
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
        scroll_offset: f32,
    ) {
        // 跳过 fixed 元素，它们会单独渲染
        if node.style.is_fixed {
            return;
        }
        
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        
        // Debug: 打印顶层节点的子节点数量（只打印前几次）
        static DRAW_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let class_str = node.attrs.get("class").map(|s| s.as_str()).unwrap_or("");
        let count = DRAW_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count < 5 {
            eprintln!("[DRAW] tag={} class={} children={}", node.tag, class_str, node.children.len());
        }
        
        // Debug: 打印 list-item 的布局（只打印一次）
        static PRINTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if class_str.contains("list-item") && !PRINTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            eprintln!("[LAYOUT] list-item: x={:.0} y={:.0} w={:.0} h={:.0} children={}", x, y, w, h, node.children.len());
            for (i, child) in node.children.iter().enumerate() {
                let cl = taffy.layout(child.taffy_node).unwrap();
                let cc = child.attrs.get("class").map(|s| s.as_str()).unwrap_or("");
                eprintln!("  child[{}] {}: x={:.0} y={:.0} w={:.0} h={:.0}", i, cc, cl.location.x, cl.location.y, cl.size.width, cl.size.height);
            }
        }
        
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
        } else if matches!(node.tag.as_str(), "input" | "textarea") {
            // 输入框但还没有交互状态，显示 placeholder 或初始值
            let placeholder = node.attrs.get("placeholder").cloned().unwrap_or_default();
            let initial_value = node.attrs.get("value").cloned().unwrap_or_default();
            
            if initial_value.is_empty() {
                node_to_draw.text = placeholder;
                node_to_draw.style.text_color = Some(Color::from_hex(0xBFBFBF));
            } else {
                node_to_draw.text = initial_value;
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
                self.draw_child_with_interaction(canvas, taffy, child, x, y, text_color, interaction, scroll_offset); 
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
        scroll_offset: f32,
    ) {
        // 跳过 fixed 元素，它们会单独渲染
        if node.style.is_fixed {
            return;
        }
        
        let sf = self.scale_factor;
        let layout = taffy.layout(node.taffy_node).unwrap();
        let x = ox + layout.location.x;
        let y = oy + layout.location.y;
        let w = layout.size.width;
        let h = layout.size.height;
        
        // Debug: 打印 list-item 的布局
        let class_str = node.attrs.get("class").map(|s| s.as_str()).unwrap_or("");
        if class_str.contains("list-item") {
            eprintln!("[LAYOUT-CHILD] list-item: x={:.0} y={:.0} w={:.0} h={:.0} children={}", x, y, w, h, node.children.len());
            for (i, child) in node.children.iter().enumerate() {
                let cl = taffy.layout(child.taffy_node).unwrap();
                let cc = child.attrs.get("class").map(|s| s.as_str()).unwrap_or("");
                eprintln!("  child[{}] {}: x={:.0} y={:.0} w={:.0} h={:.0} tag={}", i, cc, cl.location.x, cl.location.y, cl.size.width, cl.size.height, child.tag);
                // 打印 list-info 的子节点
                if cc.contains("list-info") {
                    eprintln!("    list-info children: {}", child.children.len());
                    for (j, gc) in child.children.iter().enumerate() {
                        let gcl = taffy.layout(gc.taffy_node).unwrap();
                        let gcc = gc.attrs.get("class").map(|s| s.as_str()).unwrap_or("");
                        let text_preview: String = gc.text.chars().take(15).collect();
                        eprintln!("      gc[{}] {}: x={:.0} y={:.0} w={:.0} h={:.0} tag={} text={}", 
                            j, gcc, gcl.location.x, gcl.location.y, gcl.size.width, gcl.size.height, gc.tag, text_preview);
                    }
                }
            }
        }
        // Debug: 打印 product-list 的子节点
        static PRINTED3: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if class_str.contains("product-list") && !PRINTED3.swap(true, std::sync::atomic::Ordering::Relaxed) {
            eprintln!("[LAYOUT-CHILD] product-list: children={}", node.children.len());
            for (i, child) in node.children.iter().enumerate() {
                let cc = child.attrs.get("class").map(|s| s.as_str()).unwrap_or("");
                eprintln!("  child[{}] class={} tag={}", i, cc, child.tag);
            }
        }
        
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
        } else if matches!(node.tag.as_str(), "input" | "textarea") {
            // 输入框但还没有交互状态，显示 placeholder 或初始值
            let placeholder = node.attrs.get("placeholder").cloned().unwrap_or_default();
            let initial_value = node.attrs.get("value").cloned().unwrap_or_default();
            
            if initial_value.is_empty() {
                node_to_draw.text = placeholder;
                node_to_draw.style.text_color = Some(Color::from_hex(0xBFBFBF));
            } else {
                node_to_draw.text = initial_value;
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
                self.draw_child_with_interaction(canvas, taffy, child, x, y, text_color, interaction, scroll_offset); 
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
            "video" => VideoComponent::draw(node, canvas, self.text_renderer.as_ref(), x, y, w, h, sf),
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
