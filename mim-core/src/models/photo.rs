use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photo {
    pub id: String,
    pub relative_path: String,
    pub filename: String,
    pub file_size: u64,
    pub content_hash: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<String>,

    // EXIF
    pub taken_at: Option<DateTime<Utc>>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub focal_length: Option<f64>,
    pub aperture: Option<f64>,
    pub shutter_speed: Option<String>,
    pub iso: Option<u32>,

    // GPS
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub location_name: Option<String>,

    // AI
    pub ai_description: Option<String>,
    pub ai_processed_at: Option<DateTime<Utc>>,

    // Analysis
    pub aesthetic_score: Option<f64>,
    pub blur_score: Option<f64>,
    pub scene_type: Option<String>,
    pub dominant_colors: Option<String>,
    pub perceptual_hash: Option<String>,
    pub is_screenshot: bool,
    pub is_nsfw: bool,
    pub ocr_text: Option<String>,
    pub weather: Option<String>,
    pub time_of_day: Option<String>,
    pub event_id: Option<String>,
    pub analysis_processed: bool,

    // Processing state
    pub thumbnail_generated: bool,
    pub faces_processed: bool,
    pub ai_processed: bool,

    // Media type
    pub media_type: MediaType,

    // Timestamps
    pub file_modified_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Photo,
    Video,
}

impl Photo {
    pub fn new(relative_path: String, filename: String, file_size: u64, content_hash: String) -> Self {
        let now = Utc::now();
        Photo {
            id: Uuid::new_v4().to_string(),
            relative_path,
            filename,
            file_size,
            content_hash,
            width: None,
            height: None,
            format: None,
            taken_at: None,
            camera_make: None,
            camera_model: None,
            lens_model: None,
            focal_length: None,
            aperture: None,
            shutter_speed: None,
            iso: None,
            latitude: None,
            longitude: None,
            altitude: None,
            location_name: None,
            ai_description: None,
            ai_processed_at: None,
            aesthetic_score: None,
            blur_score: None,
            scene_type: None,
            dominant_colors: None,
            perceptual_hash: None,
            is_screenshot: false,
            is_nsfw: false,
            ocr_text: None,
            weather: None,
            time_of_day: None,
            event_id: None,
            analysis_processed: false,
            thumbnail_generated: false,
            faces_processed: false,
            ai_processed: false,
            media_type: MediaType::Photo,
            file_modified_at: now,
            created_at: now,
            updated_at: now,
        }
    }
}
