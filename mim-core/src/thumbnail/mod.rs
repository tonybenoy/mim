use crate::Result;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView};
use std::path::{Path, PathBuf};
use tracing::warn;

#[derive(Debug, Clone, Copy)]
pub enum ThumbnailSize {
    Micro = 64,
    Grid = 256,
    Preview = 1024,
}

impl ThumbnailSize {
    pub fn suffix(&self) -> &str {
        match self {
            ThumbnailSize::Micro => "64",
            ThumbnailSize::Grid => "256",
            ThumbnailSize::Preview => "1024",
        }
    }

    pub fn all() -> &'static [ThumbnailSize] {
        &[ThumbnailSize::Micro, ThumbnailSize::Grid, ThumbnailSize::Preview]
    }
}

pub struct ThumbnailGenerator;

impl ThumbnailGenerator {
    pub fn thumbnail_path(cache_dir: &Path, content_hash: &str, size: ThumbnailSize) -> PathBuf {
        let prefix = &content_hash[..2.min(content_hash.len())];
        cache_dir
            .join(prefix)
            .join(format!("{}_{}.webp", content_hash, size.suffix()))
    }

    pub fn exists(cache_dir: &Path, content_hash: &str, size: ThumbnailSize) -> bool {
        Self::thumbnail_path(cache_dir, content_hash, size).exists()
    }

    pub fn generate(
        source_path: &Path,
        cache_dir: &Path,
        content_hash: &str,
        sizes: &[ThumbnailSize],
    ) -> Result<Vec<PathBuf>> {
        let img = image::open(source_path)?;
        let mut paths = Vec::new();

        for &size in sizes {
            let path = Self::thumbnail_path(cache_dir, content_hash, size);
            if path.exists() {
                paths.push(path);
                continue;
            }

            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let dim = size as u32;
            let thumb = resize_contain(&img, dim);

            // Save as WebP via image crate (it supports WebP encoding)
            if let Err(e) = thumb.save(&path) {
                warn!("Failed to save thumbnail {}: {}", path.display(), e);
                // Fallback: save as PNG
                let png_path = path.with_extension("png");
                thumb.save(&png_path)?;
                paths.push(png_path);
                continue;
            }

            paths.push(path);
        }

        Ok(paths)
    }

    pub fn generate_all(
        source_path: &Path,
        cache_dir: &Path,
        content_hash: &str,
    ) -> Result<Vec<PathBuf>> {
        Self::generate(source_path, cache_dir, content_hash, ThumbnailSize::all())
    }
}

/// Generate a square face crop thumbnail from a bounding box.
pub fn generate_face_crop(
    source_path: &Path,
    cache_dir: &Path,
    face_id: &str,
    bbox_x: f32,
    bbox_y: f32,
    bbox_w: f32,
    bbox_h: f32,
) -> Result<PathBuf> {
    let out_path = cache_dir.join("faces").join(format!("{}.webp", face_id));
    if out_path.exists() {
        return Ok(out_path);
    }

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let img = image::open(source_path)?;
    let (img_w, img_h) = img.dimensions();

    // Expand bbox to square with 30% padding
    let cx = bbox_x + bbox_w / 2.0;
    let cy = bbox_y + bbox_h / 2.0;
    let size = bbox_w.max(bbox_h) * 1.6;
    let half = size / 2.0;

    let x1 = (cx - half).max(0.0) as u32;
    let y1 = (cy - half).max(0.0) as u32;
    let x2 = ((cx + half) as u32).min(img_w);
    let y2 = ((cy + half) as u32).min(img_h);

    let crop_w = x2.saturating_sub(x1).max(1);
    let crop_h = y2.saturating_sub(y1).max(1);

    let cropped = img.crop_imm(x1, y1, crop_w, crop_h);
    let resized = cropped.resize_exact(128, 128, FilterType::Lanczos3);

    if let Err(_) = resized.save(&out_path) {
        let png_path = out_path.with_extension("png");
        resized.save(&png_path)?;
        return Ok(png_path);
    }

    Ok(out_path)
}

fn resize_contain(img: &DynamicImage, max_dim: u32) -> DynamicImage {
    let (w, h) = img.dimensions();
    if w <= max_dim && h <= max_dim {
        return img.clone();
    }

    let ratio = f64::min(max_dim as f64 / w as f64, max_dim as f64 / h as f64);
    let new_w = (w as f64 * ratio).round() as u32;
    let new_h = (h as f64 * ratio).round() as u32;

    DynamicImage::ImageRgba8(image::imageops::resize(
        img,
        new_w.max(1),
        new_h.max(1),
        FilterType::Lanczos3,
    ))
}
