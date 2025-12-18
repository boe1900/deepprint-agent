
use axum::{
    extract::Json,
    routing::{get, post},
    Router,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use crate::engine::Engine;
use std::fs;
use std::path::PathBuf;

// --- 数据结构 ---

#[derive(Serialize)]
struct PrinterInfo {
    name: String,
    system_name: String,
    is_default: bool,
}

#[derive(Deserialize)]
pub struct PrintRequest {
    task_id: String,
    content: String,
    // 新增：宽和高 (单位 mm)，可选参数，默认 A4
    pub width_mm: Option<f32>,
    pub height_mm: Option<f32>,
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
    // 调试用：返回 PDF 的路径方便查看
    debug_path: Option<String>, 
}

// --- 路由处理函数 ---

/// 1. 健康检查
async fn health_check() -> &'static str {
    "DeepPrint Agent is Running (Rust + Skia)"
}

/// 2. 获取打印机列表
async fn get_printers() -> Json<Vec<PrinterInfo>> {
    // 使用 printers crate 获取系统设备
    // 注意：确保 Cargo.toml 中添加了 printers 依赖
    let printers = printers::get_printers();
    
    let list = printers.iter().map(|p| PrinterInfo {
        name: p.name.clone(),
        system_name: p.system_name.clone(),
        is_default: p.is_default,
    }).collect();

    Json(list)
}

/// 3. 处理打印请求 (生成 PDF)
async fn handle_print(Json(req): Json<PrintRequest>) -> Json<ApiResponse> {
    println!("接收到打印任务: {}", req.task_id);

    let engine = Engine::new();

    // 1. 获取 PDF 数据 (现在是 Vec<u8> 类型)
    let pdf_bytes = engine.generate_pdf(&req.content, req.width_mm, req.height_mm);

    let output_path = dirs::desktop_dir()
        .unwrap_or(PathBuf::from("."))
        .join(format!("deepprint_{}.pdf", req.task_id));

    // 2. 写入文件
    // 之前的 pdf_data.as_bytes() 删掉，因为 Vec<u8> 可以直接作为引用传给 fs::write
    match fs::write(&output_path, &pdf_bytes) {
        Ok(_) => Json(ApiResponse {
            success: true,
            message: "PDF Rendered & Saved successfully".to_string(),
            debug_path: Some(output_path.to_string_lossy().to_string()),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            message: format!("File save error: {}", e),
            debug_path: None,
        })
    }
}

// --- 服务启动入口 ---

pub async fn start_server() {
    // 允许跨域 (CORS)，否则 Web 端无法调用 localhost
    let cors = CorsLayer::permissive();

    let app = Router::new()
        .route("/", get(health_check))
        .route("/printers", get(get_printers))
        .route("/print", post(handle_print))
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 18088));
    println!("DeepPrint Agent listening on http://{}", addr);

    // 启动服务
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}