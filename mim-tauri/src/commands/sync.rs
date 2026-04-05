use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tracing::info;

#[derive(Debug, Clone, Serialize)]
pub struct SyncStatus {
    pub installed: bool,
    pub running: bool,
    pub device_id: Option<String>,
    pub api_port: u16,
}

#[tauri::command]
pub async fn setup_sync(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<SyncStatus, String> {
    let _ = app_handle.emit("sync-status", "downloading");

    state
        .ensure_sync_binary()
        .await
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit("sync-status", "starting");

    state.start_sync().map_err(|e| e.to_string())?;

    // Wait a moment for Syncthing to initialize
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let device_id = state.get_sync_device_id().await.ok();

    let _ = app_handle.emit("sync-status", "running");

    info!("Sync setup complete, device ID: {:?}", device_id);

    Ok(SyncStatus {
        installed: true,
        running: true,
        device_id,
        api_port: state.sync_api_port(),
    })
}

#[tauri::command]
pub async fn get_sync_status(
    state: State<'_, AppState>,
) -> Result<SyncStatus, String> {
    let running = state.is_sync_running();
    let device_id = if running {
        state.get_sync_device_id().await.ok()
    } else {
        None
    };

    Ok(SyncStatus {
        installed: state.is_sync_installed(),
        running,
        device_id,
        api_port: state.sync_api_port(),
    })
}

#[tauri::command]
pub async fn stop_sync(
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.stop_sync();
    Ok(())
}

#[tauri::command]
pub async fn add_sync_folder(
    folder_path: String,
    label: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .add_sync_folder(&folder_path, &label)
        .await
        .map_err(|e| e.to_string())
}
