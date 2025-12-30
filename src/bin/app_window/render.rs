//! 渲染相关逻辑

use std::num::NonZeroU32;
use mini_render::Canvas;

/// 将内容渲染到窗口缓冲区
pub fn present_to_buffer(
    buffer: &mut [u32],
    buffer_width: u32,
    buffer_height: u32,
    canvas: &Canvas,
    fixed_canvas: Option<&Canvas>,
    tabbar_canvas: Option<&Canvas>,
    scroll_offset: i32,
    has_tabbar: bool,
    tabbar_physical_height: u32,
) {
    let canvas_data = canvas.to_rgba();
    let canvas_width = canvas.width();
    let canvas_height = canvas.height();
    
    let content_area_height = buffer_height - if has_tabbar { tabbar_physical_height } else { 0 };
    
    // 渲染内容区域
    for y in 0..content_area_height {
        let src_y = (y as i32 + scroll_offset).clamp(0, canvas_height as i32 - 1) as u32;
        for x in 0..buffer_width.min(canvas_width) {
            let src_idx = ((src_y * canvas_width + x) * 4) as usize;
            let dst_idx = (y * buffer_width + x) as usize;
            if src_idx + 3 < canvas_data.len() && dst_idx < buffer.len() {
                let r = canvas_data[src_idx] as u32;
                let g = canvas_data[src_idx + 1] as u32;
                let b = canvas_data[src_idx + 2] as u32;
                buffer[dst_idx] = (r << 16) | (g << 8) | b;
            }
        }
    }
    
    // 渲染 fixed 元素（覆盖在内容区域上）
    if let Some(fixed_canvas) = fixed_canvas {
        let fixed_data = fixed_canvas.to_rgba();
        let fixed_width = fixed_canvas.width();
        let fixed_height = fixed_canvas.height();
        for y in 0..content_area_height.min(fixed_height) {
            for x in 0..buffer_width.min(fixed_width) {
                let src_idx = ((y * fixed_width + x) * 4) as usize;
                let dst_idx = (y * buffer_width + x) as usize;
                if src_idx + 3 < fixed_data.len() && dst_idx < buffer.len() {
                    let a = fixed_data[src_idx + 3];
                    if a > 0 {
                        let r = fixed_data[src_idx] as u32;
                        let g = fixed_data[src_idx + 1] as u32;
                        let b = fixed_data[src_idx + 2] as u32;
                        if a == 255 {
                            buffer[dst_idx] = (r << 16) | (g << 8) | b;
                        } else {
                            // Alpha 混合
                            let dst = buffer[dst_idx];
                            let dst_r = (dst >> 16) & 0xFF;
                            let dst_g = (dst >> 8) & 0xFF;
                            let dst_b = dst & 0xFF;
                            let alpha = a as u32;
                            let inv_alpha = 255 - alpha;
                            let new_r = (r * alpha + dst_r * inv_alpha) / 255;
                            let new_g = (g * alpha + dst_g * inv_alpha) / 255;
                            let new_b = (b * alpha + dst_b * inv_alpha) / 255;
                            buffer[dst_idx] = (new_r << 16) | (new_g << 8) | new_b;
                        }
                    }
                }
            }
        }
    }
    
    // 渲染 TabBar
    if has_tabbar {
        if let Some(tabbar_canvas) = tabbar_canvas {
            let tabbar_data = tabbar_canvas.to_rgba();
            let tabbar_width = tabbar_canvas.width();
            let tabbar_height = tabbar_canvas.height();
            for y in 0..tabbar_physical_height.min(tabbar_height) {
                let dst_y = content_area_height + y;
                for x in 0..buffer_width.min(tabbar_width) {
                    let src_idx = ((y * tabbar_width + x) * 4) as usize;
                    let dst_idx = (dst_y * buffer_width + x) as usize;
                    if src_idx + 3 < tabbar_data.len() && dst_idx < buffer.len() {
                        let r = tabbar_data[src_idx] as u32;
                        let g = tabbar_data[src_idx + 1] as u32;
                        let b = tabbar_data[src_idx + 2] as u32;
                        buffer[dst_idx] = (r << 16) | (g << 8) | b;
                    }
                }
            }
        }
    }
}
