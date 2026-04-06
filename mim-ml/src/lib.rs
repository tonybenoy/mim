pub mod analysis;
pub mod clustering;
pub mod detection;
pub mod error;
pub mod gemma;
pub mod models;
pub mod ocr;
pub mod pipeline;
pub mod preprocessing;
pub mod recognition;
pub mod upscale;

pub use analysis::{AnalysisResult, PhotoAnalyzer};
pub use error::{MlError, Result};
pub use gemma::GemmaVision;
pub use models::{DownloadProgress, ScrfdModelConfig, get_scrfd_config};
pub use pipeline::{FacePipeline, ProcessingProgress};
pub use upscale::Upscaler;

/// Returns true if the `cuda` feature was enabled at compile time.
pub fn is_cuda_available() -> bool {
    cfg!(feature = "cuda")
}
