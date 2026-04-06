use crate::state::AppState;
use mim_core::db::{FacesDb, PhotosDb, TagsDb};
use mim_core::models::{PhotoTag, Tag, TagSource};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct TaggingProgress {
    pub total: usize,
    pub processed: usize,
    pub tagged: usize,
}

#[tauri::command]
pub async fn tag_photos(
    folder_path: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<TaggingProgress, String> {
    let _ = app_handle.emit("gemma-status", "downloading-models");

    let gemma = state
        .get_or_init_gemma_with_handle(&app_handle)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit("gemma-status", "model-loaded");

    let _ = app_handle.emit("gemma-status", "tagging");

    let db = state
        .get_or_open_sidecar(&folder_path)
        .map_err(|e| e.to_string())?;

    let root = std::path::PathBuf::from(&folder_path);
    let handle = app_handle.clone();

    let result = tokio::task::spawn_blocking(move || {
        let photos = PhotosDb::list_unprocessed_ai(db.reader())
            .map_err(|e| e.to_string())?;

        let total = photos.len();
        info!("Tagging {} unprocessed photos with Gemma", total);

        let mut progress = TaggingProgress {
            total,
            processed: 0,
            tagged: 0,
        };

        for photo in &photos {
            let source_path = root.join(&photo.relative_path);
            if !source_path.exists() {
                let _ = PhotosDb::mark_ai_processed(db.writer(), &photo.id);
                progress.processed += 1;
                continue;
            }

            match gemma.analyze_image(&source_path) {
                Ok(analysis) => {
                    // Store description
                    let _ = PhotosDb::update_ai_description(
                        db.writer(),
                        &photo.id,
                        &analysis.description,
                    );

                    // Store OCR text if extracted
                    if let Some(ref ocr) = analysis.ocr_text {
                        if !ocr.is_empty() {
                            let _ = PhotosDb::update_ocr_text(db.writer(), &photo.id, ocr);
                        }
                    }

                    // Store tags
                    for tag_name in &analysis.tags {
                        let tag = Tag {
                            id: Uuid::new_v4().to_string(),
                            name: tag_name.clone(),
                            category: None,
                        };
                        let _ = TagsDb::insert(db.writer(), &tag);
                        let photo_tag = PhotoTag {
                            photo_id: photo.id.clone(),
                            tag_id: tag.id.clone(),
                            confidence: Some(0.8),
                            source: TagSource::Gemma,
                        };
                        let _ = TagsDb::add_to_photo(db.writer(), &photo_tag);
                    }

                    progress.tagged += 1;
                }
                Err(e) => {
                    tracing::warn!("Failed to tag {}: {}", photo.filename, e);
                }
            }

            let _ = PhotosDb::mark_ai_processed(db.writer(), &photo.id);
            progress.processed += 1;
            let _ = handle.emit("gemma-tagging-progress", &progress);
        }

        info!(
            "Tagging complete: {}/{} photos tagged",
            progress.tagged, progress.total
        );

        Ok::<_, String>(progress)
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
    .map_err(|e| e)?;

    Ok(result)
}

#[tauri::command]
pub async fn chat_about_photo(
    folder_path: String,
    photo_id: String,
    question: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let gemma = state
        .get_or_init_gemma_with_handle(&app_handle)
        .await
        .map_err(|e| e.to_string())?;

    let db = state
        .get_or_open_sidecar(&folder_path)
        .map_err(|e| e.to_string())?;

    let photo = PhotosDb::get_by_id(db.reader(), &photo_id)
        .map_err(|e| e.to_string())?
        .ok_or("Photo not found")?;

    // Build rich context from all available metadata
    let mut context_parts: Vec<String> = Vec::new();

    context_parts.push(format!("Filename: {}", photo.filename));
    if let Some(ref taken) = photo.taken_at {
        context_parts.push(format!("Taken: {}", taken));
    }
    if photo.camera_make.is_some() || photo.camera_model.is_some() {
        context_parts.push(format!("Camera: {} {}",
            photo.camera_make.as_deref().unwrap_or(""),
            photo.camera_model.as_deref().unwrap_or("")
        ));
    }
    if let (Some(lat), Some(lon)) = (photo.latitude, photo.longitude) {
        let loc = photo.location_name.as_deref().unwrap_or("Unknown");
        context_parts.push(format!("Location: {} ({:.4}, {:.4})", loc, lat, lon));
    }
    if let Some(ref desc) = photo.ai_description {
        context_parts.push(format!("AI Description: {}", desc));
    }
    if let Some(ref ocr) = photo.ocr_text {
        if ocr != "[text detected]" && !ocr.is_empty() {
            context_parts.push(format!("Text in image (OCR): {}", ocr));
        }
    }
    if let Some(ref scene) = photo.scene_type {
        context_parts.push(format!("Scene: {}", scene));
    }
    if let Some(ref tod) = photo.time_of_day {
        context_parts.push(format!("Time of day: {}", tod));
    }
    if let Some(ref w) = photo.weather {
        context_parts.push(format!("Weather: {}", w));
    }
    if let Some(w) = photo.width {
        if let Some(h) = photo.height {
            context_parts.push(format!("Dimensions: {}x{}", w, h));
        }
    }

    // Get tags
    if let Ok(tags) = TagsDb::list_for_photo(db.reader(), &photo_id) {
        if !tags.is_empty() {
            let tag_names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
            context_parts.push(format!("Tags: {}", tag_names.join(", ")));
        }
    }

    // Get faces
    let central_db = state.central_db_arc();
    if let Ok(faces) = FacesDb::list_for_photo(db.reader(), &photo_id) {
        if !faces.is_empty() {
            let mut face_info = format!("{} face(s) detected", faces.len());
            let named: Vec<String> = faces.iter()
                .filter_map(|f| f.identity_id.as_ref())
                .filter_map(|id| {
                    let conn = central_db.reader().lock();
                    conn.query_row(
                        "SELECT name FROM face_identities WHERE id = ?1",
                        [id],
                        |row| row.get::<_, String>(0),
                    ).ok()
                })
                .collect();
            if !named.is_empty() {
                face_info.push_str(&format!(", identified as: {}", named.join(", ")));
            }
            context_parts.push(face_info);
        }
    }

    let context = context_parts.join("\n");

    let root = std::path::PathBuf::from(&folder_path);
    let source_path = root.join(&photo.relative_path);

    let enriched_prompt = format!(
        "You are a photo assistant. Here is metadata about this photo:\n{}\n\nUser question: {}",
        context, question
    );

    let response = tokio::task::spawn_blocking(move || {
        gemma.chat_about_image(&source_path, &enriched_prompt)
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
    .map_err(|e| e.to_string())?;

    Ok(response)
}

#[tauri::command]
pub async fn tag_single_photo(
    folder_path: String,
    photo_id: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let gemma = state
        .get_or_init_gemma_with_handle(&app_handle)
        .await
        .map_err(|e| e.to_string())?;

    let db = state.get_or_open_sidecar(&folder_path).map_err(|e| e.to_string())?;
    let photo = PhotosDb::get_by_id(db.reader(), &photo_id)
        .map_err(|e| e.to_string())?
        .ok_or("Photo not found")?;

    let root = std::path::PathBuf::from(&folder_path);
    let source_path = root.join(&photo.relative_path);
    let pid = photo.id.clone();

    let result = tokio::task::spawn_blocking(move || {
        gemma.analyze_image(&source_path)
    })
    .await
    .map_err(|e| format!("task join: {e}"))?
    .map_err(|e| e.to_string())?;

    // Store description
    let _ = PhotosDb::update_ai_description(db.writer(), &pid, &result.description);

    // Store OCR
    if let Some(ref ocr) = result.ocr_text {
        if !ocr.is_empty() {
            let _ = PhotosDb::update_ocr_text(db.writer(), &pid, ocr);
        }
    }

    // Store tags
    for tag_name in &result.tags {
        let tag = Tag { id: Uuid::new_v4().to_string(), name: tag_name.clone(), category: None };
        let _ = TagsDb::insert(db.writer(), &tag);
        let photo_tag = PhotoTag {
            photo_id: pid.clone(), tag_id: tag.id.clone(),
            confidence: Some(0.8), source: TagSource::Gemma,
        };
        let _ = TagsDb::add_to_photo(db.writer(), &photo_tag);
    }

    let _ = PhotosDb::mark_ai_processed(db.writer(), &pid);
    Ok(result.description)
}
