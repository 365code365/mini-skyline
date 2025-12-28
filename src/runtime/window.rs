//! 窗口管理

/// 窗口配置
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Mini App".to_string(),
            width: 375,
            height: 667,
            resizable: true,
        }
    }
}

/// 窗口抽象
pub struct Window {
    config: WindowConfig,
}

impl Window {
    pub fn new(config: WindowConfig) -> Self {
        Self { config }
    }
    
    pub fn width(&self) -> u32 {
        self.config.width
    }
    
    pub fn height(&self) -> u32 {
        self.config.height
    }
    
    pub fn title(&self) -> &str {
        &self.config.title
    }
}
