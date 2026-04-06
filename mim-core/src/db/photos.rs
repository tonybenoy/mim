use crate::models::Photo;
use crate::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection, Row};
use std::sync::Arc;

pub const SELECT_COLUMNS: &str = "
    id, relative_path, filename, file_size, content_hash,
    width, height, format, media_type,
    taken_at, camera_make, camera_model, lens_model,
    focal_length, aperture, shutter_speed, iso,
    latitude, longitude, altitude, location_name,
    ai_description, ai_processed_at,
    aesthetic_score, blur_score, scene_type, dominant_colors,
    perceptual_hash, is_screenshot, is_nsfw, ocr_text,
    weather, time_of_day, event_id, analysis_processed,
    rating, is_favorite, is_trashed, trashed_at,
    thumbnail_generated, faces_processed, ai_processed,
    file_modified_at, created_at, updated_at
";

pub fn photo_from_row(row: &Row<'_>) -> rusqlite::Result<Photo> {
    Ok(Photo {
        id: row.get(0)?,
        relative_path: row.get(1)?,
        filename: row.get(2)?,
        file_size: row.get::<_, i64>(3)? as u64,
        content_hash: row.get(4)?,
        width: row.get(5)?,
        height: row.get(6)?,
        format: row.get(7)?,
        media_type: {
            let s: String = row.get(8)?;
            if s == "video" { crate::models::MediaType::Video } else { crate::models::MediaType::Photo }
        },
        taken_at: row.get::<_, Option<String>>(9)?
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        camera_make: row.get(10)?,
        camera_model: row.get(11)?,
        lens_model: row.get(12)?,
        focal_length: row.get(13)?,
        aperture: row.get(14)?,
        shutter_speed: row.get(15)?,
        iso: row.get(16)?,
        latitude: row.get(17)?,
        longitude: row.get(18)?,
        altitude: row.get(19)?,
        location_name: row.get(20)?,
        ai_description: row.get(21)?,
        ai_processed_at: row.get::<_, Option<String>>(22)?
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        aesthetic_score: row.get(23)?,
        blur_score: row.get(24)?,
        scene_type: row.get(25)?,
        dominant_colors: row.get(26)?,
        perceptual_hash: row.get(27)?,
        is_screenshot: row.get(28)?,
        is_nsfw: row.get(29)?,
        ocr_text: row.get(30)?,
        weather: row.get(31)?,
        time_of_day: row.get(32)?,
        event_id: row.get(33)?,
        analysis_processed: row.get(34)?,
        rating: row.get::<_, u8>(35).unwrap_or(0),
        is_favorite: row.get(36).unwrap_or(false),
        is_trashed: row.get(37).unwrap_or(false),
        trashed_at: row.get::<_, Option<String>>(38)?
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        thumbnail_generated: row.get(39)?,
        faces_processed: row.get(40)?,
        ai_processed: row.get(41)?,
        file_modified_at: row.get::<_, String>(42)?
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap_or_else(|_| chrono::Utc::now()),
        created_at: row.get::<_, String>(43)?
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap_or_else(|_| chrono::Utc::now()),
        updated_at: row.get::<_, String>(44)?
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}

pub struct PhotosDb;

impl PhotosDb {
    pub fn insert(conn: &Arc<Mutex<Connection>>, photo: &Photo) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO photos (
                id, relative_path, filename, file_size, content_hash,
                width, height, format, media_type,
                taken_at, camera_make, camera_model, lens_model,
                focal_length, aperture, shutter_speed, iso,
                latitude, longitude, altitude, location_name,
                ai_description, ai_processed_at,
                aesthetic_score, blur_score, scene_type, dominant_colors,
                perceptual_hash, is_screenshot, is_nsfw, ocr_text,
                weather, time_of_day, event_id, analysis_processed,
                rating, is_favorite, is_trashed, trashed_at,
                thumbnail_generated, faces_processed, ai_processed,
                file_modified_at, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9,
                ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21,
                ?22, ?23,
                ?24, ?25, ?26, ?27,
                ?28, ?29, ?30, ?31,
                ?32, ?33, ?34, ?35,
                ?36, ?37, ?38, ?39,
                ?40, ?41, ?42,
                ?43, ?44, ?45
            )",
            params![
                photo.id,
                photo.relative_path,
                photo.filename,
                photo.file_size as i64,
                photo.content_hash,
                photo.width,
                photo.height,
                photo.format,
                format!("{:?}", photo.media_type).to_lowercase(),
                photo.taken_at.map(|t| t.to_rfc3339()),
                photo.camera_make,
                photo.camera_model,
                photo.lens_model,
                photo.focal_length,
                photo.aperture,
                photo.shutter_speed,
                photo.iso,
                photo.latitude,
                photo.longitude,
                photo.altitude,
                photo.location_name,
                photo.ai_description,
                photo.ai_processed_at.map(|t| t.to_rfc3339()),
                photo.aesthetic_score,
                photo.blur_score,
                photo.scene_type,
                photo.dominant_colors,
                photo.perceptual_hash,
                photo.is_screenshot,
                photo.is_nsfw,
                photo.ocr_text,
                photo.weather,
                photo.time_of_day,
                photo.event_id,
                photo.analysis_processed,
                photo.rating,
                photo.is_favorite,
                photo.is_trashed,
                photo.trashed_at.map(|t| t.to_rfc3339()),
                photo.thumbnail_generated,
                photo.faces_processed,
                photo.ai_processed,
                photo.file_modified_at.to_rfc3339(),
                photo.created_at.to_rfc3339(),
                photo.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list(conn: &Arc<Mutex<Connection>>, limit: u32, offset: u32) -> Result<Vec<Photo>> {
        let conn = conn.lock();
        let sql = format!(
            "SELECT {} FROM photos WHERE is_trashed = 0 ORDER BY COALESCE(taken_at, created_at) DESC LIMIT ?1 OFFSET ?2",
            SELECT_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;
        let photos = stmt.query_map(params![limit, offset], photo_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(photos)
    }

    pub fn get_by_id(conn: &Arc<Mutex<Connection>>, id: &str) -> Result<Option<Photo>> {
        let conn = conn.lock();
        let sql = format!("SELECT {} FROM photos WHERE id = ?1", SELECT_COLUMNS);
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query_map(params![id], photo_from_row)?;
        match rows.next() {
            Some(Ok(photo)) => Ok(Some(photo)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    pub fn count(conn: &Arc<Mutex<Connection>>) -> Result<u32> {
        let conn = conn.lock();
        let count: u32 = conn.query_row("SELECT COUNT(*) FROM photos WHERE is_trashed = 0", [], |row| row.get(0))?;
        Ok(count)
    }

    pub fn exists_by_path(conn: &Arc<Mutex<Connection>>, relative_path: &str) -> Result<bool> {
        let conn = conn.lock();
        let mut stmt = conn.prepare("SELECT 1 FROM photos WHERE relative_path = ?1")?;
        Ok(stmt.exists(params![relative_path])?)
    }

    pub fn list_unprocessed_faces(conn: &Arc<Mutex<Connection>>) -> Result<Vec<Photo>> {
        let conn = conn.lock();
        let sql = format!(
            "SELECT {} FROM photos WHERE faces_processed = 0 AND media_type = 'photo' AND is_trashed = 0 ORDER BY COALESCE(taken_at, created_at) DESC",
            SELECT_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;
        let photos = stmt.query_map([], photo_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(photos)
    }

    pub fn list_unprocessed_ai(conn: &Arc<Mutex<Connection>>) -> Result<Vec<Photo>> {
        let conn = conn.lock();
        let sql = format!(
            "SELECT {} FROM photos WHERE ai_processed = 0 AND media_type = 'photo' AND is_trashed = 0 ORDER BY COALESCE(taken_at, created_at) DESC",
            SELECT_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;
        let photos = stmt.query_map([], photo_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(photos)
    }

    pub fn list_unprocessed_analysis(conn: &Arc<Mutex<Connection>>) -> Result<Vec<Photo>> {
        let conn = conn.lock();
        let sql = format!(
            "SELECT {} FROM photos WHERE analysis_processed = 0 AND media_type = 'photo' AND is_trashed = 0 ORDER BY COALESCE(taken_at, created_at) DESC",
            SELECT_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;
        let photos = stmt.query_map([], photo_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(photos)
    }

    pub fn update_analysis(
        conn: &Arc<Mutex<Connection>>,
        photo_id: &str,
        blur_score: Option<f64>,
        dominant_colors: Option<&str>,
        perceptual_hash: Option<&str>,
        is_screenshot: bool,
        time_of_day: Option<&str>,
    ) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE photos SET
                blur_score = ?1,
                dominant_colors = ?2,
                perceptual_hash = ?3,
                is_screenshot = ?4,
                time_of_day = ?5,
                analysis_processed = 1,
                updated_at = ?6
             WHERE id = ?7",
            params![
                blur_score,
                dominant_colors,
                perceptual_hash,
                is_screenshot,
                time_of_day,
                chrono::Utc::now().to_rfc3339(),
                photo_id,
            ],
        )?;
        Ok(())
    }

    pub fn update_event_id(conn: &Arc<Mutex<Connection>>, photo_id: &str, event_id: &str) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE photos SET event_id = ?1, updated_at = ?2 WHERE id = ?3",
            params![event_id, chrono::Utc::now().to_rfc3339(), photo_id],
        )?;
        Ok(())
    }

    pub fn update_location_name(conn: &Arc<Mutex<Connection>>, photo_id: &str, name: &str) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE photos SET location_name = ?1, updated_at = ?2 WHERE id = ?3",
            params![name, chrono::Utc::now().to_rfc3339(), photo_id],
        )?;
        Ok(())
    }

    pub fn find_similar_by_phash(conn: &Arc<Mutex<Connection>>, hash: &str, threshold: u32) -> Result<Vec<Photo>> {
        // Load all photos that have a perceptual hash, then filter by hamming distance.
        // SQLite doesn't have a built-in hamming distance function, so we do it in Rust.
        let conn = conn.lock();
        let sql = format!(
            "SELECT {} FROM photos WHERE perceptual_hash IS NOT NULL",
            SELECT_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;
        let photos = stmt.query_map([], photo_from_row)?
            .filter_map(|r| r.ok())
            .filter(|p| {
                if let Some(ref ph) = p.perceptual_hash {
                    hamming_distance_hex(hash, ph) <= threshold
                } else {
                    false
                }
            })
            .collect();
        Ok(photos)
    }

    pub fn mark_ai_processed(conn: &Arc<Mutex<Connection>>, photo_id: &str) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE photos SET ai_processed = 1, ai_processed_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![chrono::Utc::now().to_rfc3339(), photo_id],
        )?;
        Ok(())
    }

    pub fn update_ai_description(conn: &Arc<Mutex<Connection>>, photo_id: &str, description: &str) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE photos SET ai_description = ?1, updated_at = ?2 WHERE id = ?3",
            params![description, chrono::Utc::now().to_rfc3339(), photo_id],
        )?;
        Ok(())
    }

    pub fn mark_thumbnail_generated(conn: &Arc<Mutex<Connection>>, photo_id: &str) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE photos SET thumbnail_generated = 1, updated_at = ?1 WHERE id = ?2",
            params![chrono::Utc::now().to_rfc3339(), photo_id],
        )?;
        Ok(())
    }

    pub fn mark_faces_processed(conn: &Arc<Mutex<Connection>>, photo_id: &str) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE photos SET faces_processed = 1, updated_at = ?1 WHERE id = ?2",
            params![chrono::Utc::now().to_rfc3339(), photo_id],
        )?;
        Ok(())
    }

    // ── Favorites & Rating ──────────────────────────────────

    pub fn toggle_favorite(conn: &Arc<Mutex<Connection>>, photo_id: &str) -> Result<bool> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE photos SET is_favorite = CASE WHEN is_favorite = 1 THEN 0 ELSE 1 END, updated_at = ?1 WHERE id = ?2",
            params![chrono::Utc::now().to_rfc3339(), photo_id],
        )?;
        let val: bool = conn.query_row(
            "SELECT is_favorite FROM photos WHERE id = ?1",
            params![photo_id],
            |row| row.get(0),
        )?;
        Ok(val)
    }

    pub fn set_rating(conn: &Arc<Mutex<Connection>>, photo_id: &str, rating: u8) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE photos SET rating = ?1, updated_at = ?2 WHERE id = ?3",
            params![rating.min(5), chrono::Utc::now().to_rfc3339(), photo_id],
        )?;
        Ok(())
    }

    // ── Trash ───────────────────────────────────────────────

    pub fn trash_photo(conn: &Arc<Mutex<Connection>>, photo_id: &str) -> Result<()> {
        let conn = conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE photos SET is_trashed = 1, trashed_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, photo_id],
        )?;
        Ok(())
    }

    pub fn restore_photo(conn: &Arc<Mutex<Connection>>, photo_id: &str) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "UPDATE photos SET is_trashed = 0, trashed_at = NULL, updated_at = ?1 WHERE id = ?2",
            params![chrono::Utc::now().to_rfc3339(), photo_id],
        )?;
        Ok(())
    }

    pub fn empty_trash(conn: &Arc<Mutex<Connection>>) -> Result<u32> {
        let conn = conn.lock();
        let count: u32 = conn.query_row(
            "SELECT COUNT(*) FROM photos WHERE is_trashed = 1",
            [],
            |row| row.get(0),
        )?;
        conn.execute("DELETE FROM photos WHERE is_trashed = 1", [])?;
        Ok(count)
    }

    pub fn list_trashed(conn: &Arc<Mutex<Connection>>) -> Result<Vec<Photo>> {
        let conn = conn.lock();
        let sql = format!(
            "SELECT {} FROM photos WHERE is_trashed = 1 ORDER BY trashed_at DESC",
            SELECT_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;
        let photos = stmt.query_map([], photo_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(photos)
    }

    // ── Memories (On This Day) ──────────────────────────────

    pub fn get_memories(conn: &Arc<Mutex<Connection>>) -> Result<Vec<Photo>> {
        let conn = conn.lock();
        let now = chrono::Utc::now();
        let month = now.format("%m").to_string();
        let day = now.format("%d").to_string();
        let this_year = now.format("%Y").to_string();
        let sql = format!(
            "SELECT {} FROM photos
             WHERE is_trashed = 0
               AND taken_at IS NOT NULL
               AND strftime('%m', taken_at) = ?1
               AND strftime('%d', taken_at) = ?2
               AND strftime('%Y', taken_at) != ?3
             ORDER BY taken_at ASC",
            SELECT_COLUMNS
        );
        let mut stmt = conn.prepare(&sql)?;
        let photos = stmt.query_map(params![month, day, this_year], photo_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(photos)
    }

    // ── Storage Stats ───────────────────────────────────────

    pub fn get_total_photo_size(conn: &Arc<Mutex<Connection>>) -> Result<u64> {
        let conn = conn.lock();
        let size: i64 = conn.query_row(
            "SELECT COALESCE(SUM(file_size), 0) FROM photos WHERE is_trashed = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(size as u64)
    }

    pub fn count_all(conn: &Arc<Mutex<Connection>>) -> Result<u32> {
        let conn = conn.lock();
        let count: u32 = conn.query_row("SELECT COUNT(*) FROM photos", [], |row| row.get(0))?;
        Ok(count)
    }

    // ── Smart Albums ────────────────────────────────────────

    pub fn query_smart_album(conn: &Arc<Mutex<Connection>>, rules_json: &str) -> Result<Vec<Photo>> {
        let conn = conn.lock();
        let rules: serde_json::Value = serde_json::from_str(rules_json)
            .map_err(|e| crate::Error::Other(format!("Invalid rules JSON: {}", e)))?;

        let mut conditions = vec!["is_trashed = 0".to_string()];
        let mut params_list: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(min_rating) = rules.get("min_rating").and_then(|v| v.as_i64()) {
            conditions.push(format!("rating >= ?{}", param_idx));
            params_list.push(Box::new(min_rating as i32));
            param_idx += 1;
        }

        if let Some(true) = rules.get("favorites_only").and_then(|v| v.as_bool()) {
            conditions.push("is_favorite = 1".to_string());
        }

        if let Some(media) = rules.get("media_type").and_then(|v| v.as_str()) {
            conditions.push(format!("media_type = ?{}", param_idx));
            params_list.push(Box::new(media.to_string()));
            param_idx += 1;
        }

        // Tag-based filtering via subquery
        if let Some(tags) = rules.get("tags").and_then(|v| v.as_array()) {
            for tag in tags {
                if let Some(tag_name) = tag.as_str() {
                    conditions.push(format!(
                        "id IN (SELECT pt.photo_id FROM photo_tags pt JOIN tags t ON pt.tag_id = t.id WHERE t.name LIKE ?{})",
                        param_idx
                    ));
                    params_list.push(Box::new(format!("%{}%", tag_name)));
                    param_idx += 1;
                }
            }
        }

        let _ = param_idx; // suppress unused warning

        let where_clause = conditions.join(" AND ");
        let sql = format!(
            "SELECT {} FROM photos WHERE {} ORDER BY COALESCE(taken_at, created_at) DESC LIMIT 1000",
            SELECT_COLUMNS, where_clause
        );

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_list.iter().map(|p| p.as_ref()).collect();
        let photos = stmt.query_map(param_refs.as_slice(), photo_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(photos)
    }
}

/// Compute hamming distance between two hex-encoded hashes.
pub fn hamming_distance_hex(a: &str, b: &str) -> u32 {
    let a_bytes = u64::from_str_radix(a, 16).unwrap_or(0);
    let b_bytes = u64::from_str_radix(b, 16).unwrap_or(0);
    (a_bytes ^ b_bytes).count_ones()
}
