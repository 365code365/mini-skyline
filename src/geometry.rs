//! 几何图形模块

/// 2D 点
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance(&self, other: &Point) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn lerp(&self, other: &Point, t: f32) -> Point {
        Point {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

/// 尺寸
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// 矩形
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    pub fn from_points(p1: Point, p2: Point) -> Self {
        let x = p1.x.min(p2.x);
        let y = p1.y.min(p2.y);
        let width = (p1.x - p2.x).abs();
        let height = (p1.y - p2.y).abs();
        Self { x, y, width, height }
    }

    pub fn left(&self) -> f32 { self.x }
    pub fn top(&self) -> f32 { self.y }
    pub fn right(&self) -> f32 { self.x + self.width }
    pub fn bottom(&self) -> f32 { self.y + self.height }

    pub fn contains(&self, point: &Point) -> bool {
        point.x >= self.x && point.x <= self.right() &&
        point.y >= self.y && point.y <= self.bottom()
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right() && self.right() > other.x &&
        self.y < other.bottom() && self.bottom() > other.y
    }

    pub fn inset(&self, dx: f32, dy: f32) -> Rect {
        Rect {
            x: self.x + dx,
            y: self.y + dy,
            width: (self.width - 2.0 * dx).max(0.0),
            height: (self.height - 2.0 * dy).max(0.0),
        }
    }
}
