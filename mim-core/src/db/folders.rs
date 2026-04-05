use crate::models::FolderSource;
use crate::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::sync::Arc;

pub struct FoldersDb;

impl FoldersDb {
    pub fn insert_source(conn: &Arc<Mutex<Connection>>, source: &FolderSource) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO folder_sources (id, path, label, drive_serial, is_available, is_locked, password_hash, last_scanned_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                source.id,
                source.path,
                source.label,
                source.drive_serial,
                source.is_available,
                source.is_locked,
                source.password_hash,
                source.last_scanned_at.map(|t| t.to_rfc3339()),
                source.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list_sources(conn: &Arc<Mutex<Connection>>) -> Result<Vec<FolderSource>> {
        let conn = conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, path, label, drive_serial, is_available, is_locked, password_hash, last_scanned_at, created_at
             FROM folder_sources ORDER BY created_at"
        )?;

        let sources = stmt.query_map([], |row| {
            Ok(FolderSource {
                id: row.get(0)?,
                path: row.get(1)?,
                label: row.get(2)?,
                drive_serial: row.get(3)?,
                is_available: row.get(4)?,
                is_locked: row.get(5)?,
                password_hash: row.get(6)?,
                last_scanned_at: row.get::<_, Option<String>>(7)?
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                created_at: row.get::<_, String>(8)?
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now()),
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(sources)
    }

    pub fn get_source_by_path(conn: &Arc<Mutex<Connection>>, path: &str) -> Result<Option<FolderSource>> {
        let conn = conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, path, label, drive_serial, is_available, is_locked, password_hash, last_scanned_at, created_at
             FROM folder_sources WHERE path = ?1"
        )?;

        let mut rows = stmt.query_map(params![path], |row| {
            Ok(FolderSource {
                id: row.get(0)?,
                path: row.get(1)?,
                label: row.get(2)?,
                drive_serial: row.get(3)?,
                is_available: row.get(4)?,
                is_locked: row.get(5)?,
                password_hash: row.get(6)?,
                last_scanned_at: row.get::<_, Option<String>>(7)?
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                created_at: row.get::<_, String>(8)?
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .unwrap_or_else(|_| chrono::Utc::now()),
            })
        })?;

        match rows.next() {
            Some(Ok(source)) => Ok(Some(source)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    pub fn set_locked(
        conn: &Arc<Mutex<Connection>>,
        folder_id: &str,
        is_locked: bool,
        password_hash: Option<&str>,
    ) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE folder_sources SET is_locked = ?1, password_hash = ?2 WHERE id = ?3",
            params![is_locked, password_hash, folder_id],
        )?;
        Ok(())
    }

    pub fn remove_source(conn: &Arc<Mutex<Connection>>, id: &str) -> Result<()> {
        let conn = conn.lock();
        conn.execute("DELETE FROM folder_sources WHERE id = ?1", params![id])?;
        Ok(())
    }
}
