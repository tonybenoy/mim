use crate::error::{MlError, Result};
use crate::models::ModelManager;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};
use ort::session::Session;
use ort::value::Tensor;
use std::path::{Path, PathBuf};
use tracing::info;

const REALESRGAN_URL: &str =
    "https://huggingface.co/onnx-community/realesrgan-x4plus/resolve/main/onnx/model.onnx";
const REALESRGAN_FILENAME: &str = "realesrgan_x4plus.onnx";

const TILE_SIZE: u32 = 512;
const TILE_OVERLAP: u32 = 32;
const SCALE_FACTOR: u32 = 4;

pub struct Upscaler {
    models_dir: PathBuf,
}

impl Upscaler {
    pub fn new(models_dir: impl Into<PathBuf>) -> Self {
        Self {
            models_dir: models_dir.into(),
        }
    }

    /// Attempt ONNX-based upscaling, falling back to Lanczos4 resize if model fails.
    pub async fn upscale(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        match self.upscale_onnx(input_path, output_path).await {
            Ok(()) => Ok(()),
            Err(e) => {
                info!(
                    "ONNX upscale failed ({}), falling back to Lanczos4 resize",
                    e
                );
                self.upscale_lanczos(input_path, output_path)
            }
        }
    }

    /// Lanczos4 fallback: simple 4x resize with high-quality interpolation.
    fn upscale_lanczos(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        let img = image::open(input_path)
            .map_err(|e| MlError::Other(format!("open image: {e}")))?;
        let (w, h) = img.dimensions();
        let new_w = w * SCALE_FACTOR;
        let new_h = h * SCALE_FACTOR;
        info!(
            "Lanczos4 upscale: {}x{} -> {}x{}",
            w, h, new_w, new_h
        );
        let resized = img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);
        resized
            .save(output_path)
            .map_err(|e| MlError::Other(format!("save image: {e}")))?;
        info!("Lanczos4 upscale saved to {}", output_path.display());
        Ok(())
    }

    /// ONNX Real-ESRGAN upscaling with tile-based processing.
    async fn upscale_onnx(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        let manager = ModelManager::new(self.models_dir.clone());
        let model_path = manager
            .ensure_model(REALESRGAN_FILENAME, REALESRGAN_URL)
            .await?;

        let img = image::open(input_path)
            .map_err(|e| MlError::Other(format!("open image: {e}")))?;
        let (width, height) = img.dimensions();
        let out_w = width * SCALE_FACTOR;
        let out_h = height * SCALE_FACTOR;

        info!(
            "ONNX upscale: {}x{} -> {}x{} (tile_size={})",
            width, height, out_w, out_h, TILE_SIZE
        );

        let model_path_clone = model_path.clone();
        let img_clone = img.clone();

        // Run the ONNX inference on a blocking thread
        let result: RgbaImage = tokio::task::spawn_blocking(move || {
            run_tiled_inference(&model_path_clone, &img_clone)
        })
        .await
        .map_err(|e| MlError::Other(format!("join: {e}")))?
        .map_err(|e| MlError::Other(format!("{e}")))?;

        let output = DynamicImage::ImageRgba8(result);
        output
            .save(output_path)
            .map_err(|e| MlError::Other(format!("save: {e}")))?;

        info!("ONNX upscale saved to {}", output_path.display());
        Ok(())
    }
}

/// Run tiled Real-ESRGAN inference over the whole image.
fn run_tiled_inference(
    model_path: &Path,
    img: &DynamicImage,
) -> std::result::Result<RgbaImage, String> {
    let mut session = Session::builder()
        .map_err(|e| format!("session builder: {e}"))?
        .commit_from_file(model_path)
        .map_err(|e| format!("load ONNX model: {e}"))?;

    let (width, height) = img.dimensions();
    let out_w = width * SCALE_FACTOR;
    let out_h = height * SCALE_FACTOR;

    let rgba_img = img.to_rgba8();
    let mut output_img: RgbaImage = ImageBuffer::new(out_w, out_h);

    let step = TILE_SIZE - TILE_OVERLAP;
    let tiles_x = (width + step - 1) / step;
    let tiles_y = (height + step - 1) / step;

    // Get the input name from the model
    let input_name = session.inputs()[0].name().to_string();

    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let x_start = (tx * step).min(width.saturating_sub(TILE_SIZE));
            let y_start = (ty * step).min(height.saturating_sub(TILE_SIZE));
            let tile_w = TILE_SIZE.min(width - x_start);
            let tile_h = TILE_SIZE.min(height - y_start);

            // Extract tile
            let tile = image::imageops::crop_imm(&rgba_img, x_start, y_start, tile_w, tile_h)
                .to_image();

            // Pad tile to TILE_SIZE x TILE_SIZE if needed
            let mut padded: RgbaImage = ImageBuffer::new(TILE_SIZE, TILE_SIZE);
            image::imageops::overlay(&mut padded, &tile, 0, 0);

            // Convert to NCHW float tensor [1, 3, H, W] in range [0, 1]
            let ts = TILE_SIZE as usize;
            let mut data = vec![0.0f32; 3 * ts * ts];
            for py in 0..TILE_SIZE {
                for px in 0..TILE_SIZE {
                    let pixel = padded.get_pixel(px, py);
                    let idx_base = (py as usize) * ts + (px as usize);
                    data[idx_base] = pixel[0] as f32 / 255.0;
                    data[ts * ts + idx_base] = pixel[1] as f32 / 255.0;
                    data[2 * ts * ts + idx_base] = pixel[2] as f32 / 255.0;
                }
            }

            let shape = vec![1usize, 3, ts, ts];
            let ort_tensor = Tensor::from_array((shape, data.into_boxed_slice()))
                .map_err(|e| format!("create tensor: {e}"))?;

            // Run inference
            let outputs = session
                .run(ort::inputs![input_name.as_str() => ort_tensor])
                .map_err(|e| format!("inference: {e}"))?;

            let (out_shape, out_data) = outputs[0]
                .try_extract_tensor::<f32>()
                .map_err(|e| format!("extract tensor: {e}"))?;

            let _out_c = out_shape[1] as usize;
            let out_h_tile = out_shape[2] as usize;
            let out_w_tile = out_shape[3] as usize;

            let out_tile_w = tile_w * SCALE_FACTOR;
            let out_tile_h = tile_h * SCALE_FACTOR;
            let out_x = x_start * SCALE_FACTOR;
            let out_y = y_start * SCALE_FACTOR;

            // Determine overlap regions for blending
            let inner_x = if tx > 0 {
                (TILE_OVERLAP / 2) * SCALE_FACTOR
            } else {
                0
            };
            let inner_y = if ty > 0 {
                (TILE_OVERLAP / 2) * SCALE_FACTOR
            } else {
                0
            };

            // Copy output tile to output image (skip overlap region on edges except first tile)
            for py in inner_y..out_tile_h {
                for px in inner_x..out_tile_w {
                    let dest_x = out_x + px;
                    let dest_y = out_y + py;
                    if dest_x >= out_w || dest_y >= out_h {
                        continue;
                    }
                    let idx_base = (py as usize) * out_w_tile + (px as usize);
                    let r = (out_data[idx_base].clamp(0.0, 1.0) * 255.0) as u8;
                    let g = (out_data[out_h_tile * out_w_tile + idx_base].clamp(0.0, 1.0) * 255.0) as u8;
                    let b = (out_data[2 * out_h_tile * out_w_tile + idx_base].clamp(0.0, 1.0) * 255.0) as u8;
                    output_img.put_pixel(dest_x, dest_y, Rgba([r, g, b, 255]));
                }
            }
        }
    }

    Ok(output_img)
}
