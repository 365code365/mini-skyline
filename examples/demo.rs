//! 渲染引擎示例 - 展示各种绑制功能

use mini_render::{Canvas, Color, Paint, PaintStyle, Path, Rect};

fn main() {
    // 创建 800x600 画布
    let mut canvas = Canvas::new(800, 600);

    // 清空为白色背景
    canvas.clear(Color::WHITE);

    // 1. 绘制填充矩形
    let rect_paint = Paint::new()
        .with_color(Color::from_hex(0x4A90D9))
        .with_style(PaintStyle::Fill);
    canvas.draw_rect(&Rect::new(50.0, 50.0, 200.0, 100.0), &rect_paint);

    // 2. 绘制描边矩形
    let stroke_paint = Paint::new()
        .with_color(Color::from_hex(0xE74C3C))
        .with_style(PaintStyle::Stroke)
        .with_stroke_width(3.0);
    canvas.draw_rect(&Rect::new(50.0, 180.0, 200.0, 100.0), &stroke_paint);

    // 3. 绘制填充圆形
    let circle_paint = Paint::new()
        .with_color(Color::from_hex(0x2ECC71))
        .with_style(PaintStyle::Fill)
        .with_anti_alias(true);
    canvas.draw_circle(450.0, 100.0, 60.0, &circle_paint);

    // 4. 绘制描边圆形
    let circle_stroke = Paint::new()
        .with_color(Color::from_hex(0x9B59B6))
        .with_style(PaintStyle::Stroke)
        .with_stroke_width(4.0)
        .with_anti_alias(true);
    canvas.draw_circle(450.0, 250.0, 60.0, &circle_stroke);

    // 5. 绘制线段
    let line_paint = Paint::new()
        .with_color(Color::from_hex(0xF39C12))
        .with_stroke_width(2.0)
        .with_anti_alias(true);
    canvas.draw_line(300.0, 350.0, 500.0, 450.0, &line_paint);
    canvas.draw_line(300.0, 450.0, 500.0, 350.0, &line_paint);

    // 6. 绘制圆角矩形路径
    let mut round_rect_path = Path::new();
    round_rect_path.add_round_rect(550.0, 50.0, 200.0, 120.0, 20.0);
    let round_rect_paint = Paint::new()
        .with_color(Color::from_hex(0x1ABC9C))
        .with_style(PaintStyle::Fill);
    canvas.draw_path(&round_rect_path, &round_rect_paint);

    // 7. 绘制椭圆
    let mut oval_path = Path::new();
    oval_path.add_oval(650.0, 280.0, 80.0, 50.0);
    let oval_paint = Paint::new()
        .with_color(Color::from_hex(0xE91E63))
        .with_style(PaintStyle::Fill);
    canvas.draw_path(&oval_path, &oval_paint);

    // 8. 绘制自定义路径（星形）
    let mut star_path = Path::new();
    let cx = 150.0;
    let cy = 450.0;
    let outer_r = 80.0;
    let inner_r = 35.0;
    let points = 5;

    for i in 0..(points * 2) {
        let angle = std::f32::consts::PI / 2.0 - (i as f32) * std::f32::consts::PI / points as f32;
        let r = if i % 2 == 0 { outer_r } else { inner_r };
        let x = cx + r * angle.cos();
        let y = cy - r * angle.sin();

        if i == 0 {
            star_path.move_to(x, y);
        } else {
            star_path.line_to(x, y);
        }
    }
    star_path.close();

    let star_paint = Paint::new()
        .with_color(Color::from_hex(0xFFD700))
        .with_style(PaintStyle::Fill);
    canvas.draw_path(&star_path, &star_paint);

    // 9. 绘制贝塞尔曲线
    let mut bezier_path = Path::new();
    bezier_path.move_to(550.0, 400.0);
    bezier_path.cubic_to(600.0, 350.0, 700.0, 500.0, 750.0, 400.0);

    let bezier_paint = Paint::new()
        .with_color(Color::from_hex(0x3498DB))
        .with_style(PaintStyle::Stroke)
        .with_stroke_width(3.0);
    canvas.draw_path(&bezier_path, &bezier_paint);

    // 10. 绘制半透明叠加效果
    let transparent_paint = Paint::new()
        .with_color(Color::new(255, 0, 0, 128))
        .with_style(PaintStyle::Fill);
    canvas.draw_rect(&Rect::new(100.0, 100.0, 150.0, 150.0), &transparent_paint);

    // 11. 绘制渐变效果（手动实现）
    for i in 0..100 {
        let t = i as f32 / 100.0;
        let r = (255.0 * (1.0 - t)) as u8;
        let b = (255.0 * t) as u8;
        let gradient_paint = Paint::new()
            .with_color(Color::rgb(r, 0, b))
            .with_style(PaintStyle::Fill);
        canvas.draw_rect(&Rect::new(300.0 + i as f32 * 2.0, 500.0, 2.0, 50.0), &gradient_paint);
    }

    // 保存结果
    match canvas.save_png("output.png") {
        Ok(_) => println!("✅ 渲染完成！已保存到 output.png"),
        Err(e) => eprintln!("❌ 保存失败: {}", e),
    }
}
