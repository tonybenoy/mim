use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderSource {
    pub id: String,
    pub path: String,
    pub label: Option<String>,
    pub drive_serial: Option<String>,
    pub is_available: bool,
    pub is_locked: bool,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub last_scanned_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub relative_path: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub photo_count: u32,
    pub cover_photo_id: Option<String>,
}

impl FolderSource {
    pub fn new(path: String, label: Option<String>) -> Self {
        FolderSource {
            id: Uuid::new_v4().to_string(),
            path,
            label,
            drive_serial: None,
            is_available: true,
            is_locked: false,
            password_hash: None,
            last_scanned_at: None,
            created_at: Utc::now(),
        }
    }
}

impl Folder {
    pub fn new(relative_path: String, name: String, parent_id: Option<String>) -> Self {
        Folder {
            id: Uuid::new_v4().to_string(),
            relative_path,
            name,
            parent_id,
            photo_count: 0,
            cover_photo_id: None,
        }
    }
}
