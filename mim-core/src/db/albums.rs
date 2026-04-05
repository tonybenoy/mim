use crate::models::{Album, AlbumType};
use crate::Result;
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::sync::Arc;

pub struct AlbumsDb;

impl AlbumsDb {
    pub fn create(conn: &Arc<Mutex<Connection>>, album: &Album) -> Result<()> {
        let conn = conn.lock();
        let album_type = match album.album_type {
            AlbumType::Manual => "manual",
            AlbumType::Smart => "smart",
            AlbumType::Favorites => "favorites",
        };
        conn.execute(
            "INSERT INTO albums (id, name, cover_photo_id, album_type, rules, photo_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                album.id,
                album.name,
                album.cover_photo_id,
                album_type,
                album.rules.as_ref().map(|r| r.to_string()),
                album.photo_count,
                album.created_at.to_rfc3339(),
                album.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list(conn: &Arc<Mutex<Connection>>) -> Result<Vec<Album>> {
        let conn = conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, cover_photo_id, album_type, rules, photo_count, created_at, updated_at
             FROM albums ORDER BY updated_at DESC",
        )?;

        let albums = stmt
            .query_map([], |row| {
                let type_str: String = row.get(3)?;
                Ok(Album {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    cover_photo_id: row.get(2)?,
                    album_type: match type_str.as_str() {
                        "smart" => AlbumType::Smart,
                        "favorites" => AlbumType::Favorites,
                        _ => AlbumType::Manual,
                    },
                    rules: row
                        .get::<_, Option<String>>(4)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    photo_count: row.get(5)?,
                    created_at: row
                        .get::<_, String>(6)?
                        .parse::<chrono::DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: row
                        .get::<_, String>(7)?
                        .parse::<chrono::DateTime<Utc>>()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(albums)
    }

    pub fn add_photo(conn: &Arc<Mutex<Connection>>, album_id: &str, photo_id: &str) -> Result<()> {
        let conn = conn.lock();
        let position: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(position), 0) + 1 FROM album_photos WHERE album_id = ?1",
                params![album_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        conn.execute(
            "INSERT OR IGNORE INTO album_photos (album_id, photo_id, position) VALUES (?1, ?2, ?3)",
            params![album_id, photo_id, position],
        )?;

        conn.execute(
            "UPDATE albums SET photo_count = (SELECT COUNT(*) FROM album_photos WHERE album_id = ?1), updated_at = ?2 WHERE id = ?1",
            params![album_id, Utc::now().to_rfc3339()],
        )?;

        // Set cover if none
        conn.execute(
            "UPDATE albums SET cover_photo_id = ?2 WHERE id = ?1 AND cover_photo_id IS NULL",
            params![album_id, photo_id],
        )?;

        Ok(())
    }

    pub fn remove_photo(conn: &Arc<Mutex<Connection>>, album_id: &str, photo_id: &str) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "DELETE FROM album_photos WHERE album_id = ?1 AND photo_id = ?2",
            params![album_id, photo_id],
        )?;
        conn.execute(
            "UPDATE albums SET photo_count = (SELECT COUNT(*) FROM album_photos WHERE album_id = ?1), updated_at = ?2 WHERE id = ?1",
            params![album_id, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_photos(conn: &Arc<Mutex<Connection>>, album_id: &str) -> Result<Vec<String>> {
        let conn = conn.lock();
        let mut stmt = conn.prepare(
            "SELECT photo_id FROM album_photos WHERE album_id = ?1 ORDER BY position",
        )?;
        let ids = stmt
            .query_map(params![album_id], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(ids)
    }

    pub fn delete(conn: &Arc<Mutex<Connection>>, album_id: &str) -> Result<()> {
        let conn = conn.lock();
        conn.execute("DELETE FROM album_photos WHERE album_id = ?1", params![album_id])?;
        conn.execute("DELETE FROM albums WHERE id = ?1", params![album_id])?;
        Ok(())
    }

    pub fn rename(conn: &Arc<Mutex<Connection>>, album_id: &str, name: &str) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE albums SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![name, Utc::now().to_rfc3339(), album_id],
        )?;
        Ok(())
    }
}
