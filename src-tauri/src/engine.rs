use skia_safe::{
    pdf,
    Color, Font, FontMgr, FontStyle, Paint, Rect,
    TextBlob,
};

/// DeepPrint 渲染引擎
pub struct Engine;

impl Engine {
    pub fn new() -> Self {
        Engine
    }

    /// 生成 PDF 文件并返回二进制数据
    /// 修复点：参考 skia-org 示例，使用 Vec<u8> 作为流，避免了 import 错误
    pub fn generate_pdf(&self, text: &str) -> Vec<u8> {
        // 1. 创建一个标准的 Rust Vec 来接收 PDF 数据
        // skia-safe 实现了 std::io::Write trait，所以可以直接写进 Vec
        let mut document_buffer = Vec::new();

        // 2. 开启一个作用域，确保 document 对 buffer 的借用在函数结束前释放
        {
            // 创建 PDF 文档，将 buffer 的可变引用传进去
            // 修复: document 本身不需要是 mut 的，因为它只用来调用了一次 begin_page
            let document = pdf::new_document(&mut document_buffer, None);

            // 3. 开始新页面 (Document -> Document<OnPage>)
            // begin_page 会消耗旧的 document 变量，返回一个新的页面对象
            let mut on_page_doc = document.begin_page((595.0, 842.0), None);

            // 4. 获取 Canvas 进行绘图
            let canvas = on_page_doc.canvas();

            // --- 绘图逻辑开始 ---
            
            // 字体管理
            let font_mgr = FontMgr::new();
            let typeface = font_mgr
                .match_family_style("Arial", FontStyle::normal())
                .or_else(|| font_mgr.match_family_style("Helvetica", FontStyle::normal()))
                .unwrap_or_else(|| {
                    font_mgr
                        .match_family_style("", FontStyle::normal())
                        .expect("系统未找到可用字体")
                });

            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            paint.set_color(Color::BLACK);

            // 绘制标题
            let title_font = Font::new(typeface.clone(), 24.0);
            if let Some(blob) = TextBlob::from_str("DeepPrint Native Engine", &title_font) {
                canvas.draw_text_blob(&blob, (50.0, 50.0), &paint);
            }

            // 绘制内容
            let content_font = Font::new(typeface, 14.0);
            if let Some(blob) = TextBlob::from_str(&format!("Content: {}", text), &content_font) {
                canvas.draw_text_blob(&blob, (50.0, 100.0), &paint);
            }

            // 绘制矩形
            let rect = Rect::from_xywh(50.0, 120.0, 500.0, 200.0);
            paint.set_style(skia_safe::paint::Style::Stroke);
            paint.set_stroke_width(2.0);
            paint.set_color(Color::RED);
            canvas.draw_rect(rect, &paint);

            // --- 绘图逻辑结束 ---

            // 5. 结束页面 (Document<OnPage> -> Document<OffPage>)
            // 必须显式调用 end_page 来完成当前页的写入
            let document = on_page_doc.end_page();

            // 6. 关闭文档流
            document.close();
        } 
        // 作用域结束，document 被销毁，document_buffer 的借用被释放

        // 7. 直接返回 Vec<u8>，这比 skia_safe::Data 更通用
        document_buffer
    }
}