/**
 * Mini Render Engine - C API
 * 类似 Skia 的轻量级渲染引擎
 */

#ifndef MINI_RENDER_H
#define MINI_RENDER_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// 画布句柄
typedef struct Canvas Canvas;

// 路径句柄
typedef struct Path Path;

// 画笔样式
typedef enum {
    MR_STYLE_FILL = 0,
    MR_STYLE_STROKE = 1,
    MR_STYLE_FILL_AND_STROKE = 2
} MRPaintStyle;

// ============ Canvas API ============

// 创建画布
Canvas* mr_canvas_new(uint32_t width, uint32_t height);

// 销毁画布
void mr_canvas_free(Canvas* canvas);

// 清空画布
void mr_canvas_clear(Canvas* canvas, uint8_t r, uint8_t g, uint8_t b, uint8_t a);

// 获取画布尺寸
uint32_t mr_canvas_width(const Canvas* canvas);
uint32_t mr_canvas_height(const Canvas* canvas);

// 绘制矩形
void mr_canvas_draw_rect(
    Canvas* canvas,
    float x, float y, float width, float height,
    uint8_t r, uint8_t g, uint8_t b, uint8_t a,
    uint8_t style, float stroke_width
);

// 绘制圆形
void mr_canvas_draw_circle(
    Canvas* canvas,
    float cx, float cy, float radius,
    uint8_t r, uint8_t g, uint8_t b, uint8_t a,
    uint8_t style, float stroke_width
);

// 绘制线段
void mr_canvas_draw_line(
    Canvas* canvas,
    float x0, float y0, float x1, float y1,
    uint8_t r, uint8_t g, uint8_t b, uint8_t a,
    float stroke_width
);

// 绘制路径
void mr_canvas_draw_path(
    Canvas* canvas,
    const Path* path,
    uint8_t r, uint8_t g, uint8_t b, uint8_t a,
    uint8_t style, float stroke_width
);

// 获取像素数据
size_t mr_canvas_get_pixels(const Canvas* canvas, uint8_t* out, size_t len);

// 保存为 PNG
bool mr_canvas_save_png(const Canvas* canvas, const char* path);

// ============ Path API ============

// 创建路径
Path* mr_path_new(void);

// 销毁路径
void mr_path_free(Path* path);

// 移动到
void mr_path_move_to(Path* path, float x, float y);

// 画线到
void mr_path_line_to(Path* path, float x, float y);

// 二次贝塞尔曲线
void mr_path_quad_to(Path* path, float cx, float cy, float x, float y);

// 三次贝塞尔曲线
void mr_path_cubic_to(Path* path, float c1x, float c1y, float c2x, float c2y, float x, float y);

// 闭合路径
void mr_path_close(Path* path);

// 添加圆角矩形
void mr_path_add_round_rect(Path* path, float x, float y, float w, float h, float radius);

// 添加椭圆
void mr_path_add_oval(Path* path, float cx, float cy, float rx, float ry);

#ifdef __cplusplus
}
#endif

#endif // MINI_RENDER_H
