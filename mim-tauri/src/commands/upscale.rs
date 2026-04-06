use crate::state::AppState;
use mim_core::db::PhotosDb;
use mim_ml::Upscaler;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn upscale_photo(
    folder_path: String,
    photo_id: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state
        .get_or_open_sidecar(&folder_path)
        .map_err(|e| e.to_string())?;
    let photo = PhotosDb::get_by_id(db.reader(), &photo_id)
        .map_err(|e| e.to_string())?
        .ok_or("Photo not found")?;

    let source_path = std::path::Path::new(&folder_path).join(&photo.relative_path);
    if !source_path.exists() {
        return Err("Source file not found".to_string());
    }

    // Build output path: {name}_upscaled.png next to original
    let stem = source_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let output_path = source_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join(format!("{}_upscaled.png", stem));

    let _ = app_handle.emit(
        "upscale-progress",
        serde_json::json!({
            "photo_id": photo_id,
            "stage": "starting",
        }),
    );

    let models_dir = state.config.models_dir.clone();
    let output_path_clone = output_path.clone();

    let upscaler = Upscaler::new(models_dir);
    upscaler
        .upscale(&source_path, &output_path_clone)
        .await
        .map_err(|e| format!("Upscale failed: {e}"))?;

    let _ = app_handle.emit(
        "upscale-progress",
        serde_json::json!({
            "photo_id": photo_id,
            "stage": "done",
            "output_path": output_path.to_string_lossy(),
        }),
    );

    Ok(output_path.to_string_lossy().to_string())
}
