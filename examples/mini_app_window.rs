//! 带窗口的小程序示例

use mini_render::runtime::MiniApp;
use mini_render::ui::{View, Button, Text, Layout, FlexDirection, FlexAlign, Component};
use mini_render::{Canvas, Color, Paint, PaintStyle, Rect, Path};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Mini App Window Demo");
    
    // 创建应用
    let mut app = MiniApp::new(375, 667)?;
    app.init()?;
    
    // 加载小程序代码
    let code = r#"
        console.log('=== Mini App Demo ===');
        
        App({
            globalData: { count: 0 },
            
            onLaunch: function() {
                console.log('App launched');
            },
            
            onTap: function(e) {
                this.globalData.count++;
                console.log('Tap #' + this.globalData.count + ' at (' + e.x + ', ' + e.y + ')');
                
                if (this.globalData.count % 3 === 0) {
                    wx.showToast({ title: 'Triple tap!', icon: 'success' });
                }
            }
        });
        
        Page({
            route: 'index',
            data: { title: 'Mini App Demo' },
            
            onLoad: function() {
                console.log('Index page loaded');
                
                // 使用 Canvas API
                var ctx = wx.createCanvasContext('myCanvas');
                ctx.setFillStyle('#4A90D9');
                ctx.fillRect(10, 10, 100, 50);
                ctx.setFillStyle('#E74C3C');
                ctx.arc(200, 35, 25, 0, 2 * Math.PI);
                ctx.fill();
                ctx.draw();
            }
        });
        
        console.log('Demo code loaded');
    "#;
    
    app.load_script(code)?;
    app.start()?;
    
    // 创建 UI 组件
    let mut canvas = Canvas::new(375, 667);
    
    // 主循环
    let start = Instant::now();
    let mut frame = 0;
    
    while start.elapsed() < Duration::from_secs(3) {
        app.update()?;
        
        // 渲染 UI
        canvas.clear(Color::from_hex(0xF5F5F5));
        
        // 状态栏
        let status_bar = Rect::new(0.0, 0.0, 375.0, 44.0);
        canvas.draw_rect(&status_bar, &Paint::new().with_color(Color::from_hex(0x007AFF)));
        
        // 导航栏
        let nav_bar = Rect::new(0.0, 44.0, 375.0, 44.0);
        canvas.draw_rect(&nav_bar, &Paint::new().with_color(Color::WHITE));
        
        // 内容区域
        let content = Rect::new(16.0, 100.0, 343.0, 200.0);
        let mut path = Path::new();
        path.add_round_rect(content.x, content.y, content.width, content.height, 12.0);
        canvas.draw_path(&path, &Paint::new().with_color(Color::WHITE));
        
        // 按钮
        let btn_rect = Rect::new(100.0, 350.0, 175.0, 44.0);
        let mut btn_path = Path::new();
        btn_path.add_round_rect(btn_rect.x, btn_rect.y, btn_rect.width, btn_rect.height, 22.0);
        canvas.draw_path(&btn_path, &Paint::new().with_color(Color::from_hex(0x07C160)));
        
        // 底部 TabBar
        let tab_bar = Rect::new(0.0, 617.0, 375.0, 50.0);
        canvas.draw_rect(&tab_bar, &Paint::new().with_color(Color::WHITE));
        
        // Tab 图标
        for i in 0..4 {
            let x = 47.0 + i as f32 * 94.0;
            canvas.draw_circle(x, 637.0, 12.0, &Paint::new().with_color(Color::from_hex(0xCCCCCC)));
        }
        
        // 模拟点击
        if frame == 30 {
            app.on_tap(187.0, 372.0)?;
        }
        
        frame += 1;
        std::thread::sleep(Duration::from_millis(16));
    }
    
    // 保存结果
    canvas.save_png("mini_app_window.png")?;
    println!("✅ Saved to mini_app_window.png");
    
    app.stop();
    Ok(())
}
