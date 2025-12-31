//! 微信小程序风格滚动控制器

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
    // (position, timestamp_ms)
    velocity_samples: Vec<(f32, u64)>,
    is_decelerating: bool,
    is_bouncing: bool,
    bounce_timer: f32,
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
            bounce_timer: 0.0,
            bounce_start_pos: 0.0,
            bounce_target_pos: 0.0,
        }
    }
    
    /// 更新内容高度（当实际内容高度变化时调用）
    pub fn update_content_height(&mut self, content_height: f32, viewport_height: f32) {
        if (content_height - self.last_content_height).abs() > 1.0 || (self.max_scroll - (content_height - viewport_height).max(0.0)).abs() > 1.0 {
            self.last_content_height = content_height;
            // max_scroll = 内容高度 - 视口高度，确保滚动到底时内容底部刚好贴着视口底部
            self.max_scroll = (content_height - viewport_height).max(0.0).floor();
            if self.position > self.max_scroll {
                self.position = self.max_scroll;
            }
        }
    }
    
    pub fn begin_drag(&mut self, y: f32, timestamp: u64) {
        self.is_dragging = true;
        self.is_decelerating = false;
        self.is_bouncing = false;
        self.drag_start_pos = y;
        self.drag_start_scroll = self.position;
        self.velocity = 0.0;
        self.velocity_samples.clear();
        self.velocity_samples.push((y, timestamp));
    }
    
    pub fn update_drag(&mut self, y: f32, timestamp: u64) {
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
        self.velocity_samples.push((y, timestamp));
        // Keep samples from last 100ms
        self.velocity_samples.retain(|(_, t)| timestamp >= *t && timestamp - *t < 100);
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
        // timestamp is in ms, convert to seconds
        let dt = (last.1.saturating_sub(first.1)) as f32 / 1000.0;
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
        self.bounce_timer = 0.0;
        self.bounce_start_pos = self.position;
        self.bounce_target_pos = self.position.clamp(self.min_scroll, self.max_scroll);
        self.velocity = 0.0;
    }
    
    pub fn update(&mut self, dt: f32) -> bool {
        if self.is_dragging { return false; }
        if self.is_bouncing {
            self.bounce_timer += dt;
            let duration = 0.3;
            if self.bounce_timer >= duration {
                self.position = self.bounce_target_pos;
                self.is_bouncing = false;
                return false;
            }
            let t = self.bounce_timer / duration;
            let ease = 1.0 - (1.0 - t).powi(3);
            self.position = self.bounce_start_pos + (self.bounce_target_pos - self.bounce_start_pos) * ease;
            return true;
        }
        if self.is_decelerating {
            // 正常惯性减速
            let deceleration = 0.92_f32.powf(dt * 60.0);
            self.velocity *= deceleration;
            self.position += self.velocity * dt;
            
            // 严格限制在边界内
            self.position = self.position.clamp(self.min_scroll, self.max_scroll);
            
            // 停止条件
            if self.velocity.abs() < 3.0 {
                self.velocity = 0.0;
                self.is_decelerating = false;
                return false;
            }
            return true;
        }
        false
    }
    
    pub fn handle_scroll(&mut self, delta: f32, is_precise: bool) {
        // 忽略极微小的滚动事件
        if delta.abs() < 0.1 {
            return;
        }
        
        // 如果没有可滚动空间，直接返回
        if self.max_scroll <= 0.0 {
            return;
        }
        
        if is_precise {
            // Trackpad: 直接跟随手指移动，严格限制在边界内
            let new_pos = (self.position + delta).clamp(self.min_scroll, self.max_scroll);
            self.position = new_pos;
            
            // 估算速度用于惯性
            let instantaneous_velocity = delta * 60.0;
            self.velocity = self.velocity * 0.6 + instantaneous_velocity * 0.4;
            
            // 标记为减速状态
            self.is_decelerating = true; 
            self.is_bouncing = false;
        } else {
            // Mouse Wheel: 脉冲式滚动，严格限制在边界内
            let new_pos = (self.position + delta * 2.0).clamp(self.min_scroll, self.max_scroll);
            self.position = new_pos;
            self.velocity = 0.0;
            self.is_decelerating = false;
            self.is_bouncing = false;
        }
    }
    
    pub fn get_position(&self) -> f32 { self.position }
    pub fn get_max_scroll(&self) -> f32 { self.max_scroll }
    pub fn is_animating(&self) -> bool { self.is_decelerating || self.is_bouncing }
}
