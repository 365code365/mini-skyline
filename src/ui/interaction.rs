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
    pub selection_start: Option<usize>, // 选择起始位置
    pub selection_end: Option<usize>,   // 选择结束位置
    pub is_password: bool,
    pub bounds: Rect, // 输入框位置
}

impl FocusedInput {
    /// 是否有选中文本
    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some() && self.selection_end.is_some()
    }
    
    /// 获取选中范围 (start, end)，保证 start <= end
    pub fn get_selection_range(&self) -> Option<(usize, usize)> {
        match (self.selection_start, self.selection_end) {
            (Some(s), Some(e)) if s != e => {
                Some((s.min(e), s.max(e)))
            }
            _ => None
        }
    }
    
    /// 全选
    pub fn select_all(&mut self) {
        self.selection_start = Some(0);
        self.selection_end = Some(self.value.chars().count());
        self.cursor_pos = self.value.chars().count();
    }
    
    /// 清除选择
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
    }
    
    /// 删除选中的文本，返回是否有删除
    pub fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.get_selection_range() {
            let chars: Vec<char> = self.value.chars().collect();
            let mut new_chars = Vec::new();
            for (i, c) in chars.into_iter().enumerate() {
                if i < start || i >= end {
                    new_chars.push(c);
                }
            }
            self.value = new_chars.into_iter().collect();
            self.cursor_pos = start;
            self.clear_selection();
            true
        } else {
            false
        }
    }
}

/// 计算光标位置
///
/// 根据 x 坐标（相对于输入框左边缘）和每个字符的宽度，计算光标应该在的位置
/// 返回光标位置（字符索引）
pub fn calculate_cursor_position(text: &str, char_widths: &[f32], click_x: f32, padding_left: f32) -> usize {
    // 减去左边距后的点击位置
    let click_offset = click_x - padding_left;

    if click_offset <= 0.0 {
        // 点击在文本左侧，光标在开头
        return 0;
    }

    let mut cumulative_width = 0.0;
    for (i, &width) in char_widths.iter().enumerate() {
        let next_width = cumulative_width + width;

        if click_offset < next_width - width / 2.0 {
            // 点击在字符的前半部分，光标在字符之前
            return i;
        } else if click_offset >= next_width - width / 2.0 && click_offset < next_width {
            // 点击在字符的后半部分，光标在字符之后
            return i + 1;
        }

        cumulative_width = next_width;
    }

    // 点击在所有字符之后，光标在末尾
    text.chars().count()
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
    Button,
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

/// 按下的按钮
#[derive(Clone, Debug)]
pub struct PressedButton {
    pub id: String,
    pub bounds: Rect,
}

/// 点击动画状态
#[derive(Clone, Debug)]
pub struct ClickAnimation {
    pub id: String,
    pub start_time: std::time::Instant,
    pub duration_ms: u64,
}

/// 交互管理器
pub struct InteractionManager {
    /// 组件状态
    pub states: HashMap<String, ComponentState>,
    /// 聚焦的输入框
    pub focused_input: Option<FocusedInput>,
    /// 拖动中的滑块
    pub dragging_slider: Option<DraggingSlider>,
    /// 按下的按钮
    pub pressed_button: Option<PressedButton>,
    /// 点击动画
    pub click_animations: Vec<ClickAnimation>,
    /// 当前页面的交互元素
    elements: Vec<InteractiveElement>,
}

impl InteractionManager {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            focused_input: None,
            dragging_slider: None,
            pressed_button: None,
            click_animations: Vec::new(),
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
                // Radio 是互斥的，需要取消同一组内其他 radio 的选中状态
                // 简单实现：取消所有其他 radio 的选中状态（假设页面上只有一组 radio）
                // 更好的实现应该基于 radio-group 或父容器
                let radio_ids: Vec<String> = self.elements.iter()
                    .filter(|e| e.interaction_type == InteractionType::Radio && e.id != element.id)
                    .map(|e| e.id.clone())
                    .collect();
                
                for id in radio_ids {
                    self.states.insert(id, ComponentState {
                        checked: false,
                        value: String::new(),
                    });
                }
                
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
                // 获取当前值（如果没有状态，使用空字符串而不是 element.value）
                let current_value = self.states.get(&element.id)
                    .map(|s| s.value.clone())
                    .unwrap_or_else(|| {
                        // 如果 element.value 是空的，说明没有初始值
                        if element.value.is_empty() {
                            String::new()
                        } else {
                            element.value.clone()
                        }
                    });
                
                // 初始化状态（如果还没有）
                if !self.states.contains_key(&element.id) {
                    self.states.insert(element.id.clone(), ComponentState {
                        checked: false,
                        value: current_value.clone(),
                    });
                }
                
                self.focused_input = Some(FocusedInput {
                    id: element.id.clone(),
                    value: current_value.clone(),
                    cursor_pos: current_value.chars().count(),
                    selection_start: None,
                    selection_end: None,
                    is_password: false,
                    bounds: element.bounds,
                });

                // 记录点击位置用于后续计算光标位置
                let click_x = x - element.bounds.x;

                Some(InteractionResult::Focus {
                    id: element.id,
                    bounds: element.bounds,
                    click_x,
                })
            }
            InteractionType::Button => {
                // 按钮点击不需要在这里触发动画，按下状态由鼠标按下/松开控制
                Some(InteractionResult::ButtonClick {
                    id: element.id,
                    bounds: element.bounds,
                })
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
                // 如果有选中文本，先删除
                input.delete_selection();
                
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
                // 如果有选中文本，删除选中部分
                if input.delete_selection() {
                    self.states.insert(input.id.clone(), ComponentState {
                        checked: false,
                        value: input.value.clone(),
                    });
                    return Some(InteractionResult::InputChange {
                        id: input.id.clone(),
                        value: input.value.clone(),
                    });
                }
                
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
                // 如果有选中文本，删除选中部分
                if input.delete_selection() {
                    self.states.insert(input.id.clone(), ComponentState {
                        checked: false,
                        value: input.value.clone(),
                    });
                    return Some(InteractionResult::InputChange {
                        id: input.id.clone(),
                        value: input.value.clone(),
                    });
                }
                
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
                input.clear_selection();
                if input.cursor_pos > 0 {
                    input.cursor_pos -= 1;
                }
                None
            }
            KeyInput::Right => {
                input.clear_selection();
                if input.cursor_pos < input.value.chars().count() {
                    input.cursor_pos += 1;
                }
                None
            }
            KeyInput::Home => {
                input.clear_selection();
                input.cursor_pos = 0;
                None
            }
            KeyInput::End => {
                input.clear_selection();
                input.cursor_pos = input.value.chars().count();
                None
            }
            KeyInput::SelectAll => {
                input.select_all();
                None
            }
            KeyInput::Copy => {
                // 返回选中的文本用于复制
                if let Some((start, end)) = input.get_selection_range() {
                    let selected: String = input.value.chars().skip(start).take(end - start).collect();
                    return Some(InteractionResult::CopyText { text: selected });
                }
                None
            }
            KeyInput::Cut => {
                // 剪切：复制并删除
                if let Some((start, end)) = input.get_selection_range() {
                    let selected: String = input.value.chars().skip(start).take(end - start).collect();
                    input.delete_selection();
                    
                    self.states.insert(input.id.clone(), ComponentState {
                        checked: false,
                        value: input.value.clone(),
                    });
                    
                    return Some(InteractionResult::CutText { 
                        text: selected,
                        id: input.id.clone(),
                        value: input.value.clone(),
                    });
                }
                None
            }
            KeyInput::Paste(text) => {
                // 如果有选中文本，先删除
                input.delete_selection();
                
                // 插入粘贴的文本
                let mut chars: Vec<char> = input.value.chars().collect();
                for (i, c) in text.chars().enumerate() {
                    chars.insert(input.cursor_pos + i, c);
                }
                input.value = chars.into_iter().collect();
                input.cursor_pos += text.chars().count();
                
                self.states.insert(input.id.clone(), ComponentState {
                    checked: false,
                    value: input.value.clone(),
                });
                
                Some(InteractionResult::InputChange {
                    id: input.id.clone(),
                    value: input.value.clone(),
                })
            }
            KeyInput::ShiftLeft => {
                // 扩展选择向左
                if input.selection_start.is_none() {
                    input.selection_start = Some(input.cursor_pos);
                    input.selection_end = Some(input.cursor_pos);
                }
                if input.cursor_pos > 0 {
                    input.cursor_pos -= 1;
                    input.selection_end = Some(input.cursor_pos);
                }
                None
            }
            KeyInput::ShiftRight => {
                // 扩展选择向右
                if input.selection_start.is_none() {
                    input.selection_start = Some(input.cursor_pos);
                    input.selection_end = Some(input.cursor_pos);
                }
                if input.cursor_pos < input.value.chars().count() {
                    input.cursor_pos += 1;
                    input.selection_end = Some(input.cursor_pos);
                }
                None
            }
            KeyInput::ShiftHome => {
                // 选择到开头
                if input.selection_start.is_none() {
                    input.selection_start = Some(input.cursor_pos);
                }
                input.cursor_pos = 0;
                input.selection_end = Some(0);
                None
            }
            KeyInput::ShiftEnd => {
                // 选择到结尾
                if input.selection_start.is_none() {
                    input.selection_start = Some(input.cursor_pos);
                }
                let len = input.value.chars().count();
                input.cursor_pos = len;
                input.selection_end = Some(len);
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
        self.pressed_button = None;
        self.click_animations.clear();
        self.elements.clear();
    }
    
    /// 设置按钮按下状态
    pub fn set_button_pressed(&mut self, id: String, bounds: Rect) {
        self.pressed_button = Some(PressedButton { id, bounds });
    }
    
    /// 清除按钮按下状态
    pub fn clear_button_pressed(&mut self) {
        self.pressed_button = None;
    }
    
    /// 检查按钮是否被按下
    pub fn is_button_pressed(&self, id: &str) -> bool {
        self.pressed_button.as_ref().map(|b| b.id == id).unwrap_or(false)
    }
    
    /// 触发点击动画
    pub fn trigger_click_animation(&mut self, id: String) {
        // 移除该 id 的旧动画
        self.click_animations.retain(|a| a.id != id);
        // 添加新动画
        self.click_animations.push(ClickAnimation {
            id,
            start_time: std::time::Instant::now(),
            duration_ms: 150, // 150ms 动画
        });
    }
    
    /// 更新动画状态，返回是否还有动画在进行
    pub fn update_animations(&mut self) -> bool {
        let now = std::time::Instant::now();
        self.click_animations.retain(|a| {
            now.duration_since(a.start_time).as_millis() < a.duration_ms as u128
        });
        !self.click_animations.is_empty()
    }
    
    /// 检查按钮是否在点击动画中
    pub fn is_in_click_animation(&self, id: &str) -> bool {
        let now = std::time::Instant::now();
        self.click_animations.iter().any(|a| {
            a.id == id && now.duration_since(a.start_time).as_millis() < a.duration_ms as u128
        })
    }
    
    /// 是否有动画在进行
    pub fn has_animations(&self) -> bool {
        !self.click_animations.is_empty()
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
    SelectAll,      // Ctrl+A
    Copy,           // Ctrl+C
    Cut,            // Ctrl+X
    Paste(String),  // Ctrl+V
    ShiftLeft,      // Shift+Left (扩展选择)
    ShiftRight,     // Shift+Right
    ShiftHome,      // Shift+Home
    ShiftEnd,       // Shift+End
}

/// 交互结果
#[derive(Debug, Clone)]
pub enum InteractionResult {
    Toggle { id: String, checked: bool },
    Select { id: String, value: String },
    SliderChange { id: String, value: i32 },
    SliderEnd { id: String },
    Focus { id: String, bounds: Rect, click_x: f32 },
    InputChange { id: String, value: String },
    InputBlur { id: String, value: String },
    ButtonClick { id: String, bounds: Rect },
    CopyText { text: String },
    CutText { text: String, id: String, value: String },
}
