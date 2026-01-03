//! Canvas 组件 - 微信小程序 Canvas 2D API 实现

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Canvas 组件
pub struct CanvasComponent;

/// Canvas 2D 上下文 - 实现微信小程序 Canvas 2D API
#[derive(Clone)]
pub struct Canvas2DContext {
    /// canvas-id
    pub canvas_id: String,
    /// 画布宽度
    pub width: u32,
    /// 画布高度
    pub height: u32,
    /// 内部画布
    canvas: Arc<Mutex<Canvas>>,
    /// 当前填充颜色
    fill_style: Color,
    /// 当前描边颜色
    stroke_style: Color,
    /// 线宽
    line_width: f32,
    /// 字体大小
    font_size: f32,
    /// 文本对齐
    text_align: TextAlign,
    /// 文本基线
    text_baseline: TextBaseline,
    /// 全局透明度
    global_alpha: f32,
    /// 当前路径
    current_path: Vec<PathCommand>,
    /// 状态栈
    state_stack: Vec<ContextState>,
}


/// 文本基线
#[derive(Clone, Copy, Default)]
pub enum TextBaseline {
    Top,
    Hanging,
    #[default]
    Middle,
    Alphabetic,
    Ideographic,
    Bottom,
}

/// 路径命令
#[derive(Clone)]
enum PathCommand {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    Arc(f32, f32, f32, f32, f32, bool),
    QuadraticCurveTo(f32, f32, f32, f32),
    BezierCurveTo(f32, f32, f32, f32, f32, f32),
    Rect(f32, f32, f32, f32),
    ClosePath,
}

/// 上下文状态（用于 save/restore）
#[derive(Clone)]
struct ContextState {
    fill_style: Color,
    stroke_style: Color,
    line_width: f32,
    font_size: f32,
    text_align: TextAlign,
    text_baseline: TextBaseline,
    global_alpha: f32,
}

impl Canvas2DContext {
    pub fn new(canvas_id: &str, width: u32, height: u32) -> Self {
        let canvas = Canvas::new(width, height);
        Self {
            canvas_id: canvas_id.to_string(),
            width,
            height,
            canvas: Arc::new(Mutex::new(canvas)),
            fill_style: Color::BLACK,
            stroke_style: Color::BLACK,
            line_width: 1.0,
            font_size: 10.0,
            text_align: TextAlign::Left,
            text_baseline: TextBaseline::default(),
            global_alpha: 1.0,
            current_path: Vec::new(),
            state_stack: Vec::new(),
        }
    }

    // ========== 状态管理 ==========
    
    /// 保存当前状态
    pub fn save(&mut self) {
        self.state_stack.push(ContextState {
            fill_style: self.fill_style,
            stroke_style: self.stroke_style,
            line_width: self.line_width,
            font_size: self.font_size,
            text_align: self.text_align,
            text_baseline: self.text_baseline,
            global_alpha: self.global_alpha,
        });
        if let Ok(mut canvas) = self.canvas.lock() {
            canvas.save();
        }
    }
    
    /// 恢复上一次保存的状态
    pub fn restore(&mut self) {
        if let Some(state) = self.state_stack.pop() {
            self.fill_style = state.fill_style;
            self.stroke_style = state.stroke_style;
            self.line_width = state.line_width;
            self.font_size = state.font_size;
            self.text_align = state.text_align;
            self.text_baseline = state.text_baseline;
            self.global_alpha = state.global_alpha;
        }
        if let Ok(mut canvas) = self.canvas.lock() {
            canvas.restore();
        }
    }

    // ========== 样式设置 ==========
    
    /// 设置填充颜色
    pub fn set_fill_style(&mut self, color: &str) {
        if let Some(c) = parse_color_str(color) {
            self.fill_style = c;
        }
    }
    
    /// 设置描边颜色
    pub fn set_stroke_style(&mut self, color: &str) {
        if let Some(c) = parse_color_str(color) {
            self.stroke_style = c;
        }
    }
    
    /// 设置线宽
    pub fn set_line_width(&mut self, width: f32) {
        self.line_width = width;
    }

    /// 设置全局透明度
    pub fn set_global_alpha(&mut self, alpha: f32) {
        self.global_alpha = alpha.clamp(0.0, 1.0);
    }
    
    /// 设置字体
    pub fn set_font(&mut self, font: &str) {
        // 解析字体字符串，如 "16px sans-serif"
        for part in font.split_whitespace() {
            if part.ends_with("px") {
                if let Ok(size) = part.trim_end_matches("px").parse::<f32>() {
                    self.font_size = size;
                }
            }
        }
    }
    
    /// 设置文本对齐
    pub fn set_text_align(&mut self, align: &str) {
        self.text_align = match align {
            "center" => TextAlign::Center,
            "right" | "end" => TextAlign::Right,
            _ => TextAlign::Left,
        };
    }
    
    /// 设置文本基线
    pub fn set_text_baseline(&mut self, baseline: &str) {
        self.text_baseline = match baseline {
            "top" => TextBaseline::Top,
            "hanging" => TextBaseline::Hanging,
            "middle" => TextBaseline::Middle,
            "alphabetic" => TextBaseline::Alphabetic,
            "ideographic" => TextBaseline::Ideographic,
            "bottom" => TextBaseline::Bottom,
            _ => TextBaseline::Middle,
        };
    }

    // ========== 矩形绑制 ==========
    
    /// 清除矩形区域（设置为透明）
    pub fn clear_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        if let Ok(mut canvas) = self.canvas.lock() {
            let x0 = x.max(0.0) as i32;
            let y0 = y.max(0.0) as i32;
            let x1 = (x + width).min(canvas.width() as f32) as i32;
            let y1 = (y + height).min(canvas.height() as f32) as i32;
            
            // 直接设置像素为透明
            for py in y0..y1 {
                for px in x0..x1 {
                    canvas.set_pixel_direct(px, py, Color::TRANSPARENT);
                }
            }
        }
    }
    
    /// 填充矩形
    pub fn fill_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        if let Ok(mut canvas) = self.canvas.lock() {
            let rect = GeoRect::new(x, y, width, height);
            let paint = Paint::new()
                .with_color(self.apply_alpha(self.fill_style))
                .with_style(PaintStyle::Fill);
            canvas.draw_rect(&rect, &paint);
        }
    }

    /// 描边矩形
    pub fn stroke_rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        if let Ok(mut canvas) = self.canvas.lock() {
            let rect = GeoRect::new(x, y, width, height);
            let paint = Paint::new()
                .with_color(self.apply_alpha(self.stroke_style))
                .with_style(PaintStyle::Stroke)
                .with_stroke_width(self.line_width);
            canvas.draw_rect(&rect, &paint);
        }
    }

    // ========== 路径绑制 ==========
    
    /// 开始新路径
    pub fn begin_path(&mut self) {
        self.current_path.clear();
    }
    
    /// 关闭路径
    pub fn close_path(&mut self) {
        self.current_path.push(PathCommand::ClosePath);
    }
    
    /// 移动到指定点
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.current_path.push(PathCommand::MoveTo(x, y));
    }
    
    /// 绘制直线到指定点
    pub fn line_to(&mut self, x: f32, y: f32) {
        self.current_path.push(PathCommand::LineTo(x, y));
    }
    
    /// 绘制圆弧
    pub fn arc(&mut self, x: f32, y: f32, radius: f32, start_angle: f32, end_angle: f32, counter_clockwise: bool) {
        self.current_path.push(PathCommand::Arc(x, y, radius, start_angle, end_angle, counter_clockwise));
    }
    
    /// 绘制二次贝塞尔曲线
    pub fn quadratic_curve_to(&mut self, cpx: f32, cpy: f32, x: f32, y: f32) {
        self.current_path.push(PathCommand::QuadraticCurveTo(cpx, cpy, x, y));
    }
    
    /// 绘制三次贝塞尔曲线
    pub fn bezier_curve_to(&mut self, cp1x: f32, cp1y: f32, cp2x: f32, cp2y: f32, x: f32, y: f32) {
        self.current_path.push(PathCommand::BezierCurveTo(cp1x, cp1y, cp2x, cp2y, x, y));
    }
    
    /// 添加矩形路径
    pub fn rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.current_path.push(PathCommand::Rect(x, y, width, height));
    }

    /// 填充当前路径
    pub fn fill(&mut self) {
        let path = self.build_path();
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new()
                .with_color(self.apply_alpha(self.fill_style))
                .with_style(PaintStyle::Fill)
                .with_anti_alias(true);
            canvas.draw_path(&path, &paint);
        }
    }
    
    /// 描边当前路径
    pub fn stroke(&mut self) {
        let path = self.build_path();
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new()
                .with_color(self.apply_alpha(self.stroke_style))
                .with_style(PaintStyle::Stroke)
                .with_stroke_width(self.line_width)
                .with_anti_alias(true);
            canvas.draw_path(&path, &paint);
        }
    }
    
    /// 构建 Path 对象
    fn build_path(&self) -> Path {
        let mut path = Path::new();
        for cmd in &self.current_path {
            match cmd {
                PathCommand::MoveTo(x, y) => { path.move_to(*x, *y); }
                PathCommand::LineTo(x, y) => { path.line_to(*x, *y); }
                PathCommand::Arc(x, y, r, start, end, ccw) => {
                    path.arc(*x, *y, *r, *start, *end, *ccw);
                }
                PathCommand::QuadraticCurveTo(cpx, cpy, x, y) => {
                    path.quad_to(*cpx, *cpy, *x, *y);
                }
                PathCommand::BezierCurveTo(cp1x, cp1y, cp2x, cp2y, x, y) => {
                    path.cubic_to(*cp1x, *cp1y, *cp2x, *cp2y, *x, *y);
                }
                PathCommand::Rect(x, y, w, h) => {
                    path.move_to(*x, *y);
                    path.line_to(*x + *w, *y);
                    path.line_to(*x + *w, *y + *h);
                    path.line_to(*x, *y + *h);
                    path.close();
                }
                PathCommand::ClosePath => { path.close(); }
            }
        }
        path
    }

    // ========== 圆形绘制 ==========
    
    /// 绘制填充圆
    pub fn fill_circle(&mut self, x: f32, y: f32, radius: f32) {
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new()
                .with_color(self.apply_alpha(self.fill_style))
                .with_style(PaintStyle::Fill)
                .with_anti_alias(true);
            canvas.draw_circle(x, y, radius, &paint);
        }
    }

    /// 绘制描边圆
    pub fn stroke_circle(&mut self, x: f32, y: f32, radius: f32) {
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new()
                .with_color(self.apply_alpha(self.stroke_style))
                .with_style(PaintStyle::Stroke)
                .with_stroke_width(self.line_width)
                .with_anti_alias(true);
            canvas.draw_circle(x, y, radius, &paint);
        }
    }

    // ========== 线条绘制 ==========
    
    /// 绘制线条（从当前点到指定点）
    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        if let Ok(mut canvas) = self.canvas.lock() {
            let paint = Paint::new()
                .with_color(self.apply_alpha(self.stroke_style))
                .with_stroke_width(self.line_width)
                .with_anti_alias(true);
            canvas.draw_line(x1, y1, x2, y2, &paint);
        }
    }

    // ========== 变换 ==========
    
    /// 平移
    pub fn translate(&mut self, x: f32, y: f32) {
        if let Ok(mut canvas) = self.canvas.lock() {
            canvas.translate(x, y);
        }
    }

    // ========== 渐变 ==========
    
    /// 创建线性渐变
    pub fn create_linear_gradient(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> LinearGradient {
        LinearGradient::new(x0, y0, x1, y1)
    }
    
    /// 创建径向渐变
    pub fn create_radial_gradient(&self, x0: f32, y0: f32, r0: f32, x1: f32, y1: f32, r1: f32) -> RadialGradient {
        RadialGradient::new(x0, y0, r0, x1, y1, r1)
    }

    // ========== 辅助方法 ==========
    
    /// 应用全局透明度
    fn apply_alpha(&self, color: Color) -> Color {
        if self.global_alpha >= 1.0 {
            color
        } else {
            Color::new(color.r, color.g, color.b, (color.a as f32 * self.global_alpha) as u8)
        }
    }
    
    /// 获取画布像素数据
    pub fn get_image_data(&self) -> Vec<u8> {
        if let Ok(canvas) = self.canvas.lock() {
            canvas.to_rgba()
        } else {
            Vec::new()
        }
    }
    
    /// 获取内部 Canvas 引用（用于渲染）
    pub fn get_canvas(&self) -> Arc<Mutex<Canvas>> {
        self.canvas.clone()
    }
}


/// 线性渐变
#[derive(Clone)]
pub struct LinearGradient {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    stops: Vec<(f32, Color)>,
}

impl LinearGradient {
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self { x0, y0, x1, y1, stops: Vec::new() }
    }
    
    /// 添加颜色停止点
    pub fn add_color_stop(&mut self, offset: f32, color: &str) {
        if let Some(c) = parse_color_str(color) {
            self.stops.push((offset.clamp(0.0, 1.0), c));
            self.stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        }
    }
    
    /// 获取指定位置的颜色
    pub fn get_color_at(&self, x: f32, y: f32) -> Color {
        if self.stops.is_empty() {
            return Color::TRANSPARENT;
        }
        if self.stops.len() == 1 {
            return self.stops[0].1;
        }
        
        // 计算点在渐变线上的投影位置
        let dx = self.x1 - self.x0;
        let dy = self.y1 - self.y0;
        let len_sq = dx * dx + dy * dy;
        if len_sq == 0.0 {
            return self.stops[0].1;
        }
        
        let t = ((x - self.x0) * dx + (y - self.y0) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);
        
        self.interpolate_color(t)
    }
    
    fn interpolate_color(&self, t: f32) -> Color {
        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }
        if t >= self.stops.last().unwrap().0 {
            return self.stops.last().unwrap().1;
        }
        
        for i in 0..self.stops.len() - 1 {
            let (t0, c0) = &self.stops[i];
            let (t1, c1) = &self.stops[i + 1];
            if t >= *t0 && t <= *t1 {
                let ratio = (t - t0) / (t1 - t0);
                return Color::new(
                    (c0.r as f32 + (c1.r as f32 - c0.r as f32) * ratio) as u8,
                    (c0.g as f32 + (c1.g as f32 - c0.g as f32) * ratio) as u8,
                    (c0.b as f32 + (c1.b as f32 - c0.b as f32) * ratio) as u8,
                    (c0.a as f32 + (c1.a as f32 - c0.a as f32) * ratio) as u8,
                );
            }
        }
        self.stops[0].1
    }
}


/// 径向渐变
#[derive(Clone)]
pub struct RadialGradient {
    x0: f32,
    y0: f32,
    r0: f32,
    x1: f32,
    y1: f32,
    r1: f32,
    stops: Vec<(f32, Color)>,
}

impl RadialGradient {
    pub fn new(x0: f32, y0: f32, r0: f32, x1: f32, y1: f32, r1: f32) -> Self {
        Self { x0, y0, r0, x1, y1, r1, stops: Vec::new() }
    }
    
    /// 添加颜色停止点
    pub fn add_color_stop(&mut self, offset: f32, color: &str) {
        if let Some(c) = parse_color_str(color) {
            self.stops.push((offset.clamp(0.0, 1.0), c));
            self.stops.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        }
    }
    
    /// 获取指定位置的颜色
    pub fn get_color_at(&self, x: f32, y: f32) -> Color {
        if self.stops.is_empty() {
            return Color::TRANSPARENT;
        }
        
        // 简化实现：使用到中心点的距离
        let dx = x - self.x1;
        let dy = y - self.y1;
        let dist = (dx * dx + dy * dy).sqrt();
        let t = if self.r1 > self.r0 {
            ((dist - self.r0) / (self.r1 - self.r0)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        
        self.interpolate_color(t)
    }
    
    fn interpolate_color(&self, t: f32) -> Color {
        if self.stops.is_empty() {
            return Color::TRANSPARENT;
        }
        if self.stops.len() == 1 {
            return self.stops[0].1;
        }
        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }
        if t >= self.stops.last().unwrap().0 {
            return self.stops.last().unwrap().1;
        }
        
        for i in 0..self.stops.len() - 1 {
            let (t0, c0) = &self.stops[i];
            let (t1, c1) = &self.stops[i + 1];
            if t >= *t0 && t <= *t1 {
                let ratio = (t - t0) / (t1 - t0);
                return Color::new(
                    (c0.r as f32 + (c1.r as f32 - c0.r as f32) * ratio) as u8,
                    (c0.g as f32 + (c1.g as f32 - c0.g as f32) * ratio) as u8,
                    (c0.b as f32 + (c1.b as f32 - c0.b as f32) * ratio) as u8,
                    (c0.a as f32 + (c1.a as f32 - c0.a as f32) * ratio) as u8,
                );
            }
        }
        self.stops[0].1
    }
}


/// Canvas 上下文管理器 - 全局管理所有 canvas 实例
pub struct CanvasContextManager {
    contexts: HashMap<String, Canvas2DContext>,
}

impl CanvasContextManager {
    pub fn new() -> Self {
        Self { contexts: HashMap::new() }
    }
    
    /// 获取或创建 canvas 上下文
    pub fn get_context(&mut self, canvas_id: &str, width: u32, height: u32) -> &mut Canvas2DContext {
        self.contexts.entry(canvas_id.to_string())
            .or_insert_with(|| Canvas2DContext::new(canvas_id, width, height))
    }
    
    /// 获取已存在的上下文
    pub fn get_existing_context(&self, canvas_id: &str) -> Option<&Canvas2DContext> {
        self.contexts.get(canvas_id)
    }
    
    /// 移除上下文
    pub fn remove_context(&mut self, canvas_id: &str) {
        self.contexts.remove(canvas_id);
    }
    
    /// 清除所有上下文
    pub fn clear(&mut self) {
        self.contexts.clear();
    }
    
    /// 执行绘制命令
    pub fn execute_commands(&mut self, canvas_id: &str, commands_json: &str) {
        // 解析命令
        let commands: Vec<serde_json::Value> = serde_json::from_str(commands_json).unwrap_or_default();
        
        // 获取或创建上下文（使用较大的默认尺寸以适应大多数 canvas）
        let ctx = self.contexts.entry(canvas_id.to_string())
            .or_insert_with(|| Canvas2DContext::new(canvas_id, 400, 300));
        
        // 执行每个命令
        for cmd in commands {
            let cmd_type = cmd.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match cmd_type {
                "setFillStyle" => {
                    if let Some(color) = cmd.get("color").and_then(|v| v.as_str()) {
                        ctx.set_fill_style(color);
                    }
                }
                "setStrokeStyle" => {
                    if let Some(color) = cmd.get("color").and_then(|v| v.as_str()) {
                        ctx.set_stroke_style(color);
                    }
                }
                "setLineWidth" => {
                    if let Some(width) = cmd.get("width").and_then(|v| v.as_f64()) {
                        ctx.set_line_width(width as f32);
                    }
                }
                "fillRect" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let w = cmd.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let h = cmd.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.fill_rect(x, y, w, h);
                }
                "strokeRect" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let w = cmd.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let h = cmd.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.stroke_rect(x, y, w, h);
                }
                "clearRect" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let w = cmd.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let h = cmd.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.clear_rect(x, y, w, h);
                }
                "beginPath" => ctx.begin_path(),
                "closePath" => ctx.close_path(),
                "moveTo" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.move_to(x, y);
                }
                "lineTo" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.line_to(x, y);
                }
                "arc" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let r = cmd.get("r").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let s = cmd.get("sAngle").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let e = cmd.get("eAngle").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let cc = cmd.get("counterclockwise").and_then(|v| v.as_bool()).unwrap_or(false);
                    ctx.arc(x, y, r, s, e, cc);
                }
                "fill" => ctx.fill(),
                "stroke" => ctx.stroke(),
                "save" => ctx.save(),
                "restore" => ctx.restore(),
                "translate" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.translate(x, y);
                }
                _ => {}
            }
        }
    }
}

impl Default for CanvasContextManager {
    fn default() -> Self {
        Self::new()
    }
}

// ========== Canvas 组件实现 ==========

use once_cell::sync::Lazy;

/// 全局 Canvas 上下文管理器
pub static CANVAS_MANAGER: Lazy<Mutex<CanvasContextManager>> = Lazy::new(|| {
    Mutex::new(CanvasContextManager::new())
});

/// 执行 Canvas 绘制命令（供外部调用）
pub fn execute_canvas_draw(canvas_id: &str, commands_json: &str) {
    println!("[Canvas] execute_canvas_draw: {} commands for '{}'", 
        commands_json.len(), canvas_id);
    if let Ok(mut manager) = CANVAS_MANAGER.lock() {
        manager.execute_commands(canvas_id, commands_json);
    }
}

impl CanvasComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        // 设置默认背景色为白色
        if ns.background_color.is_none() {
            ns.background_color = Some(Color::WHITE);
        }
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "canvas".into(),
            text: String::new(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    /// 绘制 canvas 组件
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        // 绘制背景
        draw_background(canvas, &node.style, x, y, w, h);
        
        // 获取 canvas-id
        let canvas_id = node.attrs.get("canvas-id").cloned().unwrap_or_default();
        
        // 从全局管理器获取绘制内容并复制到主 canvas
        if !canvas_id.is_empty() {
            if let Ok(manager) = CANVAS_MANAGER.lock() {
                if let Some(ctx) = manager.get_existing_context(&canvas_id) {
                    // 获取 canvas 上下文的像素数据
                    if let Ok(src_canvas) = ctx.get_canvas().lock() {
                        let src_pixels = src_canvas.pixels();
                        let src_w = src_canvas.width() as usize;
                        let src_h = src_canvas.height() as usize;
                        
                        // 复制像素到目标位置
                        let dst_x = x as i32;
                        let dst_y = y as i32;
                        let copy_w = (w as usize).min(src_w);
                        let copy_h = (h as usize).min(src_h);
                        
                        for sy in 0..copy_h {
                            for sx in 0..copy_w {
                                let src_idx = sy * src_w + sx;
                                if src_idx < src_pixels.len() {
                                    let color = src_pixels[src_idx];
                                    if color.a > 0 {
                                        canvas.set_pixel(dst_x + sx as i32, dst_y + sy as i32, color);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 绘制边框
        if node.style.border_width > 0.0 {
            let border_color = node.style.border_color.unwrap_or(Color::from_hex(0xE5E5E5));
            let paint = Paint::new()
                .with_color(border_color)
                .with_style(PaintStyle::Stroke)
                .with_stroke_width(node.style.border_width);
            let rect = GeoRect::new(x, y, w, h);
            canvas.draw_rect(&rect, &paint);
        }
    }
}
