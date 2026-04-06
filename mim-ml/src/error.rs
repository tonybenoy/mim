use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum MlError {
    #[error("model not found: {0}")]
    ModelNotFound(PathBuf),

    #[error("model download failed: {0}")]
    DownloadFailed(String),

    #[error("ONNX runtime error: {0}")]
    Ort(#[from] ort::Error),

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("core error: {0}")]
    Core(String),

    #[error("preprocessing error: {0}")]
    Preprocessing(String),

    #[error("no faces to cluster")]
    NoFaces,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, MlError>;

impl From<mim_core::Error> for MlError {
    fn from(e: mim_core::Error) -> Self {
        MlError::Core(e.to_string())
    }
}

impl From<MlError> for mim_core::Error {
    fn from(e: MlError) -> Self {
        mim_core::Error::Other(e.to_string())
    }
}

impl serde::Serialize for MlError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
