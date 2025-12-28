/**
 * Mini Render Engine - C 示例
 * 演示如何从 C 调用渲染引擎
 */

#include <stdio.h>
#include "../include/mini_render.h"

int main() {
    // 创建 400x300 画布
    Canvas* canvas = mr_canvas_new(400, 300);
    if (!canvas) {
        fprintf(stderr, "Failed to create canvas\n");
        return 1;
    }

    // 清空为白色
    mr_canvas_clear(canvas, 255, 255, 255, 255);

    // 绘制蓝色填充矩形
    mr_canvas_draw_rect(canvas, 20, 20, 100, 80, 
                        0x4A, 0x90, 0xD9, 255,  // 蓝色
                        MR_STYLE_FILL, 0);

    // 绘制红色描边矩形
    mr_canvas_draw_rect(canvas, 140, 20, 100, 80,
                        0xE7, 0x4C, 0x3C, 255,  // 红色
                        MR_STYLE_STROKE, 3.0f);

    // 绘制绿色填充圆形
    mr_canvas_draw_circle(canvas, 320, 60, 40,
                          0x2E, 0xCC, 0x71, 255,  // 绿色
                          MR_STYLE_FILL, 0);

    // 绘制线段
    mr_canvas_draw_line(canvas, 20, 150, 380, 150,
                        0xF3, 0x9C, 0x12, 255,  // 橙色
                        2.0f);

    // 创建并绘制圆角矩形路径
    Path* path = mr_path_new();
    mr_path_add_round_rect(path, 50, 180, 150, 80, 15);
    mr_canvas_draw_path(canvas, path,
                        0x9B, 0x59, 0xB6, 255,  // 紫色
                        MR_STYLE_FILL, 0);
    mr_path_free(path);

    // 创建并绘制椭圆
    Path* oval = mr_path_new();
    mr_path_add_oval(oval, 300, 220, 60, 40);
    mr_canvas_draw_path(canvas, oval,
                        0x1A, 0xBC, 0x9C, 255,  // 青色
                        MR_STYLE_FILL, 0);
    mr_path_free(oval);

    // 保存结果
    if (mr_canvas_save_png(canvas, "c_output.png")) {
        printf("✅ C 示例渲染完成！已保存到 c_output.png\n");
    } else {
        fprintf(stderr, "❌ 保存失败\n");
    }

    // 清理
    mr_canvas_free(canvas);

    return 0;
}
