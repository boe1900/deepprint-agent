use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// DeepPrint 协议顶层结构 (v6.1)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepPrintTemplate {
    pub meta: Meta,
    /// 数据契约：描述模板预期的动态数据结构
    pub data_schema: String,
    /// 资源池 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<HashMap<String, String>>,
    pub canvas: Canvas,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    /// 固定为 "6.1"
    pub version: String,
    /// 模板名称
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Canvas {
    /// 纸张宽度 (pt)
    pub width: f64,
    /// 纸张高度 (pt)。若 orientation=3，此值作为最小高度参考。
    pub height: f64,
    /// 1:纵向; 2:横向; 3:高度自适应(长小票)。默认 1。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<u8>,
    /// 全局默认样式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub styles: Option<GlobalStyles>,
    /// 打印项列表。渲染顺序遵循数组顺序。
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalStyles {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_color: Option<String>,
}

/// 基础元素包装器
/// 包含所有元素共有的属性，并扁平化具体类型的特有属性
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Element {
    /// 元素唯一标识符
    pub id: String,
    /// 左上角 X 坐标 (pt)
    pub x: f64,
    /// 左上角 Y 坐标 (pt)
    pub y: f64,
    /// 宽度 (pt)
    pub w: f64,
    /// 高度 (pt)
    pub h: f64,
    /// 锚点目标元素ID，用于垂直方向相对定位
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_to: Option<String>,

    /// 具体元素的特有属性 (根据 type 字段区分)
    #[serde(flatten)]
    pub data: ElementData,
}

/// 元素类型枚举
/// 使用 `tag = "type"` 自动处理 JSON 中的 type 字段
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ElementData {
    Text(TextProps),
    Table(TableProps),
    Image(ImageProps),
    Barcode(BarcodeProps),
    Qrcode(QrcodeProps),
    Line(LineProps),
    Rect(RectProps),
    Ellipse(EllipseProps),
}

// -----------------------------------------------------------------------------
// 具体元素属性定义
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextProps {
    /// 字符串内容。支持 {{var}} 插值。
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    /// 支持 "bold", "normal" 或数字 700, 400
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<FontWeight>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_color: Option<String>,
    /// 行高倍率 (Default: 1.2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f64>,
    /// "left", "center", "right"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_align: Option<String>,
    /// "top", "middle", "bottom"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<String>,
    /// "underline", "none"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_decoration: Option<String>,
    /// 1: 溢出时自动缩小字号；0: 不缩小
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_shrink: Option<u8>,
    /// 1: 自动换行；0: 禁止换行
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_break: Option<u8>,
    /// 是否根据内容自动计算高度 (Default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_height: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableProps {
    /// 数据源变量名，如 "{{items}}"
    pub data: String,
    pub columns: Vec<TableColumn>,
    /// 1: 每页重复表头；0: 仅首页 (Default: 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_head: Option<u8>,
    /// 单元格内边距 (Default: 5)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell_padding: Option<f64>,
    /// 边框线宽 (Default: 2.83)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_height: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableColumn {
    pub title: String,
    /// 对应 rows 数据中的字段键名
    pub field: String,
    /// 列宽。支持百分比（"20%"）或固定pt数值（100.0）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<TableColumnWidth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_align: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageProps {
    /// 图片引用或 URL
    pub src: String,
    /// "contain", "cover", "fill"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_fit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BarcodeProps {
    pub value: String,
    /// 如 "CODE128", "EAN13"
    pub format: String,
    /// 是否在条码下方显示文字 (1:是; 0:否)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_value: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrcodeProps {
    pub value: String,
    /// "L", "M", "Q", "H"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correction_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineProps {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dash_array: Option<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RectProps {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dash_array: Option<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EllipseProps {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dash_array: Option<Vec<f64>>,
}

// -----------------------------------------------------------------------------
// 辅助枚举 (Untagged Enums)
// -----------------------------------------------------------------------------

/// 处理 fontWeight 的多态类型 (String 或 Number)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FontWeight {
    String(String),
    Number(u16),
}

/// 处理表格列宽的多态类型 (百分比String 或 绝对数值Number)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TableColumnWidth {
    Fixed(f64),
    Percentage(String),
}