#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use state::AppState;
use tauri::Manager;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

fn main() {
    // Set up log directory: ~/.local/share/mim/logs/
    let log_dir = directories::ProjectDirs::from("com", "mim", "mim")
        .expect("could not determine app data directory")
        .data_dir()
        .join("logs");
    std::fs::create_dir_all(&log_dir).expect("failed to create log directory");

    // Daily file rotation, keeping 7 days of logs
    let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("mim")
        .filename_suffix("log")
        .max_log_files(7)
        .build(&log_dir)
        .expect("failed to create log file appender");
    // Non-blocking writer so logging never stalls the UI thread
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,ort=warn"));

    // Log to both console (stderr) AND file
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(non_blocking),
        )
        .init();

    tracing::info!("Starting Mim photo library manager");
    tracing::info!("Log files stored in: {}", log_dir.display());

    let app_state = AppState::new().expect("Failed to initialize app state");

    tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Allow asset protocol access to all known folders at startup
            let state = app.state::<AppState>();
            if let Ok(sources) = mim_core::db::FoldersDb::list_sources(state.central_db.reader()) {
                for source in &sources {
                    tracing::info!("Allowing asset protocol for: {}", source.path);
                    if let Err(e) = app.asset_protocol_scope().allow_directory(&source.path, true) {
                        tracing::warn!("Failed to allow asset scope for {}: {}", source.path, e);
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::add_folder,
            commands::get_folders,
            commands::remove_folder,
            commands::get_photos,
            commands::get_photo_detail,
            commands::get_photo_count,
            commands::scan_folder,
            commands::get_thumbnail_url,
            commands::process_faces,
            commands::cluster_faces,
            commands::get_faces_for_photo,
            commands::get_identities,
            commands::get_identities_with_avatars,
            commands::rename_identity,
            commands::merge_identities,
            commands::tag_photos,
            commands::chat_about_photo,
            commands::search_photos,
            commands::create_album,
            commands::get_albums,
            commands::add_to_album,
            commands::remove_from_album,
            commands::get_album_photos,
            commands::delete_album,
            commands::rename_album,
            commands::find_duplicates,
            commands::share_photo,
            commands::lock_folder,
            commands::unlock_folder,
            commands::verify_folder_password,
            commands::open_locked_folder,
            commands::analyze_folder,
            commands::find_similar_photos,
            commands::get_events,
            commands::get_photo_colors,
            commands::setup_sync,
            commands::get_sync_status,
            commands::stop_sync,
            commands::add_sync_folder,
            commands::watch_folder,
            commands::unwatch_folder,
            commands::toggle_favorite,
            commands::set_rating,
            commands::trash_photo,
            commands::restore_photo,
            commands::empty_trash,
            commands::get_trashed,
            commands::open_video_external,
            commands::share_photo_os,
            commands::backup_database,
            commands::restore_database,
            commands::get_storage_stats,
            commands::get_memories,
            commands::create_smart_album,
            commands::get_smart_album_photos,
            commands::export_album_zip,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Mim");
}
