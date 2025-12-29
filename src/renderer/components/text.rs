//! text 组件 - 文本显示

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle};
use taffy::prelude::*;

pub struct TextComponent;

impl TextComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        let text_content = get_text_content(node);
        if text_content.is_empty() { return None; }
        
        let sf = ctx.scale_factor;
        let font_size = ns.font_size * sf;
        let line_height = ns.line_height.map(|lh| lh * sf).unwrap_or(font_size * 1.5);
        
        // 估算文本宽度
        // 中文字符约等于 font_size，ASCII 字符约 0.5 * font_size
        let estimated_width: f32 = text_content.chars().map(|c| {
            if c == '\n' {
                0.0 // 换行符不占宽度
            } else if c.is_ascii() {
                font_size * 0.5
            } else {
                font_size // 中文等全角字符
            }
        }).sum();
        
        // 设置 flex-shrink 允许收缩
        ts.flex_shrink = 1.0;
        
        // 设置最小高度为一行
        ts.min_size.height = length(line_height);
        
        // 如果没有设置宽度，使用估算的文本宽度
        if matches!(ts.size.width, Dimension::Auto) {
            ts.size.width = length(estimated_width);
        }
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "text".into(),
            text: text_content,
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
        text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        sf: f32
    ) {
        let color = node.style.text_color.unwrap_or(Color::BLACK);
        let size = node.style.font_size * sf;
        let line_height = node.style.line_height.map(|lh| lh * sf).unwrap_or(size * 1.5);
        let letter_spacing = node.style.letter_spacing * sf;
        
        if let Some(tr) = text_renderer {
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            
            // 处理 white-space: nowrap 和 text-overflow: ellipsis
            let should_wrap = !matches!(node.style.white_space, WhiteSpace::NoWrap | WhiteSpace::Pre);
            let use_ellipsis = matches!(node.style.text_overflow, TextOverflow::Ellipsis);
            
            let text = &node.text;
            
            if w > 0.0 && should_wrap {
                // 自动换行绘制
                draw_text_wrapped_advanced(
                    canvas, tr, text, x, y + size, size, w, h,
                    line_height, letter_spacing, &node.style, &paint
                );
            } else if w > 0.0 && use_ellipsis {
                // 单行 + 省略号
                draw_text_with_ellipsis(canvas, tr, text, x, y + size, size, w, letter_spacing, &paint);
            } else {
                // 普通绘制
                tr.draw_text_with_spacing(canvas, text, x, y + size, size, letter_spacing, &paint);
            }
            
            // 绘制文本装饰
            if node.style.text_decoration != TextDecoration::None {
                draw_text_decoration(canvas, &node.style, x, y, w, size, line_height, &paint);
            }
        }
    }
}

/// 高级换行绘制（支持 line-height, letter-spacing）
fn draw_text_wrapped_advanced(
    canvas: &mut Canvas,
    tr: &TextRenderer,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    max_width: f32,
    max_height: f32,
    line_height: f32,
    letter_spacing: f32,
    style: &NodeStyle,
    paint: &Paint,
) {
    if max_width <= 0.0 {
        tr.draw_text_with_spacing(canvas, text, x, y, size, letter_spacing, paint);
        return;
    }
    
    let mut current_y = y;
    let mut line_start = 0;
    let chars: Vec<char> = text.chars().collect();
    let mut current_width = 0.0;
    let use_ellipsis = matches!(style.text_overflow, TextOverflow::Ellipsis);
    
    for (i, ch) in chars.iter().enumerate() {
        let char_width = tr.measure_char(*ch, size) + letter_spacing;
        
        // 检查是否需要换行
        if current_width + char_width > max_width && i > line_start {
            // 检查是否超出高度
            if max_height > 0.0 && current_y + line_height > y + max_height - size {
                // 最后一行，需要省略号
                if use_ellipsis {
                    let line: String = chars[line_start..i].iter().collect();
                    draw_text_with_ellipsis(canvas, tr, &line, x, current_y, size, max_width, letter_spacing, paint);
                }
                return;
            }
            
            // 绘制当前行
            let line: String = chars[line_start..i].iter().collect();
            tr.draw_text_with_spacing(canvas, &line, x, current_y, size, letter_spacing, paint);
            
            // 移动到下一行
            current_y += line_height;
            line_start = i;
            current_width = char_width;
        } else {
            current_width += char_width;
        }
    }
    
    // 绘制最后一行
    if line_start < chars.len() {
        let line: String = chars[line_start..].iter().collect();
        tr.draw_text_with_spacing(canvas, &line, x, current_y, size, letter_spacing, paint);
    }
}

/// 绘制带省略号的文本
fn draw_text_with_ellipsis(
    canvas: &mut Canvas,
    tr: &TextRenderer,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    max_width: f32,
    letter_spacing: f32,
    paint: &Paint,
) {
    let ellipsis = "...";
    let ellipsis_width = tr.measure_text_with_spacing(ellipsis, size, letter_spacing);
    
    let text_width = tr.measure_text_with_spacing(text, size, letter_spacing);
    if text_width <= max_width {
        tr.draw_text_with_spacing(canvas, text, x, y, size, letter_spacing, paint);
        return;
    }
    
    // 找到合适的截断点
    let chars: Vec<char> = text.chars().collect();
    let mut current_width = 0.0;
    let mut truncate_at = chars.len();
    
    for (i, ch) in chars.iter().enumerate() {
        let char_width = tr.measure_char(*ch, size) + letter_spacing;
        if current_width + char_width + ellipsis_width > max_width {
            truncate_at = i;
            break;
        }
        current_width += char_width;
    }
    
    let truncated: String = chars[..truncate_at].iter().collect();
    let display_text = format!("{}{}", truncated, ellipsis);
    tr.draw_text_with_spacing(canvas, &display_text, x, y, size, letter_spacing, paint);
}

/// 绘制文本装饰（下划线、删除线等）
fn draw_text_decoration(
    canvas: &mut Canvas,
    style: &NodeStyle,
    x: f32,
    y: f32,
    w: f32,
    size: f32,
    _line_height: f32,
    paint: &Paint,
) {
    let line_y = match style.text_decoration {
        TextDecoration::Underline => y + size + 2.0,
        TextDecoration::LineThrough => y + size * 0.6,
        TextDecoration::Overline => y,
        TextDecoration::None => return,
    };
    
    // 绘制装饰线
    let mut line_paint = paint.clone();
    line_paint.style = PaintStyle::Stroke;
    
    use crate::path::Path;
    let mut path = Path::new();
    path.move_to(x, line_y);
    path.line_to(x + w, line_y);
    canvas.draw_path(&path, &line_paint);
}
