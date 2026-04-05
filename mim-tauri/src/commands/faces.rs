use crate::state::AppState;
use mim_core::db::{FaceIdentitiesDb, FacesDb, PhotosDb};
use mim_core::models::{Face, FaceIdentity};
use mim_core::thumbnail::generate_face_crop;
use mim_ml::ProcessingProgress;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tracing::info;

#[derive(Debug, Clone, Serialize)]
pub struct ClusteringResult {
    pub clusters_created: usize,
    pub faces_assigned: usize,
    pub noise_faces: usize,
}

#[tauri::command]
pub async fn process_faces(
    folder_path: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<ProcessingProgress, String> {
    let _ = app_handle.emit("face-status", "downloading-models");

    let pipeline = state
        .get_or_init_pipeline_with_handle(&app_handle)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit("face-status", "detecting");

    let db = state
        .get_or_open_sidecar(&folder_path)
        .map_err(|e| e.to_string())?;

    let root = std::path::PathBuf::from(&folder_path);
    let handle = app_handle.clone();

    let result = tokio::task::spawn_blocking(move || {
        pipeline.process_folder(&root, &db, |progress| {
            let _ = handle.emit("face-processing-progress", progress);
        })
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
    .map_err(|e| e.to_string())?;

    info!(
        "Face processing complete: {} faces in {} photos",
        result.faces_found, result.processed
    );

    Ok(result)
}

#[tauri::command]
pub async fn cluster_faces(
    folder_path: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<ClusteringResult, String> {
    let pipeline = state
        .get_or_init_pipeline_with_handle(&app_handle)
        .await
        .map_err(|e| e.to_string())?;

    let sidecar_db = state
        .get_or_open_sidecar(&folder_path)
        .map_err(|e| e.to_string())?;

    let central_db = state.central_db_arc();

    let result = tokio::task::spawn_blocking(move || {
        pipeline.cluster_folder(&sidecar_db, &central_db)
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
    .map_err(|e| e.to_string())?;

    let total_faces: usize = result.iter().map(|c| c.face_ids.len()).sum();

    Ok(ClusteringResult {
        clusters_created: result.len(),
        faces_assigned: total_faces,
        noise_faces: 0,
    })
}

#[tauri::command]
pub async fn get_faces_for_photo(
    folder_path: String,
    photo_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Face>, String> {
    let db = state
        .get_or_open_sidecar(&folder_path)
        .map_err(|e| e.to_string())?;
    FacesDb::list_for_photo(db.reader(), &photo_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_identities(
    state: State<'_, AppState>,
) -> Result<Vec<FaceIdentity>, String> {
    FaceIdentitiesDb::list(state.central_db.reader()).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct IdentityWithAvatar {
    pub identity: FaceIdentity,
    pub face_crop_path: Option<String>,
}

#[tauri::command]
pub async fn get_identities_with_avatars(
    folder_path: String,
    state: State<'_, AppState>,
) -> Result<Vec<IdentityWithAvatar>, String> {
    let identities = FaceIdentitiesDb::list(state.central_db.reader())
        .map_err(|e| e.to_string())?;

    let sidecar_db = state
        .get_or_open_sidecar(&folder_path)
        .map_err(|e| e.to_string())?;

    let best_faces = FacesDb::best_face_per_identity(sidecar_db.reader())
        .map_err(|e| e.to_string())?;

    let root = std::path::Path::new(&folder_path);
    let cache_dir = root.join(".mim").join("thumbnails");

    let mut result = Vec::new();
    for identity in identities {
        let face_crop_path = if let Some((_, photo_id, bbox)) = best_faces.iter().find(|(id, _, _)| id == &identity.id) {
            if let Ok(Some(photo)) = PhotosDb::get_by_id(sidecar_db.reader(), photo_id) {
                let source_path = root.join(&photo.relative_path);
                match generate_face_crop(&source_path, &cache_dir, &identity.id, bbox[0], bbox[1], bbox[2], bbox[3]) {
                    Ok(path) => Some(path.to_string_lossy().to_string()),
                    Err(e) => {
                        tracing::warn!("Face crop failed for {}: {}", identity.name, e);
                        None
                    }
                }
            } else { None }
        } else { None };

        result.push(IdentityWithAvatar {
            identity,
            face_crop_path,
        });
    }

    Ok(result)
}

#[tauri::command]
pub async fn rename_identity(
    identity_id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    FaceIdentitiesDb::update_name(state.central_db.writer(), &identity_id, &name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn merge_identities(
    target_id: String,
    source_id: String,
    folder_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let sidecar_db = state
        .get_or_open_sidecar(&folder_path)
        .map_err(|e| e.to_string())?;

    // Reassign faces in sidecar DB
    FacesDb::reassign_identity(sidecar_db.writer(), &source_id, &target_id)
        .map_err(|e| e.to_string())?;

    // Merge identities in central DB (updates embedding, count, deletes source)
    FaceIdentitiesDb::merge(state.central_db.writer(), &target_id, &source_id)
        .map_err(|e| e.to_string())?;

    info!("Merged identity {} into {}", source_id, target_id);
    Ok(())
}
