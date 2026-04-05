use crate::models::{Tag, PhotoTag};
use crate::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::sync::Arc;

pub struct TagsDb;

impl TagsDb {
    pub fn insert(conn: &Arc<Mutex<Connection>>, tag: &Tag) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "INSERT OR IGNORE INTO tags (id, name, category) VALUES (?1, ?2, ?3)",
            params![tag.id, tag.name, tag.category],
        )?;
        Ok(())
    }

    pub fn add_to_photo(conn: &Arc<Mutex<Connection>>, photo_tag: &PhotoTag) -> Result<()> {
        let conn = conn.lock();
        let source = match photo_tag.source {
            crate::models::TagSource::Gemma => "gemma",
            crate::models::TagSource::User => "user",
            crate::models::TagSource::Face => "face",
        };
        conn.execute(
            "INSERT OR REPLACE INTO photo_tags (photo_id, tag_id, confidence, source)
             VALUES (?1, ?2, ?3, ?4)",
            params![photo_tag.photo_id, photo_tag.tag_id, photo_tag.confidence, source],
        )?;
        Ok(())
    }

    pub fn list_for_photo(conn: &Arc<Mutex<Connection>>, photo_id: &str) -> Result<Vec<Tag>> {
        let conn = conn.lock();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.category FROM tags t
             JOIN photo_tags pt ON t.id = pt.tag_id
             WHERE pt.photo_id = ?1"
        )?;

        let tags = stmt.query_map(params![photo_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(tags)
    }
}
