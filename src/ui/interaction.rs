//! 组件交互管理器
//! 处理所有组件的交互状态和事件

use crate::Rect;
use std::collections::HashMap;

/// 组件交互状态
#[derive(Clone, Debug, Default)]
pub struct ComponentState {
    pub checked: bool,
    pub value: String,
}

/// 聚焦的输入框
#[derive(Clone, Debug)]
pub struct FocusedInput {
    pub id: String,
    pub value: String,
    pub cursor_pos: usize,
    pub is_password: bool,
}

/// 拖动中的滑块
#[derive(Clone, Debug)]
pub struct DraggingSlider {
    pub id: String,
    pub bounds: Rect,
    pub min: f32,
    pub max: f32,
}

/// 交互组件类型
#[derive(Debug, Clone, PartialEq)]
pub enum InteractionType {
    Checkbox,
    Radio,
    Switch,
    Slider,
    Input,
}

/// 可交互组件信息
#[derive(Debug, Clone)]
pub struct InteractiveElement {
    pub interaction_type: InteractionType,
    pub id: String,
    pub bounds: Rect,
    pub checked: bool,
    pub value: String,
    pub disabled: bool,
    pub min: f32,
    pub max: f32,
}

/// 交互管理器
pub struct InteractionManager {
    /// 组件状态
    pub states: HashMap<String, ComponentState>,
    /// 聚焦的输入框
    pub focused_input: Option<FocusedInput>,
    /// 拖动中的滑块
    pub dragging_slider: Option<DraggingSlider>,
    /// 当前页面的交互元素
    elements: Vec<InteractiveElement>,
}

impl InteractionManager {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            focused_input: None,
            dragging_slider: None,
            elements: Vec::new(),
        }
    }
    
    /// 清除交互元素列表（每次渲染前调用）
    pub fn clear_elements(&mut self) {
        self.elements.clear();
    }
    
    /// 注册交互元素
    pub fn register_element(&mut self, element: InteractiveElement) {
        self.elements.push(element);
    }
    
    /// 获取组件状态
    pub fn get_state(&self, id: &str) -> Option<&ComponentState> {
        self.states.get(id)
    }
    
    /// 设置组件状态
    pub fn set_state(&mut self, id: String, state: ComponentState) {
        self.states.insert(id, state);
    }
    
    /// 点击测试 - 返回点击到的交互元素
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&InteractiveElement> {
        // 调试：打印所有交互元素
        if self.elements.is_empty() {
            println!("⚠️ No interactive elements registered!");
        }
        
        for e in &self.elements {
            let hit = !e.disabled && 
                x >= e.bounds.x && x <= e.bounds.x + e.bounds.width &&
                y >= e.bounds.y && y <= e.bounds.y + e.bounds.height;
            if hit {
                println!("✅ Hit {:?} {} at ({:.1}, {:.1}, {:.1}, {:.1})", 
                    e.interaction_type, e.id, e.bounds.x, e.bounds.y, e.bounds.width, e.bounds.height);
            }
        }
        
        self.elements.iter().rev().find(|e| {
            !e.disabled && 
            x >= e.bounds.x && x <= e.bounds.x + e.bounds.width &&
            y >= e.bounds.y && y <= e.bounds.y + e.bounds.height
        })
    }
    
    /// 处理点击事件
    pub fn handle_click(&mut self, x: f32, y: f32) -> Option<InteractionResult> {
        let element = self.hit_test(x, y)?.clone();
        
        match element.interaction_type {
            InteractionType::Checkbox | InteractionType::Switch => {
                let current = self.states.get(&element.id)
                    .map(|s| s.checked)
                    .unwrap_or(element.checked);
                let new_checked = !current;
                
                self.states.insert(element.id.clone(), ComponentState {
                    checked: new_checked,
                    value: element.value.clone(),
                });
                
                Some(InteractionResult::Toggle {
                    id: element.id,
                    checked: new_checked,
                })
            }
            InteractionType::Radio => {
                self.states.insert(element.id.clone(), ComponentState {
                    checked: true,
                    value: element.value.clone(),
                });
                
                Some(InteractionResult::Select {
                    id: element.id,
                    value: element.value,
                })
            }
            InteractionType::Slider => {
                let progress = ((x - element.bounds.x) / element.bounds.width).clamp(0.0, 1.0);
                let value = element.min + progress * (element.max - element.min);
                
                self.states.insert(element.id.clone(), ComponentState {
                    checked: false,
                    value: format!("{}", value as i32),
                });
                
                self.dragging_slider = Some(DraggingSlider {
                    id: element.id.clone(),
                    bounds: element.bounds,
                    min: element.min,
                    max: element.max,
                });
                
                Some(InteractionResult::SliderChange {
                    id: element.id,
                    value: value as i32,
                })
            }
            InteractionType::Input => {
                let current_value = self.states.get(&element.id)
                    .map(|s| s.value.clone())
                    .unwrap_or(element.value.clone());
                
                self.focused_input = Some(FocusedInput {
                    id: element.id.clone(),
                    value: current_value.clone(),
                    cursor_pos: current_value.chars().count(),
                    is_password: false,
                });
                
                Some(InteractionResult::Focus { id: element.id })
            }
        }
    }
    
    /// 处理鼠标移动（用于滑块拖动）
    pub fn handle_mouse_move(&mut self, x: f32, _y: f32) -> Option<InteractionResult> {
        if let Some(ref slider) = self.dragging_slider {
            let progress = ((x - slider.bounds.x) / slider.bounds.width).clamp(0.0, 1.0);
            let value = slider.min + progress * (slider.max - slider.min);
            
            self.states.insert(slider.id.clone(), ComponentState {
                checked: false,
                value: format!("{}", value as i32),
            });
            
            return Some(InteractionResult::SliderChange {
                id: slider.id.clone(),
                value: value as i32,
            });
        }
        None
    }
    
    /// 处理鼠标释放
    pub fn handle_mouse_release(&mut self) -> Option<InteractionResult> {
        if let Some(slider) = self.dragging_slider.take() {
            return Some(InteractionResult::SliderEnd { id: slider.id });
        }
        None
    }
    
    /// 处理键盘输入
    pub fn handle_key_input(&mut self, key: KeyInput) -> Option<InteractionResult> {
        let input = self.focused_input.as_mut()?;
        
        match key {
            KeyInput::Char(c) => {
                let mut chars: Vec<char> = input.value.chars().collect();
                chars.insert(input.cursor_pos, c);
                input.value = chars.into_iter().collect();
                input.cursor_pos += 1;
                
                // 同步到状态
                self.states.insert(input.id.clone(), ComponentState {
                    checked: false,
                    value: input.value.clone(),
                });
                
                Some(InteractionResult::InputChange {
                    id: input.id.clone(),
                    value: input.value.clone(),
                })
            }
            KeyInput::Backspace => {
                if input.cursor_pos > 0 {
                    let mut chars: Vec<char> = input.value.chars().collect();
                    chars.remove(input.cursor_pos - 1);
                    input.value = chars.into_iter().collect();
                    input.cursor_pos -= 1;
                    
                    self.states.insert(input.id.clone(), ComponentState {
                        checked: false,
                        value: input.value.clone(),
                    });
                    
                    Some(InteractionResult::InputChange {
                        id: input.id.clone(),
                        value: input.value.clone(),
                    })
                } else {
                    None
                }
            }
            KeyInput::Delete => {
                let chars: Vec<char> = input.value.chars().collect();
                if input.cursor_pos < chars.len() {
                    let mut chars = chars;
                    chars.remove(input.cursor_pos);
                    input.value = chars.into_iter().collect();
                    
                    self.states.insert(input.id.clone(), ComponentState {
                        checked: false,
                        value: input.value.clone(),
                    });
                    
                    Some(InteractionResult::InputChange {
                        id: input.id.clone(),
                        value: input.value.clone(),
                    })
                } else {
                    None
                }
            }
            KeyInput::Left => {
                if input.cursor_pos > 0 {
                    input.cursor_pos -= 1;
                }
                None
            }
            KeyInput::Right => {
                if input.cursor_pos < input.value.chars().count() {
                    input.cursor_pos += 1;
                }
                None
            }
            KeyInput::Home => {
                input.cursor_pos = 0;
                None
            }
            KeyInput::End => {
                input.cursor_pos = input.value.chars().count();
                None
            }
            KeyInput::Enter | KeyInput::Escape => {
                let id = input.id.clone();
                let value = input.value.clone();
                self.focused_input = None;
                Some(InteractionResult::InputBlur { id, value })
            }
        }
    }
    
    /// 取消输入框聚焦
    pub fn blur_input(&mut self) -> Option<InteractionResult> {
        if let Some(input) = self.focused_input.take() {
            return Some(InteractionResult::InputBlur {
                id: input.id,
                value: input.value,
            });
        }
        None
    }
    
    /// 是否有输入框聚焦
    pub fn has_focused_input(&self) -> bool {
        self.focused_input.is_some()
    }
    
    /// 是否正在拖动滑块
    pub fn is_dragging_slider(&self) -> bool {
        self.dragging_slider.is_some()
    }
    
    /// 页面切换时清除状态
    pub fn clear_page_state(&mut self) {
        self.states.clear();
        self.focused_input = None;
        self.dragging_slider = None;
        self.elements.clear();
    }
}

/// 键盘输入类型
#[derive(Debug, Clone)]
pub enum KeyInput {
    Char(char),
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    Enter,
    Escape,
}

/// 交互结果
#[derive(Debug, Clone)]
pub enum InteractionResult {
    Toggle { id: String, checked: bool },
    Select { id: String, value: String },
    SliderChange { id: String, value: i32 },
    SliderEnd { id: String },
    Focus { id: String },
    InputChange { id: String, value: String },
    InputBlur { id: String, value: String },
}
