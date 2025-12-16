use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tauri::{Manager, Emitter};

// --- 数据结构 ---
#[derive(Serialize, Clone)]
pub struct PrinterDto {
    name: String,
    system_name: String,
    is_default: bool,
}

#[derive(Deserialize)]
struct PrintJob {
    printer: String,
    content: String, 
}

// --- 业务逻辑 ---
fn get_printer_list() -> Vec<PrinterDto> {
    // 调用 printers 库获取系统打印机
    printers::get_printers()
        .into_iter()
        .map(|p| PrinterDto {
            name: p.name.clone(),
            system_name: p.system_name,
            is_default: false, // 简化处理，暂不判断默认
        })
        .collect()
}

// --- HTTP Handlers (给 Web 业务系统用 18088端口) ---
async fn http_list_printers() -> Json<Vec<PrinterDto>> {
    Json(get_printer_list())
}

async fn http_handle_print(Json(payload): Json<PrintJob>) -> Json<serde_json::Value> {
    println!(">>> [HTTP] 收到打印任务: 打印机={}, 内容长度={}", payload.printer, payload.content.len());
    // TODO: 这里接入系统打印 API
    Json(serde_json::json!({ "status": "success", "job_id": "mock-123" }))
}

// --- Tauri Commands (给 Agent 自身 UI 用) ---
#[tauri::command]
fn agent_get_printers() -> Vec<PrinterDto> {
    get_printer_list()
}

// --- 启动 HTTP 服务 ---
async fn start_server() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/printers", get(http_list_printers))
        .route("/print", post(http_handle_print))
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 18088));
    println!("DeepPrint Agent listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 在后台线程启动 HTTP 服务
            tauri::async_runtime::spawn(start_server());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![agent_get_printers])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}