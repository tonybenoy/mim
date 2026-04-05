use crate::models::FaceIdentity;
use crate::Result;
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::sync::Arc;

fn f32_slice_to_bytes(v: &[f32]) -> Vec<u8> {
    bytemuck::cast_slice(v).to_vec()
}

fn bytes_to_f32_vec(bytes: &[u8]) -> Vec<f32> {
    if bytes.len() % 4 != 0 {
        return Vec::new();
    }
    bytemuck::cast_slice(bytes).to_vec()
}

pub struct FaceIdentitiesDb;

impl FaceIdentitiesDb {
    pub fn insert(conn: &Arc<Mutex<Connection>>, identity: &FaceIdentity) -> Result<()> {
        let conn = conn.lock();
        let embedding_blob = identity
            .representative_embedding
            .as_ref()
            .map(|e| f32_slice_to_bytes(e));

        conn.execute(
            "INSERT OR REPLACE INTO face_identities (
                id, name, representative_embedding, photo_count, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                identity.id,
                identity.name,
                embedding_blob,
                identity.photo_count,
                identity.created_at.to_rfc3339(),
                identity.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list(conn: &Arc<Mutex<Connection>>) -> Result<Vec<FaceIdentity>> {
        let conn = conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, representative_embedding, photo_count, created_at, updated_at
             FROM face_identities ORDER BY photo_count DESC",
        )?;

        let identities = stmt
            .query_map([], Self::row_to_identity)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(identities)
    }

    pub fn get_by_id(
        conn: &Arc<Mutex<Connection>>,
        id: &str,
    ) -> Result<Option<FaceIdentity>> {
        let conn = conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, representative_embedding, photo_count, created_at, updated_at
             FROM face_identities WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], Self::row_to_identity)?;
        match rows.next() {
            Some(Ok(identity)) => Ok(Some(identity)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    pub fn update_name(conn: &Arc<Mutex<Connection>>, id: &str, name: &str) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE face_identities SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![name, Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    pub fn update_photo_count(conn: &Arc<Mutex<Connection>>, id: &str, count: u32) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE face_identities SET photo_count = ?1, updated_at = ?2 WHERE id = ?3",
            params![count, Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    pub fn delete(conn: &Arc<Mutex<Connection>>, id: &str) -> Result<()> {
        let conn = conn.lock();
        conn.execute("DELETE FROM face_identities WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn delete_all(conn: &Arc<Mutex<Connection>>) -> Result<()> {
        let conn = conn.lock();
        conn.execute("DELETE FROM face_identities", [])?;
        Ok(())
    }

    /// Merge source identity into target. Updates photo count and recomputes
    /// representative embedding as weighted average.
    pub fn merge(
        conn: &Arc<Mutex<Connection>>,
        target_id: &str,
        source_id: &str,
    ) -> Result<()> {
        let conn = conn.lock();
        // Get both identities' photo counts for weighted average
        let (target_count, target_emb): (u32, Option<Vec<u8>>) = conn.query_row(
            "SELECT photo_count, representative_embedding FROM face_identities WHERE id = ?1",
            params![target_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let (source_count, source_emb): (u32, Option<Vec<u8>>) = conn.query_row(
            "SELECT photo_count, representative_embedding FROM face_identities WHERE id = ?1",
            params![source_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let new_count = target_count + source_count;

        // Compute weighted average embedding
        let new_emb = match (target_emb, source_emb) {
            (Some(t), Some(s)) => {
                let t_vec = bytes_to_f32_vec(&t);
                let s_vec = bytes_to_f32_vec(&s);
                if t_vec.len() == s_vec.len() && !t_vec.is_empty() {
                    let tw = target_count as f32;
                    let sw = source_count as f32;
                    let total = tw + sw;
                    let mut avg: Vec<f32> = t_vec.iter().zip(s_vec.iter())
                        .map(|(&a, &b)| (a * tw + b * sw) / total)
                        .collect();
                    // L2 normalize
                    let norm: f32 = avg.iter().map(|x| x * x).sum::<f32>().sqrt();
                    if norm > 1e-10 {
                        for v in avg.iter_mut() { *v /= norm; }
                    }
                    Some(f32_slice_to_bytes(&avg))
                } else {
                    None
                }
            }
            (Some(t), None) => Some(t),
            (None, Some(s)) => Some(s),
            _ => None,
        };

        conn.execute(
            "UPDATE face_identities SET photo_count = ?1, representative_embedding = ?2, updated_at = ?3 WHERE id = ?4",
            params![new_count, new_emb, Utc::now().to_rfc3339(), target_id],
        )?;

        // Delete source identity
        conn.execute("DELETE FROM face_identities WHERE id = ?1", params![source_id])?;

        Ok(())
    }

    fn row_to_identity(row: &rusqlite::Row<'_>) -> rusqlite::Result<FaceIdentity> {
        Ok(FaceIdentity {
            id: row.get(0)?,
            name: row.get(1)?,
            representative_embedding: row
                .get::<_, Option<Vec<u8>>>(2)?
                .map(|b| bytes_to_f32_vec(&b)),
            photo_count: row.get(3)?,
            created_at: row
                .get::<_, String>(4)?
                .parse::<chrono::DateTime<Utc>>()
                .unwrap_or_else(|_| Utc::now()),
            updated_at: row
                .get::<_, String>(5)?
                .parse::<chrono::DateTime<Utc>>()
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}
