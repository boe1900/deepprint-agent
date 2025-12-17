// 引入模块
mod engine;
mod server;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // --- 核心修改：启动 Axum 后台服务 ---
            // 使用 Tauri 的异步运行时生成一个独立任务
            // 这样 HTTP 服务不会阻塞 GUI 界面
            tauri::async_runtime::spawn(async {
                server::start_server().await;
            });
            // ----------------------------------

            // 仅做演示：启动时打开前端窗口
            let main_window = app.get_webview_window("main").unwrap();
            #[cfg(debug_assertions)] // 仅在开发模式打开控制台
            main_window.open_devtools();

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}