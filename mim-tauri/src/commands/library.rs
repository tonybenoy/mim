use crate::state::AppState;
use mim_core::db::PhotosDb;
use mim_core::models::Photo;
use serde::Serialize;
use tauri::State;

// ── Favorites ───────────────────────────────────────────

#[tauri::command]
pub async fn toggle_favorite(
    folder_path: String,
    photo_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    PhotosDb::toggle_favorite(db.writer(), &photo_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_rating(
    folder_path: String,
    photo_id: String,
    rating: u8,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    PhotosDb::set_rating(db.writer(), &photo_id, rating).map_err(|e| e.to_string())
}

// ── Trash ───────────────────────────────────────────────

#[tauri::command]
pub async fn trash_photo(
    folder_path: String,
    photo_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    PhotosDb::trash_photo(db.writer(), &photo_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restore_photo(
    folder_path: String,
    photo_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    PhotosDb::restore_photo(db.writer(), &photo_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn empty_trash(
    folder_path: String,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    PhotosDb::empty_trash(db.writer()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_trashed(
    folder_path: String,
    state: State<'_, AppState>,
) -> Result<Vec<Photo>, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    PhotosDb::list_trashed(db.reader()).map_err(|e| e.to_string())
}

// ── Video External ──────────────────────────────────────

#[tauri::command]
pub async fn open_video_external(
    folder_path: String,
    photo_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    let photo = PhotosDb::get_by_id(db.reader(), &photo_id)
        .map_err(|e| e.to_string())?
        .ok_or("Photo not found")?;
    let full_path = std::path::Path::new(&folder_path).join(&photo.relative_path);
    open::that(&full_path).map_err(|e| format!("Failed to open: {}", e))?;
    Ok(())
}

// ── Share via OS (open containing folder) ───────────────

#[tauri::command]
pub async fn share_photo_os(
    folder_path: String,
    photo_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    let photo = PhotosDb::get_by_id(db.reader(), &photo_id)
        .map_err(|e| e.to_string())?
        .ok_or("Photo not found")?;
    let full_path = std::path::Path::new(&folder_path).join(&photo.relative_path);
    // Open the containing folder
    if let Some(parent) = full_path.parent() {
        open::that(parent).map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    Ok(())
}

// ── Database Backup/Restore ─────────────────────────────

#[tauri::command]
pub async fn backup_database(
    folder_path: String,
    dest_path: String,
) -> Result<(), String> {
    let db_path = std::path::Path::new(&folder_path).join(".mim").join("mim.db");
    if !db_path.exists() {
        return Err("Database not found".to_string());
    }
    std::fs::copy(&db_path, &dest_path)
        .map_err(|e| format!("Backup failed: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn restore_database(
    folder_path: String,
    source_path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let source = std::path::Path::new(&source_path);
    if !source.exists() {
        return Err("Backup file not found".to_string());
    }
    // Close existing sidecar connection
    state.close_sidecar(&folder_path);

    let db_path = std::path::Path::new(&folder_path).join(".mim").join("mim.db");
    std::fs::copy(source, &db_path)
        .map_err(|e| format!("Restore failed: {}", e))?;

    // Reopen the sidecar
    state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    Ok(())
}

// ── Storage Stats ───────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct StorageStats {
    pub total_photos: u32,
    pub total_size: u64,
    pub thumbnail_size: u64,
    pub face_crops_size: u64,
    pub db_size: u64,
}

fn dir_size(path: &std::path::Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

#[tauri::command]
pub async fn get_storage_stats(
    folder_path: String,
    state: State<'_, AppState>,
) -> Result<StorageStats, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    let root = std::path::Path::new(&folder_path);
    let mim_dir = root.join(".mim");

    let total_photos = PhotosDb::count(db.reader()).map_err(|e| e.to_string())?;
    let total_size = PhotosDb::get_total_photo_size(db.reader()).map_err(|e| e.to_string())?;

    let thumbnail_size = dir_size(&mim_dir.join("thumbnails"));
    let face_crops_size = dir_size(&mim_dir.join("face_crops"));
    let db_path = mim_dir.join("mim.db");
    let db_size = db_path.metadata().map(|m| m.len()).unwrap_or(0);

    Ok(StorageStats {
        total_photos,
        total_size,
        thumbnail_size,
        face_crops_size,
        db_size,
    })
}

// ── Memories (On This Day) ──────────────────────────────

#[tauri::command]
pub async fn get_memories(
    folder_path: String,
    state: State<'_, AppState>,
) -> Result<Vec<Photo>, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    PhotosDb::get_memories(db.reader()).map_err(|e| e.to_string())
}

// ── Smart Albums ────────────────────────────────────────

#[tauri::command]
pub async fn create_smart_album(
    folder_path: String,
    name: String,
    rules_json: String,
    state: State<'_, AppState>,
) -> Result<mim_core::models::Album, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    let rules_value: serde_json::Value = serde_json::from_str(&rules_json)
        .map_err(|e| format!("Invalid rules JSON: {}", e))?;
    let album = mim_core::models::Album {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        cover_photo_id: None,
        album_type: mim_core::models::AlbumType::Smart,
        rules: Some(rules_value),
        photo_count: 0,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    mim_core::db::AlbumsDb::create(db.writer(), &album).map_err(|e| e.to_string())?;
    Ok(album)
}

#[tauri::command]
pub async fn get_smart_album_photos(
    folder_path: String,
    album_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Photo>, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;

    // Get the album to find its rules
    let albums = mim_core::db::AlbumsDb::list(db.reader()).map_err(|e| e.to_string())?;
    let album = albums.iter()
        .find(|a| a.id == album_id)
        .ok_or("Album not found")?;

    let rules = album.rules.as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".to_string());
    PhotosDb::query_smart_album(db.reader(), &rules).map_err(|e| e.to_string())
}

// ── Export ──────────────────────────────────────────────

#[tauri::command]
pub async fn export_album_zip(
    folder_path: String,
    album_id: String,
    dest_path: String,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    let photo_ids = mim_core::db::AlbumsDb::get_photos(db.reader(), &album_id)
        .map_err(|e| e.to_string())?;

    // Create destination directory
    let dest = std::path::Path::new(&dest_path);
    std::fs::create_dir_all(dest).map_err(|e| format!("Create dir failed: {}", e))?;

    let mut count = 0u32;
    for pid in &photo_ids {
        if let Ok(Some(photo)) = PhotosDb::get_by_id(db.reader(), pid) {
            let src = std::path::Path::new(&folder_path).join(&photo.relative_path);
            let dst = dest.join(&photo.filename);
            if src.exists() {
                if let Ok(_) = std::fs::copy(&src, &dst) {
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}
