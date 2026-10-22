//! Tauri 入口：装配插件、命令与共享状态。
//!
//! 分层约定：
//! - [`command`]：命令薄层（参数 → 阻塞任务 → DTO）；
//! - [`service`]：业务逻辑（解析、索引、缓存句柄），不依赖命令宏；
//! - [`model`]：跨进程数据结构；
//! - [`config`]：路径与可调参数；
//! - [`state`]：把上述服务装进 Tauri 托管的 `AppState`。

pub mod command;
pub mod config;
pub mod model;
pub mod service;
pub mod state;

use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let cache_root = config::cache_root(app.handle())?;
            std::fs::create_dir_all(&cache_root)?;
            eprintln!("[vanillaware] thumbnail cache: {}", cache_root.display());

            app.manage(AppState::new(cache_root));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            command::library::list_dir,
            command::library::list_formats,
            command::library::detect_profile,
            command::doc::open_doc,
            command::doc::doc_children,
            command::library::file_meta,
            command::library::file_thumb,
            command::library::read_file_head,
            command::index::start_index,
            command::index::index_status,
            command::index::search_files,
            command::texture::list_frames,
            command::texture::get_thumb_png,
            command::texture::get_frame_png,
            command::texture::get_atlas_png,
            command::animation::preview_scene,
            command::animation::convert_mbs,
            command::animation::export_quad,
            command::cache::get_cache_info,
            command::cache::clear_thumb_cache,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
