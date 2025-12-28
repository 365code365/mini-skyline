//! 布局系统 - Flexbox 风格布局

use crate::{Canvas, Color, Paint, PaintStyle};
use super::component::{Component, ComponentId, Style};

/// Flex 方向
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlexDirection {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

/// Flex 对齐
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlexAlign {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Layout - Flex 布局容器
pub struct Layout {
    id: ComponentId,
    style: Style,
    children: Vec<Box<dyn Component>>,
    direction: FlexDirection,
    justify_content: FlexAlign,
    align_items: FlexAlign,
    gap: f32,
}

impl Layout {
    pub fn new() -> Self {
        Self {
            id: ComponentId::new(),
            style: Style::default(),
            children: Vec::new(),
            direction: FlexDirection::Row,
            justify_content: FlexAlign::Start,
            align_items: FlexAlign::Start,
            gap: 0.0,
        }
    }
    
    pub fn row() -> Self {
        Self::new().with_direction(FlexDirection::Row)
    }
    
    pub fn column() -> Self {
        Self::new().with_direction(FlexDirection::Column)
    }
    
    pub fn with_frame(mut self, x: f32, y: f32, width: f32, height: f32) -> Self {
        self.style.x = x;
        self.style.y = y;
        self.style.width = width;
        self.style.height = height;
        self
    }
    
    pub fn with_direction(mut self, direction: FlexDirection) -> Self {
        self.direction = direction;
        self
    }
    
    pub fn with_justify_content(mut self, align: FlexAlign) -> Self {
        self.justify_content = align;
        self
    }
    
    pub fn with_align_items(mut self, align: FlexAlign) -> Self {
        self.align_items = align;
        self
    }
    
    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
    
    pub fn with_background(mut self, color: Color) -> Self {
        self.style.background_color = Some(color);
        self
    }
    
    pub fn with_padding(mut self, padding: f32) -> Self {
        self.style.padding = [padding; 4];
        self
    }
    
    /// 计算并应用布局
    pub fn layout(&mut self) {
        if self.children.is_empty() {
            return;
        }
        
        let bounds = self.style.content_bounds();
        let is_row = matches!(self.direction, FlexDirection::Row | FlexDirection::RowReverse);
        let is_reverse = matches!(self.direction, FlexDirection::RowReverse | FlexDirection::ColumnReverse);
        
        // 计算子元素总尺寸
        let mut total_main = 0.0;
        for child in &self.children {
            let s = child.style();
            total_main += if is_row { s.width } else { s.height };
        }
        total_main += self.gap * (self.children.len() - 1) as f32;
        
        let main_size = if is_row { bounds.width } else { bounds.height };
        let cross_size = if is_row { bounds.height } else { bounds.width };
        
        // 计算起始位置和间距
        let (mut main_pos, main_gap) = match self.justify_content {
            FlexAlign::Start => (0.0, self.gap),
            FlexAlign::End => (main_size - total_main, self.gap),
            FlexAlign::Center => ((main_size - total_main) / 2.0, self.gap),
            FlexAlign::SpaceBetween => {
                let gap = if self.children.len() > 1 {
                    (main_size - total_main + self.gap * (self.children.len() - 1) as f32) / (self.children.len() - 1) as f32
                } else {
                    0.0
                };
                (0.0, gap)
            }
            FlexAlign::SpaceAround => {
                let gap = (main_size - total_main + self.gap * (self.children.len() - 1) as f32) / self.children.len() as f32;
                (gap / 2.0, gap)
            }
            FlexAlign::SpaceEvenly => {
                let gap = (main_size - total_main + self.gap * (self.children.len() - 1) as f32) / (self.children.len() + 1) as f32;
                (gap, gap)
            }
        };
        
        // 应用布局
        let indices: Vec<usize> = if is_reverse {
            (0..self.children.len()).rev().collect()
        } else {
            (0..self.children.len()).collect()
        };
        
        for i in indices {
            let child = &mut self.children[i];
            let child_style = child.style_mut();
            
            let child_main = if is_row { child_style.width } else { child_style.height };
            let child_cross = if is_row { child_style.height } else { child_style.width };
            
            // 计算交叉轴位置
            let cross_pos = match self.align_items {
                FlexAlign::Start => 0.0,
                FlexAlign::End => cross_size - child_cross,
                FlexAlign::Center => (cross_size - child_cross) / 2.0,
                _ => 0.0,
            };
            
            if is_row {
                child_style.x = bounds.x + main_pos;
                child_style.y = bounds.y + cross_pos;
            } else {
                child_style.x = bounds.x + cross_pos;
                child_style.y = bounds.y + main_pos;
            }
            
            main_pos += child_main + main_gap;
        }
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Layout {
    fn id(&self) -> ComponentId {
        self.id
    }
    
    fn style(&self) -> &Style {
        &self.style
    }
    
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
    
    fn render(&self, canvas: &mut Canvas) {
        // 绘制背景
        if let Some(bg) = self.style.background_color {
            let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
            canvas.draw_rect(&self.style.bounds(), &paint);
        }
    }
    
    fn children(&self) -> &[Box<dyn Component>] {
        &self.children
    }
    
    fn children_mut(&mut self) -> &mut Vec<Box<dyn Component>> {
        &mut self.children
    }
    
    fn add_child(&mut self, child: Box<dyn Component>) {
        self.children.push(child);
        self.layout();
    }
    
    fn type_name(&self) -> &'static str {
        "Layout"
    }
}
