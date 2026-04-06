use crate::state::AppState;
use mim_core::db::{PhotosDb, TagsDb};
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

    let root = std::path::PathBuf::from(&folder_path);
    let source_path = root.join(&photo.relative_path);

    let response = tokio::task::spawn_blocking(move || {
        gemma.chat_about_image(&source_path, &question)
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
    .map_err(|e| e.to_string())?;

    Ok(response)
}
