use crate::state::AppState;
use tauri::State;

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
