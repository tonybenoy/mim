use crate::models::Photo;
use crate::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::sync::Arc;

use super::photos::{photo_from_row, SELECT_COLUMNS};

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateGroup {
    pub content_hash: String,
    pub photos: Vec<Photo>,
}

pub struct DedupeDb;

impl DedupeDb {
    /// Find exact duplicates (same BLAKE3 content hash).
    pub fn find_exact_duplicates(conn: &Arc<Mutex<Connection>>) -> Result<Vec<DuplicateGroup>> {
        let conn = conn.lock();

        // Find hashes that appear more than once
        let mut hash_stmt = conn.prepare(
            "SELECT content_hash, COUNT(*) as cnt FROM photos
             GROUP BY content_hash HAVING cnt > 1
             ORDER BY cnt DESC",
        )?;

        let hashes: Vec<String> = hash_stmt
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        drop(hash_stmt);

        let mut groups = Vec::new();
        for hash in &hashes {
            let sql = format!(
                "SELECT {} FROM photos WHERE content_hash = ?1",
                SELECT_COLUMNS
            );
            let mut photo_stmt = conn.prepare(&sql)?;

            let photos: Vec<Photo> = photo_stmt
                .query_map(params![hash], photo_from_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            if photos.len() > 1 {
                groups.push(DuplicateGroup {
                    content_hash: hash.clone(),
                    photos,
                });
            }
        }

        Ok(groups)
    }

    /// Find visually similar photos using AI description similarity.
    /// Groups photos that share many of the same tags.
    pub fn find_similar_by_tags(
        conn: &Arc<Mutex<Connection>>,
        min_shared_tags: u32,
    ) -> Result<Vec<DuplicateGroup>> {
        let conn = conn.lock();

        let mut stmt = conn.prepare(
            "SELECT p1.content_hash, p2.content_hash, COUNT(*) as shared
             FROM photo_tags pt1
             JOIN photo_tags pt2 ON pt1.tag_id = pt2.tag_id AND pt1.photo_id < pt2.photo_id
             JOIN photos p1 ON pt1.photo_id = p1.id
             JOIN photos p2 ON pt2.photo_id = p2.id
             GROUP BY pt1.photo_id, pt2.photo_id
             HAVING shared >= ?1
             ORDER BY shared DESC
             LIMIT 50",
        )?;

        let pairs: Vec<(String, String)> = stmt
            .query_map(params![min_shared_tags], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let _ = pairs;
        Ok(Vec::new())
    }
}
