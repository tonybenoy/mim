use crate::state::AppState;
use mim_core::watcher::{FolderEvent, FolderWatcher};
use tauri::{AppHandle, Emitter, State};
use tracing::info;

/// Start watching a folder for new files (from Syncthing, Dropbox, manual copies, etc.)
/// Emits "folder-files-changed" events when new photos appear.
#[tauri::command]
pub async fn watch_folder(
    folder_path: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(&folder_path);
    let handle = app_handle.clone();

    let (watcher, rx) = FolderWatcher::watch(&path).map_err(|e| e.to_string())?;

    // Store watcher in state so it isn't dropped
    state.add_watcher(folder_path.clone(), watcher);

    // Spawn background task to process events
    let folder = folder_path.clone();
    tokio::task::spawn_blocking(move || {
        while let Ok(event) = rx.recv() {
            match event {
                FolderEvent::FilesChanged { paths } => {
                    let count = paths.len();
                    info!("Detected {} new/changed files in {}", count, folder);
                    let _ = handle.emit(
                        "folder-files-changed",
                        serde_json::json!({
                            "folder": folder,
                            "count": count,
                            "files": paths.iter()
                                .map(|p| p.to_string_lossy().to_string())
                                .collect::<Vec<_>>(),
                        }),
                    );
                }
                FolderEvent::FilesRemoved { paths } => {
                    let count = paths.len();
                    info!("Detected {} removed files in {}", count, folder);
                    let _ = handle.emit(
                        "folder-files-removed",
                        serde_json::json!({
                            "folder": folder,
                            "count": count,
                        }),
                    );
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn unwatch_folder(
    folder_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.remove_watcher(&folder_path);
    Ok(())
}
