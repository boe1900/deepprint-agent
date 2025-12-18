mod deep_print_schema;
mod renderer;

use crate::deep_print_schema::*;
use crate::renderer::DeepPrintRenderer;
use serde_json::json;
use skia_safe::{surfaces, Color, EncodedImageFormat};
use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // -------------------------------------------------------------------------
    // 1. 模拟模版 JSON
    // -------------------------------------------------------------------------
    let template_json = r##"{
        "meta": { "version": "6.1", "name": "DeepPrint 测试小票" },
        "dataSchema": "",
        "canvas": {
            "width": 380,
            "height": 0, 
            "orientation": 3,
            "styles": { "fontSize": 12, "fontColor": "#333333", "fontFamily": "Arial" },
            "elements": [
                {
                    "id": "header",
                    "type": "text",
                    "x": 0, "y": 20, "w": 380, "h": 40,
                    "content": "DeepPrint 智慧餐厅",
                    "fontSize": 24, "fontWeight": "bold", "textAlign": "center"
                },
                {
                    "id": "sub_header",
                    "type": "text",
                    "x": 0, "y": 0, "w": 380, "h": 20,
                    "linkedTo": "header",
                    "content": "-- 结账单 --",
                    "textAlign": "center", "fontColor": "#999999"
                },
                {
                    "id": "info_block",
                    "type": "text",
                    "x": 20, "y": 20, "w": 340, "h": 20,
                    "linkedTo": "sub_header",
                    "content": "单号: {{order.no}}\n时间: {{order.time}}\n收银员: {{order.cashier}}",
                    "fontSize": 10, "lineHeight": 1.5
                },
                {
                    "id": "line_1",
                    "type": "line",
                    "x": 20, "y": 15, "w": 340, "h": 2,
                    "linkedTo": "info_block",
                    "dashArray": [5, 5], 
                    "strokeColor": "#CCCCCC"
                },
                {
                    "id": "goods_table",
                    "type": "table",
                    "x": 20, "y": 10, "w": 340, "h": 0,
                    "linkedTo": "line_1",
                    "data": "{{order.items}}",
                    "cellPadding": 8,
                    "borderWidth": 0,
                    "columns": [
                        { "title": "菜品名称", "field": "name", "width": "50%" },
                        { "title": "数量", "field": "qty", "width": "20%", "textAlign": "center" },
                        { "title": "金额", "field": "amount", "width": "30%", "textAlign": "right" }
                    ]
                },
                {
                    "id": "line_2",
                    "type": "line",
                    "x": 20, "y": 10, "w": 340, "h": 2,
                    "linkedTo": "goods_table",
                    "strokeColor": "#000000", "strokeWidth": 2
                },
                {
                    "id": "total_row",
                    "type": "text",
                    "x": 20, "y": 15, "w": 340, "h": 30,
                    "linkedTo": "line_2",
                    "content": "合计金额:   ￥{{order.total}}",
                    "textAlign": "right", "fontSize": 16, "fontWeight": "bold"
                },
                {
                    "id": "qr_code",
                    "type": "qrcode",
                    "x": 130, "y": 30, "w": 120, "h": 120,
                    "linkedTo": "total_row",
                    "value": "https://deepprint.io/invoice/{{order.no}}",
                    "correctionLevel": "M"
                },
                {
                    "id": "footer",
                    "type": "text",
                    "x": 0, "y": 10, "w": 380, "h": 20,
                    "linkedTo": "qr_code",
                    "content": "扫码开具电子发票\n谢谢惠顾，欢迎下次光临",
                    "textAlign": "center", "fontSize": 10, "fontColor": "#999999"
                }
            ]
        }
    }"##;

    // 解析模版
    let template: DeepPrintTemplate = match serde_json::from_str(template_json) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("❌ JSON 解析失败: {}", e);
            return Ok(());
        }
    };
    println!("✅ 模版解析成功: {}", template.meta.name);

    // -------------------------------------------------------------------------
    // 2. 模拟真实数据
    // -------------------------------------------------------------------------
    let data = json!({
        "order": {
            "no": "DP-20231024-8888",
            "time": "2023-10-24 18:30:45",
            "cashier": "007号",
            "total": "216.00",
            "items": [
                { "name": "招牌香辣烤鱼", "qty": 1, "amount": "128.00" },
                { "name": "蒜蓉空心菜", "qty": 1, "amount": "28.00" },
                { "name": "鲜榨西瓜汁(扎)", "qty": 1, "amount": "48.00" },
                { "name": "米饭", "qty": 4, "amount": "12.00" }
            ]
        }
    });

    // -------------------------------------------------------------------------
    // 3. 准备画布 (Surface)
    // -------------------------------------------------------------------------
    let canvas_width = template.canvas.width as i32;
    let canvas_height = 800; 
    
    // 创建 Surface
    let mut surface = surfaces::raster_n32_premul((canvas_width, canvas_height))
        .expect("无法创建 Skia Surface");
    
    // 填充白色背景
    // 使用独立的作用域或在渲染前直接调用，避免长期借用
    surface.canvas().clear(Color::WHITE);

    // -------------------------------------------------------------------------
    // 4. 执行渲染
    // -------------------------------------------------------------------------
    let renderer = DeepPrintRenderer::new();
    println!("🚀 开始渲染...");
    
    // 直接传入 surface.canvas()，避免中间变量导致类型推断为不可变借用
    match renderer.render(surface.canvas(), &template, &data) {
        Ok(_) => println!("✅ 渲染完成！"),
        Err(e) => {
            eprintln!("❌ 渲染错误: {}", e);
            return Ok(());
        }
    }

    // -------------------------------------------------------------------------
    // 5. 保存结果到文件
    // -------------------------------------------------------------------------
    let image = surface.image_snapshot();
    let file_name = "output_receipt.png";
    
    if let Some(data) = image.encode(None, EncodedImageFormat::PNG, 100) {
        let mut file = File::create(file_name)?;
        file.write_all(data.as_bytes())?;
        println!("💾 结果已保存至: ./{}", file_name);
    } else {
        eprintln!("❌ 图像编码失败");
    }

    Ok(())
}