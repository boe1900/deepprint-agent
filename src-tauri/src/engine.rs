use skia_safe::{
    pdf,
    Color, Font, FontMgr, FontStyle, Paint, Rect,
    TextBlob,
};
// 引入二维码库
use qrcode::QrCode;

pub struct Engine;

impl Engine {
    pub fn new() -> Self {
        Engine
    }

    fn mm_to_pt(mm: f32) -> f32 {
        mm * 2.83465
    }

    /// 辅助函数：绘制二维码
    /// canvas: 绘图画布
    /// text: 二维码内容
    /// x, y: 左上角坐标 (points)
    /// size: 二维码边长 (points)
    fn draw_qr_code(&self, canvas: &skia_safe::Canvas, text: &str, x: f32, y: f32, size: f32) {
        // 1. 生成二维码数据
        let code = match QrCode::new(text) {
            Ok(c) => c,
            Err(_) => return, // 如果内容太长无法生成，直接忽略
        };

        // 2. 获取二维码的矩阵数据
        // 这是一串 true/false，true 代表黑色块
        let qr_data = code.to_colors();
        let width = code.width(); // 矩阵的行列数 (例如 21x21)

        // 3. 计算每个小方块(Module)的大小
        let module_size = size / width as f32;

        let mut paint = Paint::default();
        paint.set_color(Color::BLACK);
        paint.set_anti_alias(false); // 二维码不需要抗锯齿，要锐利

        // 4. 遍历矩阵画方块
        for row in 0..width {
            for col in 0..width {
                // qrcode 库展平了数组，所以用 row * width + col 访问
                if let qrcode::Color::Dark = qr_data[row * width + col] {
                    let rect = Rect::from_xywh(
                        x + col as f32 * module_size,
                        y + row as f32 * module_size,
                        module_size,
                        module_size
                    );
                    canvas.draw_rect(rect, &paint);
                }
            }
        }
    }

    pub fn generate_pdf(&self, text: &str, width_mm: Option<f32>, height_mm: Option<f32>) -> Vec<u8> {
        let default_w = 100.0; // 默认改为常见标签尺寸 100x60mm 方便测试
        let default_h = 60.0;

        let w_mm = width_mm.unwrap_or(default_w);
        let h_mm = height_mm.unwrap_or(default_h);

        let page_width = Self::mm_to_pt(w_mm);
        let page_height = Self::mm_to_pt(h_mm);

        let mut document_buffer = Vec::new();

        {
            let document = pdf::new_document(&mut document_buffer, None);
            let mut on_page_doc = document.begin_page((page_width, page_height), None);
            let canvas = on_page_doc.canvas();

            // --- 绘图逻辑 ---
            let font_mgr = FontMgr::new();
            let typeface = font_mgr
                .match_family_style("Arial", FontStyle::normal())
                .or_else(|| font_mgr.match_family_style("Helvetica", FontStyle::normal()))
                .unwrap_or_else(|| {
                    font_mgr
                        .match_family_style("", FontStyle::normal())
                        .expect("No fonts found")
                });

            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            paint.set_color(Color::BLACK);

            // 布局参数
            let margin = 10.0;
            let qr_size = page_height - (margin * 2.0); // 让二维码高度占满（减去边距）
            
            // 1. 绘制左侧文字
            let title_font = Font::new(typeface.clone(), 18.0);
            if let Some(blob) = TextBlob::from_str("Asset Tag", &title_font) {
                canvas.draw_text_blob(&blob, (margin, margin + 20.0), &paint);
            }

            let content_font = Font::new(typeface, 12.0);
            // 简单的多行模拟
            let lines = vec![
                format!("ID: {}", text),
                format!("Date: 2025-12-17"),
                "Dept: Engineering".to_string(),
            ];

            for (i, line) in lines.iter().enumerate() {
                if let Some(blob) = TextBlob::from_str(line, &content_font) {
                    canvas.draw_text_blob(&blob, (margin, margin + 50.0 + (i as f32 * 16.0)), &paint);
                }
            }

            // 2. 绘制右侧二维码
            // x 坐标放在靠右的位置
            let qr_x = page_width - qr_size - margin;
            let qr_y = margin;
            
            self.draw_qr_code(canvas, text, qr_x, qr_y, qr_size);

            // 3. 绘制外框
            let rect = Rect::from_xywh(2.0, 2.0, page_width - 4.0, page_height - 4.0);
            paint.set_style(skia_safe::paint::Style::Stroke);
            paint.set_stroke_width(2.0);
            paint.set_color(Color::BLACK);
            canvas.draw_rect(rect, &paint);

            // --- 结束 ---
            let document = on_page_doc.end_page();
            document.close();
        } 

        document_buffer
    }
}