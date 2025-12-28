//! 路径模块 - 支持贝塞尔曲线等复杂路径

use crate::geometry::Point;

/// 路径命令
#[derive(Debug, Clone)]
pub enum PathCommand {
    MoveTo(Point),
    LineTo(Point),
    QuadTo(Point, Point),      // 控制点, 终点
    CubicTo(Point, Point, Point), // 控制点1, 控制点2, 终点
    Close,
}

/// 路径
#[derive(Debug, Clone, Default)]
pub struct Path {
    commands: Vec<PathCommand>,
    current: Point,
}

impl Path {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn move_to(&mut self, x: f32, y: f32) -> &mut Self {
        let p = Point::new(x, y);
        self.commands.push(PathCommand::MoveTo(p));
        self.current = p;
        self
    }

    pub fn line_to(&mut self, x: f32, y: f32) -> &mut Self {
        let p = Point::new(x, y);
        self.commands.push(PathCommand::LineTo(p));
        self.current = p;
        self
    }

    pub fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) -> &mut Self {
        let ctrl = Point::new(cx, cy);
        let end = Point::new(x, y);
        self.commands.push(PathCommand::QuadTo(ctrl, end));
        self.current = end;
        self
    }

    pub fn cubic_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) -> &mut Self {
        let c1 = Point::new(c1x, c1y);
        let c2 = Point::new(c2x, c2y);
        let end = Point::new(x, y);
        self.commands.push(PathCommand::CubicTo(c1, c2, end));
        self.current = end;
        self
    }

    pub fn close(&mut self) -> &mut Self {
        self.commands.push(PathCommand::Close);
        self
    }

    /// 添加矩形
    pub fn add_rect(&mut self, x: f32, y: f32, w: f32, h: f32) -> &mut Self {
        self.move_to(x, y)
            .line_to(x + w, y)
            .line_to(x + w, y + h)
            .line_to(x, y + h)
            .close()
    }

    /// 添加圆角矩形
    pub fn add_round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32) -> &mut Self {
        let r = r.min(w / 2.0).min(h / 2.0);
        let k = 0.5522847498; // 贝塞尔曲线近似圆弧的系数
        let kr = k * r;

        self.move_to(x + r, y);
        self.line_to(x + w - r, y);
        self.cubic_to(x + w - r + kr, y, x + w, y + r - kr, x + w, y + r);
        self.line_to(x + w, y + h - r);
        self.cubic_to(x + w, y + h - r + kr, x + w - r + kr, y + h, x + w - r, y + h);
        self.line_to(x + r, y + h);
        self.cubic_to(x + r - kr, y + h, x, y + h - r + kr, x, y + h - r);
        self.line_to(x, y + r);
        self.cubic_to(x, y + r - kr, x + r - kr, y, x + r, y);
        self.close()
    }

    /// 添加椭圆
    pub fn add_oval(&mut self, cx: f32, cy: f32, rx: f32, ry: f32) -> &mut Self {
        let k = 0.5522847498;
        let kx = k * rx;
        let ky = k * ry;

        self.move_to(cx + rx, cy);
        self.cubic_to(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry);
        self.cubic_to(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy);
        self.cubic_to(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry);
        self.cubic_to(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy);
        self.close()
    }
    
    /// 添加圆形
    pub fn add_circle(&mut self, cx: f32, cy: f32, r: f32) -> &mut Self {
        self.add_oval(cx, cy, r, r)
    }

    pub fn commands(&self) -> &[PathCommand] {
        &self.commands
    }

    /// 将路径转换为点序列（用于光栅化）
    pub fn flatten(&self, tolerance: f32) -> Vec<Vec<Point>> {
        let mut contours = Vec::new();
        let mut current_contour = Vec::new();
        let mut current = Point::default();
        let mut start = Point::default();

        for cmd in &self.commands {
            match cmd {
                PathCommand::MoveTo(p) => {
                    if !current_contour.is_empty() {
                        contours.push(std::mem::take(&mut current_contour));
                    }
                    current = *p;
                    start = *p;
                    current_contour.push(*p);
                }
                PathCommand::LineTo(p) => {
                    current_contour.push(*p);
                    current = *p;
                }
                PathCommand::QuadTo(ctrl, end) => {
                    flatten_quad(&current, ctrl, end, tolerance, &mut current_contour);
                    current = *end;
                }
                PathCommand::CubicTo(c1, c2, end) => {
                    flatten_cubic(&current, c1, c2, end, tolerance, &mut current_contour);
                    current = *end;
                }
                PathCommand::Close => {
                    if current != start {
                        current_contour.push(start);
                    }
                    current = start;
                }
            }
        }

        if !current_contour.is_empty() {
            contours.push(current_contour);
        }

        contours
    }
}

/// 二次贝塞尔曲线展平
fn flatten_quad(p0: &Point, p1: &Point, p2: &Point, tolerance: f32, out: &mut Vec<Point>) {
    let steps = ((p0.distance(p1) + p1.distance(p2)) / tolerance).ceil() as usize;
    let steps = steps.max(2).min(100);

    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let t2 = t * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;

        out.push(Point::new(
            mt2 * p0.x + 2.0 * mt * t * p1.x + t2 * p2.x,
            mt2 * p0.y + 2.0 * mt * t * p1.y + t2 * p2.y,
        ));
    }
}

/// 三次贝塞尔曲线展平
fn flatten_cubic(p0: &Point, p1: &Point, p2: &Point, p3: &Point, tolerance: f32, out: &mut Vec<Point>) {
    let steps = ((p0.distance(p1) + p1.distance(p2) + p2.distance(p3)) / tolerance).ceil() as usize;
    let steps = steps.max(2).min(100);

    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;

        out.push(Point::new(
            mt3 * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t3 * p3.x,
            mt3 * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t3 * p3.y,
        ));
    }
}
