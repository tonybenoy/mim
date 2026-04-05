mod metadata;

pub use metadata::extract_exif;

use crate::db::{DbPool, PhotosDb};
use crate::models::Photo;
use crate::{Error, Result};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use walkdir::WalkDir;

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp",
    "heic", "heif", "avif", "jxl", "qoi", "svg",
    "cr2", "nef", "arw", "dng", "orf", "rw2", "raf", "pef", "srw",
    "ico", "tga", "pnm", "exr",
];

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mkv", "webm", "m4v", "3gp",
];

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub total_found: usize,
    pub new_photos: usize,
    pub skipped: usize,
    pub errors: usize,
}

pub struct Scanner;

impl Scanner {
    pub fn discover_files(root: &Path) -> Vec<PathBuf> {
        WalkDir::new(root)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != ".mim"
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| {
                        let ext = ext.to_lowercase();
                        IMAGE_EXTENSIONS.contains(&ext.as_str())
                            || VIDEO_EXTENSIONS.contains(&ext.as_str())
                    })
                    .unwrap_or(false)
            })
            .map(|e| e.into_path())
            .collect()
    }

    pub fn scan_folder(root: &Path, db: &DbPool) -> Result<ScanResult> {
        let files = Self::discover_files(root);
        let total_found = files.len();
        info!("Discovered {} media files in {}", total_found, root.display());

        let results: Vec<Result<Option<Photo>>> = files
            .par_iter()
            .map(|path| {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| Error::Other("failed to compute relative path".into()))?
                    .to_string_lossy()
                    .to_string();

                if PhotosDb::exists_by_path(db.reader(), &relative)? {
                    return Ok(None);
                }

                let filename = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let file_meta = std::fs::metadata(path)?;
                let file_size = file_meta.len();

                let hash = hash_file(path)?;

                let mut photo = Photo::new(relative, filename, file_size, hash);

                // Determine media type
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext = ext.to_lowercase();
                    if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
                        photo.media_type = crate::models::MediaType::Video;
                    }
                    photo.format = Some(ext);
                }

                // Extract EXIF for images
                if photo.media_type == crate::models::MediaType::Photo {
                    if let Err(e) = metadata::apply_exif(path, &mut photo) {
                        warn!("EXIF extraction failed for {}: {}", path.display(), e);
                    }
                }

                // Get image dimensions
                if photo.media_type == crate::models::MediaType::Photo {
                    if let Ok(dims) = image::image_dimensions(path) {
                        photo.width = Some(dims.0);
                        photo.height = Some(dims.1);
                    }
                }

                Ok(Some(photo))
            })
            .collect();

        let mut new_photos = 0;
        let mut skipped = 0;
        let mut errors = 0;

        for result in results {
            match result {
                Ok(Some(photo)) => {
                    if let Err(e) = PhotosDb::insert(db.writer(), &photo) {
                        warn!("Failed to insert photo: {}", e);
                        errors += 1;
                    } else {
                        new_photos += 1;
                    }
                }
                Ok(None) => skipped += 1,
                Err(e) => {
                    warn!("Error processing file: {}", e);
                    errors += 1;
                }
            }
        }

        info!("Scan complete: {} new, {} skipped, {} errors", new_photos, skipped, errors);
        Ok(ScanResult { total_found, new_photos, skipped, errors })
    }
}

fn hash_file(path: &Path) -> Result<String> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::with_capacity(1024 * 1024, file);
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}
