//! image 组件 - 图片
//! 
//! 属性：
//! - src: 图片资源地址（支持网络URL和本地路径）
//! - mode: 图片裁剪、缩放模式
//!   - scaleToFill: 缩放模式，不保持纵横比缩放图片
//!   - aspectFit: 缩放模式，保持纵横比缩放图片，完整显示
//!   - aspectFill: 缩放模式，保持纵横比缩放图片，只保证短边完全显示
//!   - widthFix: 缩放模式，宽度不变，高度自动变化
//!   - heightFix: 缩放模式，高度不变，宽度自动变化
//! - lazy-load: 懒加载
//! - show-menu-by-longpress: 长按显示菜单

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use taffy::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::io::Read;

/// 图片缓存数据
struct ImageData {
    data: Vec<u8>,  // RGBA 数据
    width: u32,
    height: u32,
}

/// 全局图片缓存
static IMAGE_CACHE: OnceLock<Arc<Mutex<HashMap<String, Option<ImageData>>>>> = OnceLock::new();

fn get_image_cache() -> &'static Arc<Mutex<HashMap<String, Option<ImageData>>>> {
    IMAGE_CACHE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

/// 加载图片（支持网络URL和本地文件）
fn load_image(src: &str) -> Option<ImageData> {
    // 检查缓存
    {
        let cache = get_image_cache();
        let cache_guard = cache.lock().ok()?;
        if let Some(cached) = cache_guard.get(src) {
            return cached.as_ref().map(|d| ImageData {
                data: d.data.clone(),
                width: d.width,
                height: d.height,
            });
        }
    }

    let result = if src.starts_with("http://") || src.starts_with("https://") {
        load_image_from_url(src)
    } else {
        load_image_from_file(src)
    };

    // 存入缓存
    {
        let cache = get_image_cache();
        if let Ok(mut cache_guard) = cache.lock() {
            cache_guard.insert(src.to_string(), result.as_ref().map(|d| ImageData {
                data: d.data.clone(),
                width: d.width,
                height: d.height,
            }));
        }
    }

    result
}

/// 从网络URL加载图片
fn load_image_from_url(url: &str) -> Option<ImageData> {
    println!("🖼️ Loading image from URL: {}", url);
    
    // 使用 ureq 下载图片
    let response = match ureq::get(url)
        .timeout(std::time::Duration::from_secs(10))
        .call() {
            Ok(r) => r,
            Err(e) => {
                println!("❌ Failed to fetch image: {}", e);
                return None;
            }
        };
    
    let mut bytes = Vec::new();
    if let Err(e) = response.into_reader().take(10 * 1024 * 1024).read_to_end(&mut bytes) {
        println!("❌ Failed to read image data: {}", e);
        return None;
    }
    
    println!("📦 Downloaded {} bytes", bytes.len());
    
    match decode_image_bytes(&bytes) {
        Some(img) => {
            println!("✅ Image decoded: {}x{}", img.width, img.height);
            Some(img)
        }
        None => {
            println!("❌ Failed to decode image");
            None
        }
    }
}

/// 从本地文件加载图片
fn load_image_from_file(path: &str) -> Option<ImageData> {
    // 尝试多个可能的路径
    let paths_to_try = vec![
        path.to_string(),
        format!("sample-app{}", path),
        format!("sample-app/{}", path.trim_start_matches('/')),
        format!("assets{}", path),
        format!("assets/{}", path.trim_start_matches('/')),
    ];

    for p in paths_to_try {
        if let Ok(bytes) = std::fs::read(&p) {
            if let Some(img) = decode_image_bytes(&bytes) {
                return Some(img);
            }
        }
    }
    None
}

/// 解码图片字节数据
fn decode_image_bytes(bytes: &[u8]) -> Option<ImageData> {
    use image::GenericImageView;
    
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = img.dimensions();
    
    Some(ImageData {
        data: rgba.into_raw(),
        width,
        height,
    })
}

pub struct ImageComponent;

impl ImageComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let src = node.get_attr("src").unwrap_or("");
        let mode = node.get_attr("mode").unwrap_or("scaleToFill");
        
        // 默认图片大小 150x100
        let default_width = 150.0;
        let default_height = 100.0;
        
        if ts.size.width == Dimension::Auto {
            ts.size.width = length(default_width * sf);
        }
        if ts.size.height == Dimension::Auto {
            ts.size.height = length(default_height * sf);
        }
        
        // 图片占位符背景
        ns.background_color = Some(Color::from_hex(0xF5F5F5));
        ns.border_radius = 4.0 * sf;
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        // 存储 src 和 mode 到 text 字段（用 | 分隔）
        let text_data = format!("{}|{}", src, mode);
        
        Some(RenderNode {
            tag: "image".into(),
            text: text_data,
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        _text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        _sf: f32
    ) {
        let style = &node.style;
        let radius = style.border_radius;
        
        // 解析 src 和 mode
        let parts: Vec<&str> = node.text.split('|').collect();
        let src = parts.get(0).unwrap_or(&"");
        let mode = parts.get(1).unwrap_or(&"scaleToFill");
        
        // 绘制背景
        let bg_color = style.background_color.unwrap_or(Color::from_hex(0xF5F5F5));
        let bg_paint = Paint::new()
            .with_color(bg_color)
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        
        if radius > 0.0 {
            let mut path = Path::new();
            path.add_round_rect(x, y, w, h, radius);
            canvas.draw_path(&path, &bg_paint);
        } else {
            canvas.draw_rect(&GeoRect::new(x, y, w, h), &bg_paint);
        }
        
        // 尝试加载并绘制图片
        if !src.is_empty() {
            if let Some(img_data) = load_image(src) {
                canvas.draw_image(
                    &img_data.data,
                    img_data.width,
                    img_data.height,
                    x, y, w, h,
                    mode,
                    radius,
                );
                
                // 绘制边框
                if style.border_width > 0.0 {
                    if let Some(bc) = style.border_color {
                        let border_paint = Paint::new()
                            .with_color(bc)
                            .with_style(PaintStyle::Stroke)
                            .with_anti_alias(true);
                        if radius > 0.0 {
                            let mut path = Path::new();
                            path.add_round_rect(x, y, w, h, radius);
                            canvas.draw_path(&path, &border_paint);
                        } else {
                            canvas.draw_rect(&GeoRect::new(x, y, w, h), &border_paint);
                        }
                    }
                }
                return;
            }
        }
        
        // 如果图片加载失败，绘制占位符
        Self::draw_placeholder(canvas, x, y, w, h, radius, style);
    }
    
    /// 绘制图片占位符（山形+太阳图标）
    fn draw_placeholder(
        canvas: &mut Canvas,
        x: f32, y: f32, w: f32, h: f32,
        radius: f32,
        style: &NodeStyle,
    ) {
        let icon_size = w.min(h) * 0.35;
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        
        let icon_color = Color::from_hex(0xCCCCCC);
        let icon_paint = Paint::new()
            .with_color(icon_color)
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        
        // 太阳（圆形）
        let sun_x = cx - icon_size * 0.25;
        let sun_y = cy - icon_size * 0.3;
        let sun_r = icon_size * 0.15;
        canvas.draw_circle(sun_x, sun_y, sun_r, &icon_paint);
        
        // 山形（三角形）
        let mut mountain = Path::new();
        mountain.move_to(cx - icon_size * 0.5, cy + icon_size * 0.35);
        mountain.line_to(cx - icon_size * 0.15, cy - icon_size * 0.05);
        mountain.line_to(cx + icon_size * 0.1, cy + icon_size * 0.35);
        mountain.close();
        canvas.draw_path(&mountain, &icon_paint);
        
        // 右边大山
        let mut mountain2 = Path::new();
        mountain2.move_to(cx - icon_size * 0.1, cy + icon_size * 0.35);
        mountain2.line_to(cx + icon_size * 0.25, cy - icon_size * 0.25);
        mountain2.line_to(cx + icon_size * 0.55, cy + icon_size * 0.35);
        mountain2.close();
        canvas.draw_path(&mountain2, &icon_paint);
        
        // 绘制边框
        if style.border_width > 0.0 {
            if let Some(bc) = style.border_color {
                let border_paint = Paint::new()
                    .with_color(bc)
                    .with_style(PaintStyle::Stroke)
                    .with_anti_alias(true);
                if radius > 0.0 {
                    let mut path = Path::new();
                    path.add_round_rect(x, y, w, h, radius);
                    canvas.draw_path(&path, &border_paint);
                } else {
                    canvas.draw_rect(&GeoRect::new(x, y, w, h), &border_paint);
                }
            }
        }
    }
}
