use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Face {
    pub id: String,
    pub photo_id: String,
    pub bbox_x: f32,
    pub bbox_y: f32,
    pub bbox_width: f32,
    pub bbox_height: f32,
    pub detection_confidence: f32,
    pub landmarks: Option<Vec<f32>>,
    pub embedding: Option<Vec<f32>>,
    pub identity_id: Option<String>,
    pub identity_confidence: Option<f32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceIdentity {
    pub id: String,
    pub name: String,
    pub representative_embedding: Option<Vec<f32>>,
    pub photo_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Face {
    pub fn new(photo_id: String, bbox_x: f32, bbox_y: f32, bbox_width: f32, bbox_height: f32, confidence: f32) -> Self {
        Face {
            id: Uuid::new_v4().to_string(),
            photo_id,
            bbox_x,
            bbox_y,
            bbox_width,
            bbox_height,
            detection_confidence: confidence,
            landmarks: None,
            embedding: None,
            identity_id: None,
            identity_confidence: None,
            created_at: Utc::now(),
        }
    }
}

impl FaceIdentity {
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        FaceIdentity {
            id: Uuid::new_v4().to_string(),
            name,
            representative_embedding: None,
            photo_count: 0,
            created_at: now,
            updated_at: now,
        }
    }
}
