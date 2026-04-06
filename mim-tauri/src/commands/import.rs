use crate::state::AppState;
use mim_core::db::PhotosDb;
use mim_core::models::Photo;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager, State};
use tracing::{info, warn};
use walkdir::WalkDir;

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp",
    "heic", "heif", "avif", "jxl", "cr2", "nef", "arw", "dng",
    "orf", "rw2", "raf", "pef", "srw",
];

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mkv", "webm", "m4v", "3gp",
];

fn is_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext = ext.to_lowercase();
            IMAGE_EXTENSIONS.contains(&ext.as_str()) || VIDEO_EXTENSIONS.contains(&ext.as_str())
        })
        .unwrap_or(false)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoogleTakeoutMeta {
    title: Option<String>,
    description: Option<String>,
    #[serde(rename = "imageViews")]
    image_views: Option<String>,
    #[serde(rename = "creationTime")]
    creation_time: Option<GoogleTimestamp>,
    #[serde(rename = "photoTakenTime")]
    photo_taken_time: Option<GoogleTimestamp>,
    #[serde(rename = "geoData")]
    geo_data: Option<GoogleGeoData>,
    #[serde(rename = "geoDataExif")]
    geo_data_exif: Option<GoogleGeoData>,
    people: Option<Vec<GooglePerson>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoogleTimestamp {
    timestamp: Option<String>,
    formatted: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoogleGeoData {
    latitude: Option<f64>,
    longitude: Option<f64>,
    altitude: Option<f64>,
    #[serde(rename = "latitudeSpan")]
    latitude_span: Option<f64>,
    #[serde(rename = "longitudeSpan")]
    longitude_span: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GooglePerson {
    name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub total_found: usize,
    pub imported: usize,
    pub skipped: usize,
    pub errors: usize,
    pub with_metadata: usize,
}

#[tauri::command]
pub async fn import_google_takeout(
    source_path: String,
    dest_folder_path: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<ImportResult, String> {
    let source = Path::new(&source_path);
    let dest = Path::new(&dest_folder_path);

    if !source.exists() {
        return Err("Source path does not exist".to_string());
    }

    // Ensure destination exists
    std::fs::create_dir_all(dest).map_err(|e| format!("Create dest dir: {e}"))?;

    // Allow asset protocol for the dest folder
    let _ = app_handle
        .asset_protocol_scope()
        .allow_directory(&dest_folder_path, true);

    // Open sidecar DB for dest folder
    let db = state
        .get_or_open_sidecar(&dest_folder_path)
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit(
        "import-progress",
        serde_json::json!({ "stage": "scanning", "source": source_path }),
    );

    // Discover all media files in the source
    let media_files: Vec<_> = WalkDir::new(source)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.')
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && is_media_file(e.path()))
        .map(|e| e.into_path())
        .collect();

    let total_found = media_files.len();
    info!(
        "Google Takeout import: found {} media files in {}",
        total_found,
        source.display()
    );

    let mut imported = 0;
    let mut skipped = 0;
    let mut errors = 0;
    let mut with_metadata = 0;

    for (i, file_path) in media_files.iter().enumerate() {
        let filename = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Skip companion .json files (they aren't media)
        if filename.ends_with(".json") {
            continue;
        }

        // Destination file path
        let dest_file = dest.join(&filename);

        // Skip if already exists in dest
        if dest_file.exists() {
            skipped += 1;
            continue;
        }

        // Copy the file
        if let Err(e) = std::fs::copy(file_path, &dest_file) {
            warn!("Failed to copy {}: {}", filename, e);
            errors += 1;
            continue;
        }

        // Get file metadata
        let file_meta = match std::fs::metadata(&dest_file) {
            Ok(m) => m,
            Err(e) => {
                warn!("Failed to read metadata for {}: {}", filename, e);
                errors += 1;
                continue;
            }
        };
        let file_size = file_meta.len();
        let hash = format!("{:x}", md5_lite(&dest_file));

        let mut photo = Photo::new(filename.clone(), filename.clone(), file_size, hash);

        // Determine media type
        if let Some(ext) = dest_file.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            if VIDEO_EXTENSIONS.contains(&ext_lower.as_str()) {
                photo.media_type = mim_core::models::MediaType::Video;
            }
            photo.format = Some(ext_lower);
        }

        // Dimensions will be detected during the scan/thumbnail phase

        // Look for companion .json metadata
        let json_candidates = vec![
            file_path.with_extension(format!(
                "{}.json",
                file_path.extension().unwrap_or_default().to_string_lossy()
            )),
            file_path.parent().unwrap_or(Path::new(".")).join(format!("{}.json", file_path.file_name().unwrap_or_default().to_string_lossy())),
        ];

        let mut found_meta = false;
        for json_path in &json_candidates {
            if json_path.exists() {
                if let Ok(json_str) = std::fs::read_to_string(json_path) {
                    if let Ok(meta) = serde_json::from_str::<GoogleTakeoutMeta>(&json_str) {
                        apply_takeout_metadata(&mut photo, &meta);
                        found_meta = true;
                        with_metadata += 1;
                        break;
                    }
                }
            }
        }

        // Try EXIF as fallback using apply_exif from the scanner metadata module
        if !found_meta {
            // apply_exif is not directly exposed but we can use extract_exif + manual apply
            // Just skip EXIF for Google Takeout — the JSON metadata is the primary source
        }

        // Insert into DB
        if let Err(e) = PhotosDb::insert(db.writer(), &photo) {
            warn!("Failed to insert photo {}: {}", filename, e);
            errors += 1;
            continue;
        }

        imported += 1;

        // Emit progress periodically
        if i % 10 == 0 || i == total_found - 1 {
            let _ = app_handle.emit(
                "import-progress",
                serde_json::json!({
                    "stage": "importing",
                    "current": i + 1,
                    "total": total_found,
                    "imported": imported,
                }),
            );
        }
    }

    let _ = app_handle.emit(
        "import-progress",
        serde_json::json!({
            "stage": "done",
            "imported": imported,
            "total": total_found,
        }),
    );

    info!(
        "Google Takeout import complete: {} imported, {} skipped, {} errors, {} with metadata",
        imported, skipped, errors, with_metadata
    );

    Ok(ImportResult {
        total_found,
        imported,
        skipped,
        errors,
        with_metadata,
    })
}

fn apply_takeout_metadata(photo: &mut Photo, meta: &GoogleTakeoutMeta) {
    // Description
    if let Some(ref desc) = meta.description {
        if !desc.is_empty() {
            photo.ai_description = Some(desc.clone());
        }
    }

    // Photo taken time
    if let Some(ref taken) = meta.photo_taken_time {
        if let Some(ref ts) = taken.timestamp {
            if let Ok(secs) = ts.parse::<i64>() {
                if let Some(dt) = chrono::DateTime::from_timestamp(secs, 0) {
                    photo.taken_at = Some(dt);
                }
            }
        }
    } else if let Some(ref created) = meta.creation_time {
        if let Some(ref ts) = created.timestamp {
            if let Ok(secs) = ts.parse::<i64>() {
                if let Some(dt) = chrono::DateTime::from_timestamp(secs, 0) {
                    photo.taken_at = Some(dt);
                }
            }
        }
    }

    // Geo data (prefer EXIF geo, fall back to Google geo)
    let geo = meta.geo_data_exif.as_ref().or(meta.geo_data.as_ref());
    if let Some(geo) = geo {
        if let Some(lat) = geo.latitude {
            if lat.abs() > 0.001 {
                photo.latitude = Some(lat);
            }
        }
        if let Some(lng) = geo.longitude {
            if lng.abs() > 0.001 {
                photo.longitude = Some(lng);
            }
        }
        if let Some(alt) = geo.altitude {
            photo.altitude = Some(alt);
        }
    }
}

/// Quick hash for dedup — uses first+last 64KB + file size.
fn md5_lite(path: &Path) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::io::{Read, Seek, SeekFrom};

    let mut hasher = DefaultHasher::new();
    if let Ok(meta) = std::fs::metadata(path) {
        meta.len().hash(&mut hasher);
        if let Ok(mut file) = std::fs::File::open(path) {
            let mut buf = [0u8; 65536];
            if let Ok(n) = file.read(&mut buf) {
                buf[..n].hash(&mut hasher);
            }
            if meta.len() > 131072 {
                if file.seek(SeekFrom::End(-65536)).is_ok() {
                    if let Ok(n) = file.read(&mut buf) {
                        buf[..n].hash(&mut hasher);
                    }
                }
            }
        }
    }
    hasher.finish()
}
