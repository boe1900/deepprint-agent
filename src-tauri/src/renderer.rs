use crate::deep_print_schema::*;
use qrcode::{EcLevel, QrCode};
use regex::{Captures, Regex};
use serde_json::Value;
use skia_safe::{
    textlayout::{
        FontCollection, ParagraphBuilder, ParagraphStyle, TextAlign, TextStyle,
    },
    Canvas, Color, Color4f, FontMgr, Paint, PaintStyle, PathEffect, Point, Rect,
};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

/// 渲染上下文，存储渲染过程中的中间状态
struct RenderContext<'a> {
    /// 原始数据
    data: &'a Value,
    /// 字体集合 (用于 textlayout)
    font_collection: FontCollection,
    /// 字体管理器 (用于查找系统字体)
    font_mgr: FontMgr,
    /// 已计算的元素布局 {id: (y, height)}
    layout_cache: HashMap<String, (f64, f64)>,
    /// 全局样式
    global_styles: &'a Option<GlobalStyles>,
}

pub struct DeepPrintRenderer {
    // 可以在这里持有全局资源，如图片缓存等
}

impl DeepPrintRenderer {
    pub fn new() -> Self {
        Self {}
    }

    /// 核心渲染入口
    pub fn render(
        &self,
        canvas: &Canvas,
        template: &DeepPrintTemplate,
        data: &Value,
    ) -> Result<(), String> {
        // 初始化字体管理器和集合
        let font_mgr = FontMgr::default();
        let mut font_collection = FontCollection::new();
        font_collection.set_default_font_manager(font_mgr.clone(), None);

        let mut ctx = RenderContext {
            data,
            font_collection,
            font_mgr,
            layout_cache: HashMap::new(),
            global_styles: &template.canvas.styles,
        };

        // 拓扑排序 (处理 linkedTo 依赖)
        let sorted_elements = self.topological_sort(&template.canvas.elements)?;

        // 逐个渲染元素
        for element in sorted_elements {
            self.render_element(canvas, element, &mut ctx)?;
        }

        Ok(())
    }

    /// 渲染单个元素 (分发器)
    fn render_element(
        &self,
        canvas: &Canvas,
        element: &Element,
        ctx: &mut RenderContext,
    ) -> Result<(), String> {
        // 计算 Y 坐标
        let (actual_y, _) = self.calculate_y(element, ctx);

        // 计算实际高度并绘制
        let actual_height = match &element.data {
            ElementData::Text(props) => self.draw_text(canvas, element, props, actual_y, ctx),
            ElementData::Table(props) => self.draw_table(canvas, element, props, actual_y, ctx),
            ElementData::Line(props) => self.draw_line(canvas, element, props, actual_y, ctx),
            ElementData::Rect(props) => self.draw_rect(canvas, element, props, actual_y, ctx),
            ElementData::Ellipse(props) => self.draw_ellipse(canvas, element, props, actual_y, ctx),
            ElementData::Image(props) => {
                self.draw_image_placeholder(canvas, element, props, actual_y, ctx)
            }
            ElementData::Barcode(props) => self.draw_barcode(canvas, element, props, actual_y, ctx),
            ElementData::Qrcode(props) => self.draw_qrcode(canvas, element, props, actual_y, ctx),
        }?;

        // 更新布局缓存
        ctx.layout_cache
            .insert(element.id.clone(), (actual_y, actual_height));

        Ok(())
    }

    // -------------------------------------------------------------------------
    // 组件绘制逻辑
    // -------------------------------------------------------------------------

    fn draw_text(
        &self,
        canvas: &Canvas,
        base: &Element,
        props: &TextProps,
        y: f64,
        ctx: &RenderContext,
    ) -> Result<f64, String> {
        let content = Interpolator::render(&props.content, ctx.data);
        if content.is_empty() && props.auto_height.unwrap_or(true) {
            return Ok(0.0);
        }

        // 获取样式配置
        let font_size = props
            .font_size
            .or(ctx
                .global_styles
                .as_ref()
                .and_then(|s| s.font_size))
            .unwrap_or(12.0);
        
        let color_hex = props
            .font_color
            .as_deref()
            .or(ctx
                .global_styles
                .as_ref()
                .and_then(|s| s.font_color.as_deref()))
            .unwrap_or("#000000");
        let color = parse_color(color_hex);

        let font_family = props
            .font_family
            .as_deref()
            .or(ctx
                .global_styles
                .as_ref()
                .and_then(|s| s.font_family.as_deref()));

        // 构建文本样式
        let mut text_style = TextStyle::new();
        text_style.set_font_size(font_size as f32);
        // FIXED: 使用 set_foreground_paint 替代 set_foreground_color，并将 Color 转换为 Color4f
        text_style.set_foreground_paint(&Paint::new(Color4f::from(color), None));
        
        if let Some(fam) = font_family {
            text_style.set_font_families(&[fam]);
        }

        // 处理 Font Weight (简单映射)
        // 注意: skia-safe 的 api 可能会变动，这里做最基础的处理
        if let Some(weight) = &props.font_weight {
             match weight {
                 FontWeight::String(s) if s.eq_ignore_ascii_case("bold") => {
                     // text_style.set_font_style(...) // 实际设置需配合 FontMgr
                 },
                 _ => {}
             }
        }

        // 构建段落样式
        let mut para_style = ParagraphStyle::new();
        if let Some(align) = &props.text_align {
            para_style.set_text_align(match align.as_str() {
                "center" => TextAlign::Center,
                "right" => TextAlign::Right,
                _ => TextAlign::Left,
            });
        }

        // 生成段落
        let mut builder = ParagraphBuilder::new(&para_style, &ctx.font_collection);
        builder.push_style(&text_style);
        builder.add_text(&content);
        let mut paragraph = builder.build();

        // 布局
        paragraph.layout(base.w as f32);
        let text_height = paragraph.height() as f64;

        // 计算绘制位置 (垂直对齐)
        let draw_y = if !props.auto_height.unwrap_or(true) && base.h > text_height {
            match props.vertical_align.as_deref() {
                Some("middle") => y + (base.h - text_height) / 2.0,
                Some("bottom") => y + (base.h - text_height),
                _ => y,
            }
        } else {
            y
        };

        paragraph.paint(canvas, Point::new(base.x as f32, draw_y as f32));

        if props.auto_height.unwrap_or(true) {
            Ok(text_height)
        } else {
            Ok(base.h)
        }
    }

    fn draw_table(
        &self,
        canvas: &Canvas,
        base: &Element,
        props: &TableProps,
        start_y: f64,
        ctx: &RenderContext,
    ) -> Result<f64, String> {
        let mut current_y = start_y;
        
        // 边框画笔
        let mut border_paint = Paint::default();
        border_paint.set_style(PaintStyle::Stroke);
        border_paint.set_stroke_width(props.border_width.unwrap_or(2.83) as f32);
        border_paint.set_color(parse_color(props.border_color.as_deref().unwrap_or("#000000")));

        let rows_data = Interpolator::get_array_by_path(ctx.data, &props.data)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let cell_padding = props.cell_padding.unwrap_or(5.0);

        // 计算列宽
        let total_width = base.w;
        let mut col_widths = Vec::new();
        let mut fixed_used = 0.0;
        
        for col in &props.columns {
            match &col.width {
                Some(TableColumnWidth::Fixed(w)) => {
                    col_widths.push(*w);
                    fixed_used += w;
                }
                Some(TableColumnWidth::Percentage(s)) => {
                    let p = s.trim_end_matches('%').parse::<f64>().unwrap_or(0.0);
                    col_widths.push(-p); // 负数标记
                }
                None => col_widths.push(0.0),
            }
        }

        let remaining = (total_width - fixed_used).max(0.0);
        let auto_cols_count = col_widths.iter().filter(|&&w| w == 0.0).count();
        
        for w in &mut col_widths {
            if *w < 0.0 {
                *w = remaining * (w.abs() / 100.0);
            } else if *w == 0.0 && auto_cols_count > 0 {
                *w = remaining / auto_cols_count as f64;
            }
        }

        // 绘制表头
        if props.show_head.unwrap_or(1) == 1 {
            let mut x_cursor = base.x;
            let mut max_h = 0.0;

            // 预计算高度
            for (i, col) in props.columns.iter().enumerate() {
                let h = self.measure_simple_text(&col.title, col_widths[i], ctx, true);
                if h > max_h { max_h = h; }
            }
            max_h += cell_padding * 2.0;

            // 绘制
            for (i, col) in props.columns.iter().enumerate() {
                let w = col_widths[i];
                let rect = Rect::from_xywh(x_cursor as f32, current_y as f32, w as f32, max_h as f32);
                
                // 只有当线宽大于0时才绘制边框
                if border_paint.stroke_width() > 0.0 {
                    canvas.draw_rect(rect, &border_paint);
                }
                
                self.draw_cell_text(canvas, &col.title, rect, cell_padding, ctx, true, col.text_align.as_deref());
                x_cursor += w;
            }
            current_y += max_h;
        }

        // 绘制数据行
        for row in rows_data {
            let mut x_cursor = base.x;
            let mut row_height = 0.0;
            let mut cell_texts = Vec::new();

            // 预计算行高
            for (i, col) in props.columns.iter().enumerate() {
                let text = Interpolator::get_value_from_obj(row, &col.field);
                let h = self.measure_simple_text(&text, col_widths[i], ctx, false);
                if h > row_height { row_height = h; }
                cell_texts.push(text);
            }
            row_height += cell_padding * 2.0;

            // 绘制
            for (i, text) in cell_texts.iter().enumerate() {
                let w = col_widths[i];
                let rect = Rect::from_xywh(x_cursor as f32, current_y as f32, w as f32, row_height as f32);
                
                if border_paint.stroke_width() > 0.0 {
                    canvas.draw_rect(rect, &border_paint);
                }

                self.draw_cell_text(canvas, text, rect, cell_padding, ctx, false, props.columns[i].text_align.as_deref());
                x_cursor += w;
            }
            current_y += row_height;
        }

        Ok(current_y - start_y)
    }

    // 辅助: 简单文本测量 (用于表格)
    fn measure_simple_text(&self, text: &str, width: f64, ctx: &RenderContext, _bold: bool) -> f64 {
        let mut ts = TextStyle::new();
        ts.set_font_size(10.0);
        let mut builder = ParagraphBuilder::new(&ParagraphStyle::new(), &ctx.font_collection);
        builder.push_style(&ts);
        builder.add_text(text);
        let mut p = builder.build();
        p.layout(width as f32);
        p.height() as f64
    }

    // 辅助: 绘制单元格文字
    fn draw_cell_text(&self, canvas: &Canvas, text: &str, rect: Rect, padding: f64, ctx: &RenderContext, _bold: bool, align: Option<&str>) {
        let mut ts = TextStyle::new();
        ts.set_font_size(10.0);
        // FIXED: 使用 set_foreground_paint 替代 set_foreground_color，并将 Color 转换为 Color4f
        ts.set_foreground_paint(&Paint::new(Color4f::from(Color::BLACK), None));

        let mut ps = ParagraphStyle::new();
        if let Some(a) = align {
            ps.set_text_align(match a {
                "center" => TextAlign::Center,
                "right" => TextAlign::Right,
                _ => TextAlign::Left,
            });
        }

        let mut builder = ParagraphBuilder::new(&ps, &ctx.font_collection);
        builder.push_style(&ts);
        builder.add_text(text);
        let mut p = builder.build();
        
        // 考虑 padding 后的可用宽度
        let avail_w = (rect.width() - (padding * 2.0) as f32).max(0.0);
        p.layout(avail_w);

        // 垂直居中
        let text_h = p.height();
        let y = rect.top() + (rect.height() - text_h) / 2.0;
        
        p.paint(canvas, Point::new(rect.left() + padding as f32, y));
    }

    fn draw_line(&self, canvas: &Canvas, base: &Element, props: &LineProps, y: f64, _ctx: &RenderContext) -> Result<f64, String> {
        let mut p = Paint::default();
        p.set_style(PaintStyle::Stroke);
        p.set_stroke_width(props.stroke_width.unwrap_or(2.83) as f32);
        p.set_color(parse_color(props.stroke_color.as_deref().unwrap_or("#000000")));
        
        // 处理虚线
        if let Some(dash) = &props.dash_array {
            let intervals: Vec<f32> = dash.iter().map(|&x| x as f32).collect();
            // FIXED: Use PathEffect::dash instead of skia_safe::path_effect::dash
            p.set_path_effect(PathEffect::dash(&intervals, 0.0));
        }

        canvas.draw_line(
            Point::new(base.x as f32, y as f32),
            Point::new((base.x + base.w) as f32, (y + base.h) as f32),
            &p
        );
        Ok(base.h)
    }

    fn draw_rect(&self, canvas: &Canvas, base: &Element, props: &RectProps, y: f64, _ctx: &RenderContext) -> Result<f64, String> {
        let rect = Rect::from_xywh(base.x as f32, y as f32, base.w as f32, base.h as f32);
        
        if let Some(fill) = &props.fill_color {
            if !fill.is_empty() {
                let mut p = Paint::default();
                p.set_style(PaintStyle::Fill);
                p.set_color(parse_color(fill));
                canvas.draw_rect(rect, &p);
            }
        }

        let stroke_w = props.stroke_width.unwrap_or(2.83);
        if stroke_w > 0.0 {
            let mut p = Paint::default();
            p.set_style(PaintStyle::Stroke);
            p.set_stroke_width(stroke_w as f32);
            p.set_color(parse_color(props.stroke_color.as_deref().unwrap_or("#000000")));
            
            if let Some(dash) = &props.dash_array {
                let intervals: Vec<f32> = dash.iter().map(|&x| x as f32).collect();
                // FIXED: Use PathEffect::dash instead of skia_safe::path_effect::dash
                p.set_path_effect(PathEffect::dash(&intervals, 0.0));
            }
            
            canvas.draw_rect(rect, &p);
        }
        Ok(base.h)
    }

    fn draw_ellipse(&self, canvas: &Canvas, base: &Element, props: &EllipseProps, y: f64, _ctx: &RenderContext) -> Result<f64, String> {
        let rect = Rect::from_xywh(base.x as f32, y as f32, base.w as f32, base.h as f32);
        let mut p = Paint::default();
        p.set_style(PaintStyle::Stroke);
        p.set_stroke_width(props.stroke_width.unwrap_or(2.83) as f32);
        p.set_color(parse_color(props.stroke_color.as_deref().unwrap_or("#000000")));
        
        if let Some(dash) = &props.dash_array {
            let intervals: Vec<f32> = dash.iter().map(|&x| x as f32).collect();
            // FIXED: Use PathEffect::dash instead of skia_safe::path_effect::dash
            p.set_path_effect(PathEffect::dash(&intervals, 0.0));
        }

        canvas.draw_oval(rect, &p);
        Ok(base.h)
    }

    fn draw_qrcode(&self, canvas: &Canvas, base: &Element, props: &QrcodeProps, y: f64, ctx: &RenderContext) -> Result<f64, String> {
        let content = Interpolator::render(&props.value, ctx.data);
        if content.is_empty() { return Ok(base.h); }

        let level = match props.correction_level.as_deref().unwrap_or("M") {
            "L" => EcLevel::L, "Q" => EcLevel::Q, "H" => EcLevel::H, _ => EcLevel::M,
        };

        let code = QrCode::with_error_correction_level(content.as_bytes(), level)
            .map_err(|e| format!("QR Error: {}", e))?;
        
        let modules_count = code.width();
        if modules_count == 0 { return Ok(base.h); }

        let render_size = props.size.unwrap_or_else(|| base.w.min(base.h));
        let module_size = render_size / modules_count as f64;

        let mut p = Paint::default();
        p.set_color(Color::BLACK);
        p.set_style(PaintStyle::Fill);
        p.set_anti_alias(false);

        let colors = code.to_colors();
        for (i, color) in colors.iter().enumerate() {
            if matches!(color, qrcode::Color::Dark) {
                let row = i / modules_count;
                let col = i % modules_count;
                let rect = Rect::from_xywh(
                    (base.x + col as f64 * module_size) as f32,
                    (y + row as f64 * module_size) as f32,
                    module_size as f32,
                    module_size as f32
                );
                canvas.draw_rect(rect, &p);
            }
        }
        Ok(base.h)
    }

    fn draw_barcode(&self, canvas: &Canvas, base: &Element, props: &BarcodeProps, y: f64, ctx: &RenderContext) -> Result<f64, String> {
        let content = Interpolator::render(&props.value, ctx.data);
        // 占位符绘制
        let rect = Rect::from_xywh(base.x as f32, y as f32, base.w as f32, base.h as f32);
        let mut p = Paint::default();
        p.set_style(PaintStyle::Stroke);
        p.set_color(Color::BLACK);
        canvas.draw_rect(rect, &p);

        // 绘制文字标识
        let text = format!("[Barcode: {}]", content);
        let mut ts = TextStyle::new();
        ts.set_font_size(10.0);
        // FIXED: 使用 set_foreground_paint 替代 set_foreground_color，并将 Color 转换为 Color4f
        ts.set_foreground_paint(&Paint::new(Color4f::from(Color::BLACK), None));
        let mut builder = ParagraphBuilder::new(&ParagraphStyle::new(), &ctx.font_collection);
        builder.push_style(&ts);
        builder.add_text(&text);
        let mut para = builder.build();
        para.layout(base.w as f32);
        para.paint(canvas, Point::new(base.x as f32, y as f32 + (base.h as f32 - para.height())/2.0));

        Ok(base.h)
    }

    fn draw_image_placeholder(&self, canvas: &Canvas, base: &Element, _props: &ImageProps, y: f64, _ctx: &RenderContext) -> Result<f64, String> {
        let rect = Rect::from_xywh(base.x as f32, y as f32, base.w as f32, base.h as f32);
        let mut p = Paint::default();
        p.set_color(Color::LIGHT_GRAY);
        p.set_style(PaintStyle::Fill);
        canvas.draw_rect(rect, &p);
        
        p.set_color(Color::RED);
        p.set_style(PaintStyle::Stroke);
        p.set_stroke_width(1.0);
        canvas.draw_line(Point::new(rect.left(), rect.top()), Point::new(rect.right(), rect.bottom()), &p);
        canvas.draw_line(Point::new(rect.right(), rect.top()), Point::new(rect.left(), rect.bottom()), &p);
        Ok(base.h)
    }

    // -------------------------------------------------------------------------
    // 逻辑计算
    // -------------------------------------------------------------------------

    fn topological_sort<'a>(&self, elements: &'a [Element]) -> Result<Vec<&'a Element>, String> {
        let mut result = Vec::with_capacity(elements.len());
        let mut visited = HashSet::new();
        let mut temp_mark = HashSet::new();
        let elem_map: HashMap<&String, &Element> = elements.iter().map(|e| (&e.id, e)).collect();

        fn visit<'a>(
            id: &'a String,
            map: &HashMap<&'a String, &'a Element>,
            result: &mut Vec<&'a Element>,
            visited: &mut HashSet<&'a String>,
            temp_mark: &mut HashSet<&'a String>,
        ) -> Result<(), String> {
            if visited.contains(id) { return Ok(()); }
            if temp_mark.contains(id) { return Err(format!("Circular dependency: {}", id)); }

            temp_mark.insert(id);
            if let Some(elem) = map.get(id) {
                if let Some(target_id) = &elem.linked_to {
                    visit(target_id, map, result, visited, temp_mark)?;
                }
                temp_mark.remove(id);
                visited.insert(id);
                result.push(elem);
            }
            Ok(())
        }

        for elem in elements {
            if !visited.contains(&elem.id) {
                visit(&elem.id, &elem_map, &mut result, &mut visited, &mut temp_mark)?;
            }
        }
        Ok(result)
    }

    fn calculate_y(&self, element: &Element, ctx: &RenderContext) -> (f64, f64) {
        if let Some(target_id) = &element.linked_to {
            if let Some((target_y, target_h)) = ctx.layout_cache.get(target_id) {
                let prev_bottom = target_y + target_h;
                return (prev_bottom + element.y, prev_bottom);
            }
        }
        (element.y, 0.0)
    }
}

// -----------------------------------------------------------------------------
// 工具类
// -----------------------------------------------------------------------------

struct Interpolator;

impl Interpolator {
    fn get_regex() -> &'static Regex {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new(r"\{\{\s*([a-zA-Z0-9_.]+)\s*\}\}").unwrap())
    }

    pub fn render(template: &str, data: &Value) -> String {
        Self::get_regex().replace_all(template, |caps: &Captures| {
            let path = &caps[1];
            Self::get_value_by_path(data, path).unwrap_or_default()
        }).to_string()
    }

    fn get_value_by_path(data: &Value, path: &str) -> Option<String> {
        let s = Self::get_value_from_obj(data, path);
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }

    fn get_array_by_path<'a>(data: &'a Value, raw_path: &str) -> Option<&'a Vec<Value>> {
        let path = raw_path.trim_matches(|c| c == '{' || c == '}' || c == ' ');
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = data;
        for part in parts {
            if let Some(v) = current.get(part) { current = v; } else { return None; }
        }
        current.as_array()
    }

    fn get_value_from_obj(data: &Value, key: &str) -> String {
        data.get(key)
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                Value::Bool(b) => Some(b.to_string()),
                _ => None
            })
            .unwrap_or_default()
    }
}

fn parse_color(hex: &str) -> Color {
    if hex.len() == 7 && hex.starts_with('#') {
        let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0);
        Color::from_rgb(r, g, b)
    } else {
        Color::BLACK
    }
}