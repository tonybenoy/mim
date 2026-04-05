use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoTag {
    pub photo_id: String,
    pub tag_id: String,
    pub confidence: Option<f32>,
    pub source: TagSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TagSource {
    Gemma,
    User,
    Face,
}

impl Tag {
    pub fn new(name: String, category: Option<String>) -> Self {
        Tag {
            id: Uuid::new_v4().to_string(),
            name,
            category,
        }
    }
}
