mod analysis;
mod faces;
mod gemma;
mod albums;
mod import;
mod library;
mod models;
mod secure;
mod settings;
mod sync;
mod upscale;
mod watch;

pub use analysis::*;
pub use faces::*;
pub use gemma::*;
pub use albums::*;
pub use import::*;
pub use library::*;
pub use models::*;
pub use sync::*;
pub use upscale::*;
pub use watch::*;
pub use secure::*;
pub use settings::*;

use crate::state::AppState;
use mim_core::db::{DedupeDb, FoldersDb, PhotosDb, SearchDb};
use rayon::prelude::*;
use mim_core::models::{FolderSource, Photo};
use mim_core::scanner::Scanner;
use mim_core::thumbnail::{ThumbnailGenerator, ThumbnailSize};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Debug, Serialize)]
pub struct ScanProgress {
    pub total_found: usize,
    pub new_photos: usize,
    pub skipped: usize,
    pub errors: usize,
}

#[tauri::command]
pub async fn add_folder(
    path: String,
    label: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<FolderSource, String> {
    let source = FolderSource::new(path.clone(), label);
    FoldersDb::insert_source(state.central_db.writer(), &source)
        .map_err(|e| e.to_string())?;

    // Allow asset protocol access to this folder
    let _ = app_handle.asset_protocol_scope().allow_directory(&path, true);

    // Open sidecar DB for this folder
    state.get_or_open_sidecar(&path).map_err(|e| e.to_string())?;

    Ok(source)
}

#[tauri::command]
pub async fn get_folders(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<FolderSource>, String> {
    let sources = FoldersDb::list_sources(state.central_db.reader())
        .map_err(|e| e.to_string())?;

    // Allow asset protocol for all known folders
    for source in &sources {
        let _ = app_handle.asset_protocol_scope().allow_directory(&source.path, true);
    }

    Ok(sources)
}

#[tauri::command]
pub async fn remove_folder(id: String, state: State<'_, AppState>) -> Result<(), String> {
    FoldersDb::remove_source(state.central_db.writer(), &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_folder(
    path: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<ScanProgress, String> {
    // Allow asset protocol for this folder
    let _ = app_handle.asset_protocol_scope().allow_directory(&path, true);

    let db = state.get_or_open_sidecar(&path).map_err(|e| e.to_string())?;

    let root_path = path.clone();
    let db_clone = db.clone();

    let handle = app_handle.clone();

    // Run scan + thumbnails on a blocking thread so UI stays responsive
    let result = tokio::task::spawn_blocking(move || {
        let root = std::path::Path::new(&root_path);

        let _ = handle.emit("scan-status", serde_json::json!({
            "folder": root.to_string_lossy(),
            "stage": "scanning",
        }));

        let scan_result = Scanner::scan_folder(root, &db_clone)
            .map_err(|e| e.to_string())?;

        let _ = handle.emit("scan-status", serde_json::json!({
            "folder": root.to_string_lossy(),
            "stage": "thumbnails",
            "total": scan_result.total_found,
            "new": scan_result.new_photos,
        }));

        // Generate thumbnails in parallel
        let photos = PhotosDb::list(db_clone.reader(), 50000, 0)
            .map_err(|e| e.to_string())?;
        let cache_dir = root.join(".mim").join("thumbnails");

        let ungenerated: Vec<_> = photos.iter().filter(|p| !p.thumbnail_generated).collect();
        tracing::info!("Generating thumbnails for {} photos", ungenerated.len());

        let thumb_results: Vec<_> = ungenerated
            .par_iter()
            .map(|photo| {
                let source_path = root.join(&photo.relative_path);
                if source_path.exists() {
                    match ThumbnailGenerator::generate_all(&source_path, &cache_dir, &photo.content_hash) {
                        Ok(_) => Some(photo.id.clone()),
                        Err(e) => {
                            tracing::warn!("Thumbnail failed for {}: {}", photo.filename, e);
                            None
                        }
                    }
                } else {
                    None
                }
            })
            .collect();

        for id in thumb_results.into_iter().flatten() {
            let _ = PhotosDb::mark_thumbnail_generated(db_clone.writer(), &id);
        }

        let _ = handle.emit("scan-status", serde_json::json!({
            "folder": root.to_string_lossy(),
            "stage": "done",
            "total": scan_result.total_found,
            "new": scan_result.new_photos,
        }));

        Ok::<_, String>(ScanProgress {
            total_found: scan_result.total_found,
            new_photos: scan_result.new_photos,
            skipped: scan_result.skipped,
            errors: scan_result.errors,
        })
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
    .map_err(|e| e)?;

    Ok(result)
}

#[tauri::command]
pub async fn get_photos(
    folder_path: String,
    limit: Option<u32>,
    offset: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<Photo>, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    PhotosDb::list(db.reader(), limit.unwrap_or(100), offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_photo_detail(
    folder_path: String,
    photo_id: String,
    state: State<'_, AppState>,
) -> Result<Option<Photo>, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    PhotosDb::get_by_id(db.reader(), &photo_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_photo_count(
    folder_path: String,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    PhotosDb::count(db.reader()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_thumbnail_url(
    folder_path: String,
    content_hash: String,
    size: Option<String>,
) -> Result<String, String> {
    let thumb_size = match size.as_deref() {
        Some("micro") => ThumbnailSize::Micro,
        Some("preview") => ThumbnailSize::Preview,
        _ => ThumbnailSize::Grid,
    };

    let cache_dir = std::path::Path::new(&folder_path)
        .join(".mim")
        .join("thumbnails");
    let path = ThumbnailGenerator::thumbnail_path(&cache_dir, &content_hash, thumb_size);

    if path.exists() {
        Ok(path.to_string_lossy().to_string())
    } else {
        Err("Thumbnail not found".to_string())
    }
}

#[tauri::command]
pub async fn search_photos(
    folder_path: String,
    query: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<Photo>, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    SearchDb::search(db.reader(), &query, limit.unwrap_or(100)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn find_duplicates(
    folder_path: String,
    state: State<'_, AppState>,
) -> Result<Vec<mim_core::db::DuplicateGroup>, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    DedupeDb::find_exact_duplicates(db.reader()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn share_photo(
    folder_path: String,
    photo_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    let photo = PhotosDb::get_by_id(db.reader(), &photo_id)
        .map_err(|e| e.to_string())?
        .ok_or("Photo not found")?;
    let full_path = std::path::Path::new(&folder_path).join(&photo.relative_path);
    Ok(full_path.to_string_lossy().to_string())
}
