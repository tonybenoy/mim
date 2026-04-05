use crate::state::AppState;
use mim_core::db::AlbumsDb;
use mim_core::models::{Album, AlbumType};
use tauri::State;

#[tauri::command]
pub async fn create_album(
    folder_path: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<Album, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    let album = Album::new(name, AlbumType::Manual);
    AlbumsDb::create(db.writer(), &album).map_err(|e| e.to_string())?;
    Ok(album)
}

#[tauri::command]
pub async fn get_albums(
    folder_path: String,
    state: State<'_, AppState>,
) -> Result<Vec<Album>, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    AlbumsDb::list(db.reader()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_to_album(
    folder_path: String,
    album_id: String,
    photo_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    AlbumsDb::add_photo(db.writer(), &album_id, &photo_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_from_album(
    folder_path: String,
    album_id: String,
    photo_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    AlbumsDb::remove_photo(db.writer(), &album_id, &photo_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_album_photos(
    folder_path: String,
    album_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    AlbumsDb::get_photos(db.reader(), &album_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_album(
    folder_path: String,
    album_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    AlbumsDb::delete(db.writer(), &album_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_album(
    folder_path: String,
    album_id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    AlbumsDb::rename(db.writer(), &album_id, &name).map_err(|e| e.to_string())
}
