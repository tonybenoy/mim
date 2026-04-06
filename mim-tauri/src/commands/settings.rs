use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct GpuInfo {
    pub cuda_available: bool,
    pub label: String,
}

#[tauri::command]
pub async fn get_gpu_info() -> Result<GpuInfo, String> {
    let cuda_available = mim_ml::is_cuda_available();
    let label = if cuda_available {
        "GPU: CUDA available".to_string()
    } else {
        "GPU: CPU only (build with --features mim-ml/cuda for GPU)".to_string()
    };
    Ok(GpuInfo { cuda_available, label })
}

#[tauri::command]
pub async fn get_setting(
    key: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let conn = state.central_db.reader().lock();
    let mut stmt = conn
        .prepare("SELECT value FROM settings WHERE key = ?1")
        .map_err(|e| e.to_string())?;
    let result: Option<String> = stmt
        .query_row(&[&key], |row| row.get(0))
        .ok();
    Ok(result)
}

#[tauri::command]
pub async fn set_setting(
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let conn = state.central_db.writer().lock();
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        &[&key, &value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
