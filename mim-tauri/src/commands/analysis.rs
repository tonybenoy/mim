use crate::state::AppState;
use mim_core::db::{EventsDb, PhotosDb};
use mim_ml::PhotoAnalyzer;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Serialize)]
pub struct AnalysisProgress {
    pub total: usize,
    pub processed: usize,
}

#[tauri::command]
pub async fn analyze_folder(
    folder_path: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<AnalysisProgress, String> {
    tracing::info!("analyze_folder called for: {}", folder_path);
    let db = state
        .get_or_open_sidecar(&folder_path)
        .map_err(|e| {
            tracing::error!("analyze_folder: sidecar open failed: {}", e);
            e.to_string()
        })?;

    let root = folder_path.clone();
    let db_clone = db.clone();

    let result = tokio::task::spawn_blocking(move || {
        let root_path = std::path::Path::new(&root);

        let _ = app_handle.emit(
            "analysis-status",
            serde_json::json!({
                "folder": root,
                "stage": "analyzing",
            }),
        );

        let unprocessed_count = PhotosDb::list_unprocessed_analysis(db_clone.reader())
            .map(|v| v.len())
            .unwrap_or(0);

        tracing::info!("analyze_folder: {} unprocessed photos to analyze", unprocessed_count);

        let processed =
            PhotoAnalyzer::analyze_folder(db_clone.writer(), db_clone.reader(), root_path);

        tracing::info!("analyze_folder: processed {} photos", processed);

        // Also run event clustering
        let _ = PhotoAnalyzer::cluster_events(db_clone.writer(), db_clone.reader());

        let _ = app_handle.emit(
            "analysis-status",
            serde_json::json!({
                "folder": root,
                "stage": "done",
                "processed": processed,
            }),
        );

        AnalysisProgress {
            total: unprocessed_count,
            processed,
        }
    })
    .await
    .map_err(|e| format!("task join: {e}"))?;

    Ok(result)
}

#[tauri::command]
pub async fn analyze_single_photo(
    folder_path: String,
    photo_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    let photo = PhotosDb::get_by_id(db.reader(), &photo_id)
        .map_err(|e| e.to_string())?
        .ok_or("Photo not found")?;

    let root = std::path::PathBuf::from(&folder_path);
    let img_path = root.join(&photo.relative_path);

    let db_clone = db.clone();
    let pid = photo_id.clone();

    tokio::task::spawn_blocking(move || {
        match PhotoAnalyzer::analyze(&photo, &img_path) {
            Some(result) => {
                let colors_json = serde_json::to_string(&result.dominant_colors).ok();
                let _ = PhotosDb::update_analysis(
                    db_clone.writer(), &pid,
                    Some(result.blur_score as f64),
                    colors_json.as_deref(),
                    Some(&result.perceptual_hash),
                    result.is_screenshot,
                    Some(&result.time_of_day),
                    if result.has_text { Some("[text detected]") } else { None },
                );
                Ok(true)
            }
            None => Err("Analysis failed".to_string()),
        }
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
}

#[tauri::command]
pub async fn find_similar_photos(
    folder_path: String,
    photo_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<mim_core::models::Photo>, String> {
    let db = state
        .get_or_open_sidecar(&folder_path)
        .map_err(|e| e.to_string())?;

    let photo = PhotosDb::get_by_id(db.reader(), &photo_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Photo not found".to_string())?;

    let hash = photo
        .perceptual_hash
        .as_ref()
        .ok_or_else(|| "Photo has no perceptual hash — run analysis first".to_string())?;

    // Threshold of 10 bits means fairly similar images
    let similar = PhotosDb::find_similar_by_phash(db.reader(), hash, 10)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|p| p.id != photo_id)
        .collect();

    Ok(similar)
}

#[tauri::command]
pub async fn get_events(
    folder_path: String,
    state: State<'_, AppState>,
) -> Result<Vec<mim_core::db::Event>, String> {
    let db = state
        .get_or_open_sidecar(&folder_path)
        .map_err(|e| e.to_string())?;

    EventsDb::list(db.reader()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_photo_colors(
    folder_path: String,
    photo_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let db = state
        .get_or_open_sidecar(&folder_path)
        .map_err(|e| e.to_string())?;

    let photo = PhotosDb::get_by_id(db.reader(), &photo_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Photo not found".to_string())?;

    match photo.dominant_colors {
        Some(json_str) => {
            let colors: Vec<String> =
                serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
            Ok(colors)
        }
        None => Ok(Vec::new()),
    }
}
