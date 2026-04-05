use crate::models::Face;
use crate::Result;
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::sync::Arc;

pub struct FacesDb;

fn f32_slice_to_bytes(v: &[f32]) -> Vec<u8> {
    bytemuck::cast_slice(v).to_vec()
}

fn bytes_to_f32_vec(bytes: &[u8]) -> Vec<f32> {
    if bytes.len() % 4 != 0 {
        return Vec::new();
    }
    bytemuck::cast_slice(bytes).to_vec()
}

impl FacesDb {
    pub fn insert(conn: &Arc<Mutex<Connection>>, face: &Face) -> Result<()> {
        let conn = conn.lock();
        let landmarks_blob = face.landmarks.as_ref().map(|l| f32_slice_to_bytes(l));
        let embedding_blob = face.embedding.as_ref().map(|e| f32_slice_to_bytes(e));

        conn.execute(
            "INSERT OR REPLACE INTO faces (
                id, photo_id, bbox_x, bbox_y, bbox_width, bbox_height,
                detection_confidence, landmarks, embedding,
                identity_id, identity_confidence, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                face.id,
                face.photo_id,
                face.bbox_x,
                face.bbox_y,
                face.bbox_width,
                face.bbox_height,
                face.detection_confidence,
                landmarks_blob,
                embedding_blob,
                face.identity_id,
                face.identity_confidence,
                face.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list_for_photo(conn: &Arc<Mutex<Connection>>, photo_id: &str) -> Result<Vec<Face>> {
        let conn = conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, photo_id, bbox_x, bbox_y, bbox_width, bbox_height,
                    detection_confidence, landmarks, embedding,
                    identity_id, identity_confidence, created_at
             FROM faces WHERE photo_id = ?1",
        )?;

        let faces = stmt
            .query_map(params![photo_id], Self::row_to_face)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(faces)
    }

    pub fn list_all_with_embeddings(conn: &Arc<Mutex<Connection>>) -> Result<Vec<Face>> {
        let conn = conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, photo_id, bbox_x, bbox_y, bbox_width, bbox_height,
                    detection_confidence, landmarks, embedding,
                    identity_id, identity_confidence, created_at
             FROM faces WHERE embedding IS NOT NULL",
        )?;

        let faces = stmt
            .query_map([], Self::row_to_face)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(faces)
    }

    pub fn update_identity(
        conn: &Arc<Mutex<Connection>>,
        face_id: &str,
        identity_id: &str,
        confidence: f32,
    ) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE faces SET identity_id = ?1, identity_confidence = ?2 WHERE id = ?3",
            params![identity_id, confidence, face_id],
        )?;
        Ok(())
    }

    pub fn clear_identities(conn: &Arc<Mutex<Connection>>) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE faces SET identity_id = NULL, identity_confidence = NULL",
            [],
        )?;
        Ok(())
    }

    /// Reassign all faces from one identity to another.
    pub fn reassign_identity(
        conn: &Arc<Mutex<Connection>>,
        from_identity_id: &str,
        to_identity_id: &str,
    ) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE faces SET identity_id = ?1 WHERE identity_id = ?2",
            params![to_identity_id, from_identity_id],
        )?;
        Ok(())
    }

    pub fn delete_for_photo(conn: &Arc<Mutex<Connection>>, photo_id: &str) -> Result<()> {
        let conn = conn.lock();
        conn.execute("DELETE FROM faces WHERE photo_id = ?1", params![photo_id])?;
        Ok(())
    }

    /// Get the best face (highest confidence) for each identity.
    /// Returns (identity_id, photo_id, bbox) tuples.
    pub fn best_face_per_identity(
        conn: &Arc<Mutex<Connection>>,
    ) -> Result<Vec<(String, String, [f32; 4])>> {
        let conn = conn.lock();
        let mut stmt = conn.prepare(
            "SELECT f.identity_id, f.photo_id, f.bbox_x, f.bbox_y, f.bbox_width, f.bbox_height
             FROM faces f
             INNER JOIN (
                 SELECT identity_id, MAX(detection_confidence) as max_conf
                 FROM faces
                 WHERE identity_id IS NOT NULL
                 GROUP BY identity_id
             ) best ON f.identity_id = best.identity_id AND f.detection_confidence = best.max_conf
             GROUP BY f.identity_id",
        )?;

        let results = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    [
                        row.get::<_, f32>(2)?,
                        row.get::<_, f32>(3)?,
                        row.get::<_, f32>(4)?,
                        row.get::<_, f32>(5)?,
                    ],
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    pub fn count(conn: &Arc<Mutex<Connection>>) -> Result<u32> {
        let conn = conn.lock();
        let count: u32 =
            conn.query_row("SELECT COUNT(*) FROM faces", [], |row| row.get(0))?;
        Ok(count)
    }

    fn row_to_face(row: &rusqlite::Row<'_>) -> rusqlite::Result<Face> {
        Ok(Face {
            id: row.get(0)?,
            photo_id: row.get(1)?,
            bbox_x: row.get(2)?,
            bbox_y: row.get(3)?,
            bbox_width: row.get(4)?,
            bbox_height: row.get(5)?,
            detection_confidence: row.get(6)?,
            landmarks: row
                .get::<_, Option<Vec<u8>>>(7)?
                .map(|b| bytes_to_f32_vec(&b)),
            embedding: row
                .get::<_, Option<Vec<u8>>>(8)?
                .map(|b| bytes_to_f32_vec(&b)),
            identity_id: row.get(9)?,
            identity_confidence: row.get(10)?,
            created_at: row
                .get::<_, String>(11)?
                .parse::<chrono::DateTime<Utc>>()
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}
