use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub cover_photo_id: Option<String>,
    pub album_type: AlbumType,
    pub rules: Option<serde_json::Value>,
    pub photo_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlbumType {
    Manual,
    Smart,
    Favorites,
}

impl Album {
    pub fn new(name: String, album_type: AlbumType) -> Self {
        let now = Utc::now();
        Album {
            id: Uuid::new_v4().to_string(),
            name,
            cover_photo_id: None,
            album_type,
            rules: None,
            photo_count: 0,
            created_at: now,
            updated_at: now,
        }
    }
}
