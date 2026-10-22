//! Tauri 入口：注册插件、命令，并把缩略图缓存指向 app cache 目录。

mod commands;
pub mod scene;
mod state;

use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let cache_root = app.path().app_cache_dir()?.join("thumbs").join("v1");
            std::fs::create_dir_all(&cache_root)?;
            eprintln!("[vanillaware] thumbnail cache: {}", cache_root.display());

            app.manage(AppState::new(cache_root));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_dir,
            commands::file_meta,
            commands::file_thumb,
            commands::read_file_head,
            commands::start_index,
            commands::index_status,
            commands::search_files,
            commands::list_frames,
            commands::get_thumb_png,
            commands::get_frame_png,
            commands::preview_scene,
            commands::get_atlas_png,
            commands::convert_mbs,
            commands::export_quad,
            commands::get_cache_info,
            commands::clear_thumb_cache,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
