use crate::state::AppState;
use mim_core::crypto;
use mim_core::db::FoldersDb;
use tauri::State;

/// Lock a folder by setting a password. The password's Argon2 hash is stored in the
/// central DB, and the raw password becomes the SQLCipher key for the sidecar DB.
///
/// If the sidecar already exists unencrypted, it will be closed so the next open
/// uses the new key. (For an existing unencrypted DB, a separate migration to
/// re-encrypt would be needed; this command sets up the lock for new/empty sidecars
/// or sidecars that were already created with this key.)
#[tauri::command]
pub async fn lock_folder(
    folder_path: String,
    password: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Look up the folder source
    let source = FoldersDb::get_source_by_path(state.central_db.reader(), &folder_path)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Folder source not found".to_string())?;

    if source.is_locked {
        return Err("Folder is already locked".to_string());
    }

    // Hash the password for verification storage
    let hash = crypto::hash_password(&password).map_err(|e| e.to_string())?;

    // Update the central DB
    FoldersDb::set_locked(state.central_db.writer(), &source.id, true, Some(&hash))
        .map_err(|e| e.to_string())?;

    // Close the existing (unencrypted) sidecar connection so the next open uses the key
    state.close_sidecar(&folder_path);

    // Re-open with encryption so the sidecar DB file gets encrypted
    state
        .get_or_open_sidecar_with_key(&folder_path, Some(&password))
        .map_err(|e| e.to_string())?;

    tracing::info!("Folder locked: {}", folder_path);
    Ok(())
}

/// Unlock (remove the lock from) a folder. Requires the correct password.
#[tauri::command]
pub async fn unlock_folder(
    folder_path: String,
    password: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let source = FoldersDb::get_source_by_path(state.central_db.reader(), &folder_path)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Folder source not found".to_string())?;

    if !source.is_locked {
        return Err("Folder is not locked".to_string());
    }

    let hash = source
        .password_hash
        .as_deref()
        .ok_or("No password hash stored")?;
    let valid = crypto::verify_password(&password, hash).map_err(|e| e.to_string())?;
    if !valid {
        return Err("Incorrect password".to_string());
    }

    // Remove the lock
    FoldersDb::set_locked(state.central_db.writer(), &source.id, false, None)
        .map_err(|e| e.to_string())?;

    // Close the encrypted sidecar so the next open uses no key
    state.close_sidecar(&folder_path);

    tracing::info!("Folder unlocked: {}", folder_path);
    Ok(())
}

/// Verify a folder password without changing any state. Returns `true` if correct.
/// Used by the frontend to prompt for the password before opening a locked folder.
#[tauri::command]
pub async fn verify_folder_password(
    folder_path: String,
    password: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let source = FoldersDb::get_source_by_path(state.central_db.reader(), &folder_path)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Folder source not found".to_string())?;

    if !source.is_locked {
        return Ok(true);
    }

    let hash = source
        .password_hash
        .as_deref()
        .ok_or("No password hash stored")?;
    crypto::verify_password(&password, hash).map_err(|e| e.to_string())
}

/// Open a locked folder's sidecar database using the provided password.
/// The frontend should call `verify_folder_password` first, then this command
/// to actually make the sidecar available for queries.
#[tauri::command]
pub async fn open_locked_folder(
    folder_path: String,
    password: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let source = FoldersDb::get_source_by_path(state.central_db.reader(), &folder_path)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Folder source not found".to_string())?;

    if !source.is_locked {
        // Not locked, just open normally
        state
            .get_or_open_sidecar(&folder_path)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Verify password first
    let hash = source
        .password_hash
        .as_deref()
        .ok_or("No password hash stored")?;
    let valid = crypto::verify_password(&password, hash).map_err(|e| e.to_string())?;
    if !valid {
        return Err("Incorrect password".to_string());
    }

    // Open the sidecar with the encryption key
    state
        .get_or_open_sidecar_with_key(&folder_path, Some(&password))
        .map_err(|e| e.to_string())?;

    tracing::info!("Locked folder opened: {}", folder_path);
    Ok(())
}
