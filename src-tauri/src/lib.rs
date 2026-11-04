mod api;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod loopback;
mod media;
mod net;
mod session;

use api::Api;
use tauri::Manager;

/// 自定义协议在 WebView 里的地址（Tauri 2 约定），m3u8 里的分片 / key 都改写到这里。
/// - Windows（WebView2）与 Android：`http://<scheme>.localhost`
/// - macOS / iOS（WKWebView）与 Linux（WebKitGTK）：`<scheme>://localhost`
/// 前端 src/api.ts 的 LOCAL 必须与此一致。
#[cfg(any(windows, target_os = "android"))]
pub const LOCAL_ORIGIN: &str = "http://hg.localhost";
#[cfg(not(any(windows, target_os = "android")))]
pub const LOCAL_ORIGIN: &str = "hg://localhost";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Api::new())
        .setup(|app| {
            if let Ok(dir) = app.path().app_data_dir() {
                let _ = std::fs::create_dir_all(&dir);
                let state: tauri::State<Api> = app.state();
                state.session.bind(dir);
            }
            // macOS：窗口在这里手动创建（tauri.macos.conf.json 里 create=false），
            // 先起本机回环媒体服务，再把地址注入页面，前端 src/api.ts 的 LOCAL 读它。
            #[cfg(target_os = "macos")]
            {
                let origin = tauri::async_runtime::block_on(loopback::start())
                    .map_err(|e| format!("本地媒体服务启动失败: {e}"))?;
                let cfg = app
                    .config()
                    .app
                    .windows
                    .iter()
                    .find(|w| w.label == "main")
                    .cloned()
                    .ok_or("缺少 main 窗口配置")?;
                let script = format!(
                    "window.__HG_LOCAL__ = {};",
                    serde_json::to_string(&origin).unwrap_or_else(|_| "null".into())
                );
                tauri::WebviewWindowBuilder::from_config(app.handle(), &cfg)?
                    .initialization_script(&script)
                    .build()?;
            }
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("hg", |_app, request, responder| {
            let uri = request.uri().clone();
            let path = uri.path().to_string();
            let query = uri.query().unwrap_or("").to_string();
            let is_options = request.method() == tauri::http::Method::OPTIONS;
            let range = request
                .headers()
                .get("range")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            // 注意：请求 URI 在各平台写法不同，不能拿它拼子资源地址，统一用 LOCAL_ORIGIN。
            tauri::async_runtime::spawn(async move {
                let resp = if is_options {
                    media::preflight()
                } else {
                    media::handle(&path, &query, LOCAL_ORIGIN, range.as_deref()).await
                };
                responder.respond(resp);
            });
        })
        .invoke_handler(tauri::generate_handler![
            api::app_boot,
            api::debug_log,
            api::net_diag,
            api::api_login,
            api::api_register,
            api::api_logout,
            api::api_me,
            api::api_feed,
            api::api_category,
            api::api_rank,
            api::api_search,
            api::cache_clear,
            api::api_posts,
            api::api_post,
            api::api_post_like,
            api::api_post_collect,
            api::api_post_comments,
            api::api_video,
            api::api_play,
            api::api_comments,
            api::api_comment_send,
            api::api_like,
            api::api_collect,
            api::api_follow,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
