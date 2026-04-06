mod metadata;

pub use metadata::extract_exif;

use crate::db::{DbPool, PhotosDb};
use crate::models::Photo;
use crate::{Error, Result};
use rayon::prelude::*;
use std::collections::HashSet;
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

        // Pre-load existing paths into a HashSet to avoid per-file DB queries
        let existing_paths: HashSet<String> = {
            let conn = db.reader().lock();
            let mut stmt = conn.prepare("SELECT relative_path FROM photos")?;
            stmt.query_map([], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect()
        };

        let results: Vec<Result<Option<Photo>>> = files
            .par_iter()
            .map(|path| {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| Error::Other("failed to compute relative path".into()))?
                    .to_string_lossy()
                    .to_string();

                // Fast in-memory check instead of DB query per file
                if existing_paths.contains(&relative) {
                    return Ok(None);
                }

                let filename = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let file_meta = std::fs::metadata(path)?;
                let file_size = file_meta.len();

                // Fast hash: only hash first 64KB + last 64KB + file size for large files
                let hash = hash_file_fast(path, file_size)?;

                let mut photo = Photo::new(relative, filename, file_size, hash);

                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext = ext.to_lowercase();
                    if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
                        photo.media_type = crate::models::MediaType::Video;
                    }
                    photo.format = Some(ext);
                }

                // Extract EXIF (also gets dimensions) — single file read
                if photo.media_type == crate::models::MediaType::Photo {
                    if let Err(e) = metadata::apply_exif(path, &mut photo) {
                        warn!("EXIF extraction failed for {}: {}", path.display(), e);
                    }

                    // Only read dimensions if EXIF didn't provide them
                    if photo.width.is_none() {
                        if let Ok(dims) = image::image_dimensions(path) {
                            photo.width = Some(dims.0);
                            photo.height = Some(dims.1);
                        }
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

        // Prune stale DB entries — files that no longer exist on disk
        let on_disk: HashSet<String> = files.iter()
            .filter_map(|p| p.strip_prefix(root).ok())
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        let mut pruned = 0;
        for existing in &existing_paths {
            if !on_disk.contains(existing) {
                let conn = db.writer().lock();
                if conn.execute("DELETE FROM photos WHERE relative_path = ?1", [existing]).is_ok() {
                    pruned += 1;
                }
            }
        }
        if pruned > 0 {
            info!("Pruned {} stale DB entries (files no longer on disk)", pruned);
        }

        info!("Scan complete: {} new, {} skipped, {} errors, {} pruned", new_photos, skipped, errors, pruned);
        Ok(ScanResult { total_found, new_photos, skipped, errors })
    }
}

/// Fast content hash: for files > 128KB, hash first 64KB + last 64KB + file size.
/// This is ~100x faster than full-file hashing for large photos while still catching
/// nearly all duplicates. Collisions are astronomically unlikely because two different
/// images would need identical headers, trailers, AND file sizes.
fn hash_file_fast(path: &Path, file_size: u64) -> Result<String> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();

    // Include file size in hash to differentiate truncated files
    hasher.update(&file_size.to_le_bytes());

    const CHUNK: usize = 65536; // 64KB

    if file_size <= (CHUNK * 2) as u64 {
        // Small file: hash everything
        let mut buf = vec![0u8; file_size as usize];
        file.read_exact(&mut buf)?;
        hasher.update(&buf);
    } else {
        // Large file: hash head + tail
        let mut buf = [0u8; CHUNK];

        // Head
        file.read_exact(&mut buf)?;
        hasher.update(&buf);

        // Tail
        file.seek(SeekFrom::End(-(CHUNK as i64)))?;
        file.read_exact(&mut buf)?;
        hasher.update(&buf);
    }

    Ok(hasher.finalize().to_hex().to_string())
}
