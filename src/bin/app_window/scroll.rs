//! 微信小程序风格滚动控制器

use std::time::Instant;

/// 视口高度常量
pub const LOGICAL_HEIGHT: u32 = 667;

/// 微信小程序风格滚动控制器
pub struct ScrollController {
    position: f32,
    velocity: f32,
    min_scroll: f32,
    max_scroll: f32,
    last_content_height: f32,
    pub is_dragging: bool,
    drag_start_pos: f32,
    drag_start_scroll: f32,
    velocity_samples: Vec<(f32, Instant)>,
    is_decelerating: bool,
    is_bouncing: bool,
    bounce_start_time: Instant,
    bounce_start_pos: f32,
    bounce_target_pos: f32,
}

impl ScrollController {
    pub fn new(content_height: f32, viewport_height: f32) -> Self {
        Self {
            position: 0.0,
            velocity: 0.0,
            min_scroll: 0.0,
            max_scroll: (content_height - viewport_height).max(0.0),
            last_content_height: content_height,
            is_dragging: false,
            drag_start_pos: 0.0,
            drag_start_scroll: 0.0,
            velocity_samples: Vec::with_capacity(10),
            is_decelerating: false,
            is_bouncing: false,
            bounce_start_time: Instant::now(),
            bounce_start_pos: 0.0,
            bounce_target_pos: 0.0,
        }
    }
    
    /// 更新内容高度（当实际内容高度变化时调用）
    pub fn update_content_height(&mut self, content_height: f32, viewport_height: f32) {
        if (content_height - self.last_content_height).abs() > 1.0 {
            self.last_content_height = content_height;
            let bottom_padding = 160.0;
            let actual_content_height = content_height - bottom_padding;
            self.max_scroll = (actual_content_height - viewport_height).max(0.0);
            if self.position > self.max_scroll {
                self.position = self.max_scroll;
            }
        }
    }
    
    pub fn begin_drag(&mut self, y: f32) {
        self.is_dragging = true;
        self.is_decelerating = false;
        self.is_bouncing = false;
        self.drag_start_pos = y;
        self.drag_start_scroll = self.position;
        self.velocity = 0.0;
        self.velocity_samples.clear();
        self.velocity_samples.push((y, Instant::now()));
    }
    
    pub fn update_drag(&mut self, y: f32) {
        if !self.is_dragging { return; }
        let delta = self.drag_start_pos - y;
        let mut new_pos = self.drag_start_scroll + delta;
        if new_pos < self.min_scroll {
            let overshoot = self.min_scroll - new_pos;
            new_pos = self.min_scroll - Self::rubber_band(overshoot, LOGICAL_HEIGHT as f32);
        } else if new_pos > self.max_scroll {
            let overshoot = new_pos - self.max_scroll;
            new_pos = self.max_scroll + Self::rubber_band(overshoot, LOGICAL_HEIGHT as f32);
        }
        self.position = new_pos;
        let now = Instant::now();
        self.velocity_samples.push((y, now));
        self.velocity_samples.retain(|(_, t)| now.duration_since(*t).as_millis() < 100);
    }
    
    pub fn end_drag(&mut self) -> bool {
        if !self.is_dragging { return false; }
        self.is_dragging = false;
        self.velocity = self.calculate_release_velocity();
        if self.position < self.min_scroll || self.position > self.max_scroll {
            self.start_bounce();
        } else if self.velocity.abs() > 50.0 {
            self.is_decelerating = true;
        }
        self.is_decelerating || self.is_bouncing
    }

    fn calculate_release_velocity(&self) -> f32 {
        if self.velocity_samples.len() < 2 { return 0.0; }
        let first = self.velocity_samples.first().unwrap();
        let last = self.velocity_samples.last().unwrap();
        let dt = last.1.duration_since(first.1).as_secs_f32();
        if dt < 0.001 { return 0.0; }
        (first.0 - last.0) / dt * 0.8
    }
    
    fn rubber_band(offset: f32, dimension: f32) -> f32 {
        let c = 0.55;
        let x = offset.abs() / dimension;
        let result = (1.0 - (1.0 / (x * c + 1.0))) * dimension;
        if offset < 0.0 { -result } else { result }
    }
    
    fn start_bounce(&mut self) {
        self.is_bouncing = true;
        self.is_decelerating = false;
        self.bounce_start_time = Instant::now();
        self.bounce_start_pos = self.position;
        self.bounce_target_pos = self.position.clamp(self.min_scroll, self.max_scroll);
        self.velocity = 0.0;
    }
    
    pub fn update(&mut self, dt: f32) -> bool {
        if self.is_dragging { return false; }
        if self.is_bouncing {
            let elapsed = self.bounce_start_time.elapsed().as_secs_f32();
            let duration = 0.4;
            if elapsed >= duration {
                self.position = self.bounce_target_pos;
                self.is_bouncing = false;
                return false;
            }
            let t = elapsed / duration;
            let ease = 1.0 - (1.0 - t).powi(3);
            self.position = self.bounce_start_pos + (self.bounce_target_pos - self.bounce_start_pos) * ease;
            return true;
        }
        if self.is_decelerating {
            let deceleration = 0.998_f32.powf(dt * 1000.0);
            self.velocity *= deceleration;
            self.position += self.velocity * dt;
            if self.position < self.min_scroll {
                self.position = self.min_scroll;
                self.start_bounce();
                return true;
            } else if self.position > self.max_scroll {
                self.position = self.max_scroll;
                self.start_bounce();
                return true;
            }
            if self.velocity.abs() < 10.0 {
                self.velocity = 0.0;
                self.is_decelerating = false;
                return false;
            }
            return true;
        }
        false
    }
    
    pub fn handle_wheel(&mut self, delta: f32) {
        self.velocity += delta * 15.0;
        self.is_decelerating = true;
        self.is_bouncing = false;
    }
    
    pub fn get_position(&self) -> f32 { self.position }
    pub fn is_animating(&self) -> bool { self.is_decelerating || self.is_bouncing }
}
