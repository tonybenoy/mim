use crate::models::Photo;
use crate::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::sync::Arc;

use super::photos::{photo_from_row, SELECT_COLUMNS};

pub struct SearchDb;

impl SearchDb {
    /// Search photos by filename, AI description, or tags.
    pub fn search(conn: &Arc<Mutex<Connection>>, query: &str, limit: u32) -> Result<Vec<Photo>> {
        let conn = conn.lock();
        let pattern = format!("%{}%", query);

        // Build the SELECT with prefixed columns for the JOIN
        let prefixed_cols = SELECT_COLUMNS
            .split(',')
            .map(|c| format!("p.{}", c.trim()))
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            "SELECT DISTINCT {}
             FROM photos p
             LEFT JOIN photo_tags pt ON p.id = pt.photo_id
             LEFT JOIN tags t ON pt.tag_id = t.id
             WHERE p.filename LIKE ?1
                OR p.ai_description LIKE ?1
                OR p.location_name LIKE ?1
                OR t.name LIKE ?1
             ORDER BY COALESCE(p.taken_at, p.created_at) DESC
             LIMIT ?2",
            prefixed_cols
        );

        let mut stmt = conn.prepare(&sql)?;
        let photos = stmt
            .query_map(params![pattern, limit], photo_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(photos)
    }
}
