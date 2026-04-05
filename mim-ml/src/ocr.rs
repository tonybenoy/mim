use crate::error::{MlError, Result};
use crate::models::ModelManager;
use crate::preprocessing::image_to_tensor;
use image::{DynamicImage, GenericImageView, RgbImage};
use ort::session::Session;
use ort::value::Tensor;
use serde::Serialize;
use std::path::Path;
use tracing::info;

// PP-OCRv4 detection model (finds text regions)
const OCR_DET_URL: &str =
    "https://huggingface.co/onnx-community/pp-ocrv4-det/resolve/main/onnx/model.onnx";
const OCR_DET_FILENAME: &str = "ppocr_det.onnx";

// PP-OCRv4 recognition model (reads text from regions)
const OCR_REC_URL: &str =
    "https://huggingface.co/onnx-community/pp-ocrv4-rec/resolve/main/onnx/model.onnx";
const OCR_REC_FILENAME: &str = "ppocr_rec.onnx";

#[derive(Debug, Clone, Serialize)]
pub struct OcrResult {
    pub text: String,
    pub regions: Vec<OcrRegion>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OcrRegion {
    pub text: String,
    pub confidence: f32,
    pub bbox: [f32; 4],
}

pub struct OcrEngine {
    det_session: Session,
    // rec_session for character recognition would go here
    // For now, we use detection to identify text presence
    // and rely on Gemma for actual reading
}

impl OcrEngine {
    pub async fn new(models_dir: &Path) -> Result<Self> {
        let manager = ModelManager::new(models_dir.to_path_buf());

        info!("Loading OCR detection model...");
        let det_path = manager.ensure_model(OCR_DET_FILENAME, OCR_DET_URL).await?;

        let det_session = Session::builder()?.commit_from_file(&det_path)?;

        info!(
            "OCR engine loaded: {} inputs, {} outputs",
            det_session.inputs().len(),
            det_session.outputs().len()
        );

        Ok(Self { det_session })
    }

    /// Detect whether an image contains text. Returns true if text regions found.
    /// This is a fast pre-filter — only images with text get sent to Gemma for OCR.
    pub fn has_text(&mut self, img: &DynamicImage) -> Result<bool> {
        let (orig_w, orig_h) = img.dimensions();

        // Resize to 960 max dimension for detection
        let max_dim = 960u32;
        let scale = f32::min(max_dim as f32 / orig_w as f32, max_dim as f32 / orig_h as f32);
        let new_w = ((orig_w as f32 * scale) as u32).max(32);
        let new_h = ((orig_h as f32 * scale) as u32).max(32);
        // Round to multiple of 32 (PP-OCR requirement)
        let new_w = (new_w + 31) / 32 * 32;
        let new_h = (new_h + 31) / 32 * 32;

        let resized = image::imageops::resize(
            &img.to_rgb8(),
            new_w,
            new_h,
            image::imageops::FilterType::Triangle,
        );

        let input = image_to_tensor(
            &resized,
            [123.675, 116.28, 103.53],
            [58.395, 57.12, 57.375],
        );

        let shape: Vec<usize> = input.shape().to_vec();
        let (data, _) = input.into_raw_vec_and_offset();
        let ort_tensor = Tensor::from_array((shape, data.into_boxed_slice()))?;

        let input_name = self.det_session.inputs()[0].name().to_string();
        let outputs = self.det_session.run(ort::inputs![input_name.as_str() => ort_tensor])?;

        let (out_shape, out_data) = outputs[0].try_extract_tensor::<f32>()?;

        // Check if any detection scores exceed threshold
        let threshold = 0.3;
        let text_pixels = out_data.iter().filter(|&&v| v > threshold).count();
        let total_pixels = out_data.len();

        // If more than 0.5% of pixels are "text", consider it text-bearing
        let text_ratio = text_pixels as f32 / total_pixels.max(1) as f32;

        Ok(text_ratio > 0.005)
    }
}
