//! Unified photo analysis module.
//!
//! All features here are purely algorithmic — no ML model downloads required.
//! Includes: dominant color extraction, perceptual hashing, blur detection,
//! screenshot detection, time-of-day classification, and event clustering.

use chrono::Timelike;
use image::{GrayImage, RgbImage};
use mim_core::db::{Event, EventsDb, PhotosDb};
use mim_core::geocode;
use mim_core::models::Photo;
use parking_lot::Mutex;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub dominant_colors: Vec<String>,
    pub perceptual_hash: String,
    pub blur_score: f32,
    pub is_screenshot: bool,
    pub has_text: bool,
    pub time_of_day: String,
}

// ---------------------------------------------------------------------------
// PhotoAnalyzer — runs all non-model analyses on a single photo
// ---------------------------------------------------------------------------

pub struct PhotoAnalyzer;

impl PhotoAnalyzer {
    /// Run all non-model analyses on a single photo.
    pub fn analyze(photo: &Photo, image_path: &Path) -> Option<AnalysisResult> {
        let img = image::open(image_path).ok()?;
        let rgb = img.to_rgb8();
        let gray = img.to_luma8();

        let dominant_colors = extract_dominant_colors(&rgb, 5);
        let perceptual_hash = compute_phash(&gray);
        let blur_score = detect_blur(&gray);
        let is_screenshot = detect_screenshot(photo, &rgb);
        let has_text = is_screenshot || detect_text_heuristic(&rgb);
        let time_of_day = classify_time_of_day(photo, &gray);

        Some(AnalysisResult {
            dominant_colors,
            perceptual_hash,
            blur_score,
            is_screenshot,
            has_text,
            time_of_day,
        })
    }

    /// Run analysis on all unanalyzed photos in a sidecar DB.
    /// Returns the number of photos processed.
    pub fn analyze_folder(
        conn_writer: &Arc<Mutex<Connection>>,
        conn_reader: &Arc<Mutex<Connection>>,
        root_path: &Path,
    ) -> usize {
        let photos = match PhotosDb::list_unprocessed_analysis(conn_reader) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to list unprocessed photos: {}", e);
                return 0;
            }
        };

        let mut processed = 0;
        for photo in &photos {
            let img_path = root_path.join(&photo.relative_path);
            if !img_path.exists() {
                continue;
            }

            match Self::analyze(photo, &img_path) {
                Some(result) => {
                    let colors_json = serde_json::to_string(&result.dominant_colors).ok();
                    // If text detected, set a marker in ocr_text
                    let ocr_text = if result.has_text {
                        Some("[text detected]")
                    } else {
                        None
                    };
                    let _ = PhotosDb::update_analysis(
                        conn_writer,
                        &photo.id,
                        Some(result.blur_score as f64),
                        colors_json.as_deref(),
                        Some(&result.perceptual_hash),
                        result.is_screenshot,
                        Some(&result.time_of_day),
                        ocr_text,
                    );

                    // Reverse geocode if lat/lon present but location_name is missing
                    if photo.location_name.is_none() {
                        if let (Some(lat), Some(lon)) = (photo.latitude, photo.longitude) {
                            if let Some(name) = geocode::reverse_geocode(lat, lon) {
                                let _ = PhotosDb::update_location_name(conn_writer, &photo.id, &name);
                            }
                        }
                    }

                    processed += 1;
                }
                None => {
                    tracing::warn!("Failed to analyze photo: {}", photo.filename);
                }
            }
        }

        processed
    }

    /// Cluster photos into events based on time gaps and GPS distance.
    pub fn cluster_events(
        conn_writer: &Arc<Mutex<Connection>>,
        conn_reader: &Arc<Mutex<Connection>>,
    ) -> Vec<Event> {
        let photos = match PhotosDb::list(conn_reader, 100_000, 0) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };

        // Sort by taken_at (or created_at as fallback)
        let mut sorted: Vec<&Photo> = photos.iter().collect();
        sorted.sort_by_key(|p| p.taken_at.unwrap_or(p.created_at));

        if sorted.is_empty() {
            return Vec::new();
        }

        let three_hours = chrono::Duration::hours(3);
        let mut events: Vec<Event> = Vec::new();
        let mut current_event_photos: Vec<&Photo> = vec![sorted[0]];

        for window in sorted.windows(2) {
            let prev = window[0];
            let curr = window[1];
            let prev_time = prev.taken_at.unwrap_or(prev.created_at);
            let curr_time = curr.taken_at.unwrap_or(curr.created_at);

            let time_gap = curr_time.signed_duration_since(prev_time) > three_hours;
            let geo_split = should_split_by_distance(prev, curr);

            if time_gap || geo_split {
                // Finalize current event
                let event = build_event(&current_event_photos);
                events.push(event);
                current_event_photos.clear();
            }
            current_event_photos.push(curr);
        }

        // Finalize last event
        if !current_event_photos.is_empty() {
            let event = build_event(&current_event_photos);
            events.push(event);
        }

        // Persist events and update photo event_ids
        for event in &events {
            let _ = EventsDb::upsert(conn_writer, event);
        }

        // We need to reassign event IDs to photos. Rebuild the assignment.
        let mut sorted_owned: Vec<Photo> = photos;
        sorted_owned.sort_by_key(|p| p.taken_at.unwrap_or(p.created_at));

        let mut event_idx = 0;
        let mut group_start = 0;
        for i in 1..sorted_owned.len() {
            let prev = &sorted_owned[i - 1];
            let curr = &sorted_owned[i];
            let prev_time = prev.taken_at.unwrap_or(prev.created_at);
            let curr_time = curr.taken_at.unwrap_or(curr.created_at);

            let time_gap = curr_time.signed_duration_since(prev_time) > three_hours;
            let geo_split = should_split_by_distance(prev, curr);

            if time_gap || geo_split {
                // Assign event_id to photos [group_start..i)
                if event_idx < events.len() {
                    for j in group_start..i {
                        let _ = PhotosDb::update_event_id(
                            conn_writer,
                            &sorted_owned[j].id,
                            &events[event_idx].id,
                        );
                    }
                }
                event_idx += 1;
                group_start = i;
            }
        }
        // Last group
        if event_idx < events.len() {
            for j in group_start..sorted_owned.len() {
                let _ = PhotosDb::update_event_id(
                    conn_writer,
                    &sorted_owned[j].id,
                    &events[event_idx].id,
                );
            }
        }

        events
    }
}

// ---------------------------------------------------------------------------
// Dominant Color Extraction (k-means, k=5)
// ---------------------------------------------------------------------------

fn extract_dominant_colors(img: &RgbImage, k: usize) -> Vec<String> {
    // Resize to 64x64 for speed
    let small = image::imageops::resize(img, 64, 64, image::imageops::FilterType::Nearest);
    let pixels: Vec<[f32; 3]> = small
        .pixels()
        .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32])
        .collect();

    if pixels.is_empty() {
        return Vec::new();
    }

    // Initialize centroids by picking evenly spaced pixels
    let step = pixels.len().max(1) / k.max(1);
    let mut centroids: Vec<[f32; 3]> = (0..k)
        .map(|i| pixels[(i * step).min(pixels.len() - 1)])
        .collect();

    // K-means iterations
    for _ in 0..20 {
        let mut sums = vec![[0.0f64; 3]; k];
        let mut counts = vec![0u32; k];

        for px in &pixels {
            let mut best = 0;
            let mut best_dist = f32::MAX;
            for (ci, c) in centroids.iter().enumerate() {
                let d = (px[0] - c[0]).powi(2) + (px[1] - c[1]).powi(2) + (px[2] - c[2]).powi(2);
                if d < best_dist {
                    best_dist = d;
                    best = ci;
                }
            }
            sums[best][0] += px[0] as f64;
            sums[best][1] += px[1] as f64;
            sums[best][2] += px[2] as f64;
            counts[best] += 1;
        }

        for ci in 0..k {
            if counts[ci] > 0 {
                centroids[ci] = [
                    (sums[ci][0] / counts[ci] as f64) as f32,
                    (sums[ci][1] / counts[ci] as f64) as f32,
                    (sums[ci][2] / counts[ci] as f64) as f32,
                ];
            }
        }
    }

    // Count assignments for sorting by frequency
    let mut counts = vec![0u32; k];
    for px in &pixels {
        let mut best = 0;
        let mut best_dist = f32::MAX;
        for (ci, c) in centroids.iter().enumerate() {
            let d = (px[0] - c[0]).powi(2) + (px[1] - c[1]).powi(2) + (px[2] - c[2]).powi(2);
            if d < best_dist {
                best_dist = d;
                best = ci;
            }
        }
        counts[best] += 1;
    }

    // Sort by frequency descending
    let mut indexed: Vec<(usize, u32)> = counts.into_iter().enumerate().collect();
    indexed.sort_by(|a, b| b.1.cmp(&a.1));

    indexed
        .iter()
        .take(k)
        .map(|(i, _)| {
            let c = &centroids[*i];
            format!(
                "#{:02x}{:02x}{:02x}",
                c[0].round() as u8,
                c[1].round() as u8,
                c[2].round() as u8
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Perceptual Hash (pHash)
// ---------------------------------------------------------------------------

fn compute_phash(gray: &GrayImage) -> String {
    // Resize to 32x32
    let small = image::imageops::resize(gray, 32, 32, image::imageops::FilterType::Lanczos3);

    // Build 32x32 f64 matrix
    let mut matrix = [[0.0f64; 32]; 32];
    for y in 0..32 {
        for x in 0..32 {
            matrix[y][x] = small.get_pixel(x as u32, y as u32)[0] as f64;
        }
    }

    // Apply simplified DCT (type II) on each row, then each column
    let dct_rows = dct2d(&matrix);

    // Take top-left 8x8 of DCT coefficients (excluding DC at [0][0])
    let mut values = Vec::with_capacity(64);
    for y in 0..8 {
        for x in 0..8 {
            values.push(dct_rows[y][x]);
        }
    }

    // Compute median (excluding DC component)
    let mut sorted = values[1..].to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[sorted.len() / 2];

    // Build 64-bit hash
    let mut hash: u64 = 0;
    for (i, &v) in values.iter().enumerate() {
        if v > median {
            hash |= 1 << (63 - i);
        }
    }

    format!("{:016x}", hash)
}

/// Simplified 2D DCT-II. Apply 1D DCT on rows then on columns.
fn dct2d(input: &[[f64; 32]; 32]) -> [[f64; 32]; 32] {
    let mut temp = [[0.0f64; 32]; 32];
    // DCT on rows
    for y in 0..32 {
        for u in 0..32 {
            let mut sum = 0.0;
            for x in 0..32 {
                sum += input[y][x]
                    * ((2.0 * x as f64 + 1.0) * u as f64 * std::f64::consts::PI / 64.0).cos();
            }
            temp[y][u] = sum;
        }
    }
    // DCT on columns
    let mut result = [[0.0f64; 32]; 32];
    for x in 0..32 {
        for v in 0..32 {
            let mut sum = 0.0;
            for y in 0..32 {
                sum += temp[y][x]
                    * ((2.0 * y as f64 + 1.0) * v as f64 * std::f64::consts::PI / 64.0).cos();
            }
            result[v][x] = sum;
        }
    }
    result
}

/// Compute hamming distance between two hex-encoded perceptual hashes.
pub fn hamming_distance(hash1: &str, hash2: &str) -> u32 {
    mim_core::db::hamming_distance_hex(hash1, hash2)
}

// ---------------------------------------------------------------------------
// Blur Detection (Laplacian variance)
// ---------------------------------------------------------------------------

fn detect_blur(gray: &GrayImage) -> f32 {
    let (w, h) = gray.dimensions();
    if w < 3 || h < 3 {
        return 0.0;
    }

    // Resize for consistency/speed
    let small = image::imageops::resize(gray, 256.min(w), 256.min(h), image::imageops::FilterType::Nearest);
    let (sw, sh) = small.dimensions();

    // Apply Laplacian kernel [[0,1,0],[1,-4,1],[0,1,0]]
    let mut laplacian_values: Vec<f64> = Vec::new();
    for y in 1..(sh - 1) {
        for x in 1..(sw - 1) {
            let center = small.get_pixel(x, y)[0] as f64;
            let top = small.get_pixel(x, y - 1)[0] as f64;
            let bottom = small.get_pixel(x, y + 1)[0] as f64;
            let left = small.get_pixel(x - 1, y)[0] as f64;
            let right = small.get_pixel(x + 1, y)[0] as f64;
            let lap = top + bottom + left + right - 4.0 * center;
            laplacian_values.push(lap);
        }
    }

    if laplacian_values.is_empty() {
        return 0.0;
    }

    let mean = laplacian_values.iter().sum::<f64>() / laplacian_values.len() as f64;
    let variance =
        laplacian_values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / laplacian_values.len() as f64;

    // Map variance to a 0-1 blur score.
    // Low variance = blurry. Threshold ~100 for "blurry".
    // Score: 1.0 = very blurry, 0.0 = sharp.
    let score = 1.0 - (variance / 500.0).min(1.0);
    score as f32
}

// ---------------------------------------------------------------------------
// Screenshot Detection
// ---------------------------------------------------------------------------

fn detect_screenshot(photo: &Photo, img: &RgbImage) -> bool {
    // Check filename patterns
    let name_lower = photo.filename.to_lowercase();
    let filename_match = name_lower.contains("screenshot")
        || name_lower.contains("screen shot")
        || name_lower.contains("captura")
        || name_lower.contains("bildschirmfoto")
        || name_lower.starts_with("scr_")
        || name_lower.contains("screen_recording");

    if filename_match {
        return true;
    }

    // Check common screenshot aspect ratios
    if let (Some(w), Some(h)) = (photo.width, photo.height) {
        let (w, h) = (w as f64, h as f64);
        // Common phone screenshot ratios (portrait)
        let ratio = if w > h { w / h } else { h / w };
        let common_ratios = [
            19.5 / 9.0,  // iPhone X+
            16.0 / 9.0,  // HD
            18.0 / 9.0,  // Samsung
            20.0 / 9.0,  // tall phones
            4.0 / 3.0,   // iPad
        ];
        let is_exact_ratio = common_ratios.iter().any(|r| (ratio - r).abs() < 0.02);

        // Check for solid color band at top (status bar)
        if is_exact_ratio {
            let top_band = check_solid_band(img, 0, (h as u32).min(40));
            if top_band {
                return true;
            }
        }
    }

    false
}

/// Check if the top rows of the image have a nearly uniform color (status bar indicator).
fn check_solid_band(img: &RgbImage, y_start: u32, y_end: u32) -> bool {
    let (w, h) = img.dimensions();
    if y_end > h || w == 0 {
        return false;
    }

    let sample = img.get_pixel(w / 2, y_start);
    let r0 = sample[0] as i32;
    let g0 = sample[1] as i32;
    let b0 = sample[2] as i32;

    let mut matching = 0u32;
    let mut total = 0u32;
    for y in y_start..y_end.min(h) {
        for x in (0..w).step_by(4) {
            let p = img.get_pixel(x, y);
            let dr = (p[0] as i32 - r0).abs();
            let dg = (p[1] as i32 - g0).abs();
            let db = (p[2] as i32 - b0).abs();
            if dr + dg + db < 30 {
                matching += 1;
            }
            total += 1;
        }
    }

    total > 0 && (matching as f64 / total as f64) > 0.85
}

// ---------------------------------------------------------------------------
// Text Detection Heuristic (no ML model needed)
// ---------------------------------------------------------------------------

/// Detect whether an image likely contains significant text regions.
/// Checks for large uniform-color bands and high edge density typical of
/// documents, screenshots, and text-heavy images.
fn detect_text_heuristic(img: &RgbImage) -> bool {
    let (w, h) = img.dimensions();
    if w < 100 || h < 100 {
        return false;
    }

    // Resize to 128x128 for speed
    let small = image::imageops::resize(img, 128, 128, image::imageops::FilterType::Nearest);
    let gray: GrayImage = image::DynamicImage::ImageRgb8(
        image::RgbImage::from_raw(128, 128, small.into_raw()).unwrap_or_default()
    ).to_luma8();

    // Count horizontal edge transitions (text has many sharp transitions)
    let mut transitions = 0u32;
    let mut total = 0u32;
    for y in 0..128 {
        for x in 1..128 {
            let diff = (gray.get_pixel(x, y)[0] as i32 - gray.get_pixel(x - 1, y)[0] as i32).abs();
            if diff > 40 {
                transitions += 1;
            }
            total += 1;
        }
    }

    if total == 0 {
        return false;
    }

    // High transition ratio suggests text-like content
    let transition_ratio = transitions as f32 / total as f32;

    // Also check for large uniform background: sample corners
    let corners = [
        img.get_pixel(5, 5),
        img.get_pixel(w - 6, 5),
        img.get_pixel(5, h - 6),
        img.get_pixel(w - 6, h - 6),
    ];
    let corner_similar = corners.windows(2).all(|pair| {
        let a = pair[0];
        let b = pair[1];
        (a[0] as i32 - b[0] as i32).abs() +
        (a[1] as i32 - b[1] as i32).abs() +
        (a[2] as i32 - b[2] as i32).abs() < 60
    });

    // Text-heavy images: high transitions + uniform background
    (transition_ratio > 0.15 && corner_similar) || transition_ratio > 0.25
}

// ---------------------------------------------------------------------------
// Time of Day Classification
// ---------------------------------------------------------------------------

fn classify_time_of_day(photo: &Photo, gray: &GrayImage) -> String {
    // Prefer EXIF time if available
    if let Some(taken_at) = photo.taken_at {
        let hour = taken_at.hour();
        return match hour {
            5..=6 => "dawn",
            7..=9 => "morning",
            10..=13 => "midday",
            14..=16 => "afternoon",
            17..=18 => "golden_hour",
            19 => "blue_hour",
            _ => "night",
        }
        .to_string();
    }

    // Fallback: check image brightness
    let avg_brightness = average_brightness(gray);
    if avg_brightness < 50.0 {
        "night".to_string()
    } else if avg_brightness < 100.0 {
        "indoor".to_string()
    } else {
        "midday".to_string()
    }
}

fn average_brightness(gray: &GrayImage) -> f64 {
    let small = image::imageops::resize(gray, 64, 64, image::imageops::FilterType::Nearest);
    let sum: f64 = small.pixels().map(|p| p[0] as f64).sum();
    let count = small.width() * small.height();
    if count == 0 {
        return 128.0;
    }
    sum / count as f64
}

// ---------------------------------------------------------------------------
// Event Clustering helpers
// ---------------------------------------------------------------------------

fn should_split_by_distance(a: &Photo, b: &Photo) -> bool {
    match (a.latitude, a.longitude, b.latitude, b.longitude) {
        (Some(lat1), Some(lon1), Some(lat2), Some(lon2)) => {
            geocode::haversine_km(lat1, lon1, lat2, lon2) > 50.0
        }
        _ => false,
    }
}

fn build_event(photos: &[&Photo]) -> Event {
    let first = photos.first().unwrap();
    let last = photos.last().unwrap();
    let start = first.taken_at.unwrap_or(first.created_at);
    let end = last.taken_at.unwrap_or(last.created_at);

    // Generate name from date + location
    let date_str = start.format("%b %d, %Y").to_string();
    let location = photos
        .iter()
        .find_map(|p| p.location_name.as_ref())
        .cloned();

    let name = match &location {
        Some(loc) => format!("{} - {}", date_str, loc),
        None => date_str,
    };

    Event {
        id: Uuid::new_v4().to_string(),
        name,
        start_time: Some(start.to_rfc3339()),
        end_time: Some(end.to_rfc3339()),
        location_name: location,
        photo_count: photos.len() as u32,
    }
}
