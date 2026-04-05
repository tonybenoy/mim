use crate::error::Result;
use crate::preprocessing::{image_to_tensor, letterbox_resize};
use image::{DynamicImage, GenericImageView};
use ort::session::Session;
use ort::value::Tensor;
use serde::Serialize;
use std::path::Path;
use tracing::debug;

const INPUT_SIZE: u32 = 640;
const STRIDES: [u32; 3] = [8, 16, 32];

#[derive(Debug, Clone, Serialize)]
pub struct DetectedFace {
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub landmarks: [f32; 10],
}

pub struct FaceDetector {
    session: Session,
    confidence_threshold: f32,
    nms_threshold: f32,
}

impl FaceDetector {
    pub fn new(model_path: &Path) -> Result<Self> {
        let mut builder = Session::builder()?;

        #[cfg(feature = "cuda")]
        {
            use ort::execution_providers::CUDAExecutionProvider;
            builder = builder.with_execution_providers([CUDAExecutionProvider::default().build()])?;
            debug!("CUDA execution provider requested for SCRFD");
        }

        let session = builder.commit_from_file(model_path)?;

        tracing::info!(
            "SCRFD loaded: {} inputs, {} outputs, input_name={}",
            session.inputs().len(),
            session.outputs().len(),
            session.inputs()[0].name()
        );

        Ok(Self {
            session,
            confidence_threshold: 0.5,
            nms_threshold: 0.4,
        })
    }

    pub fn detect(&mut self, img: &DynamicImage) -> Result<Vec<DetectedFace>> {
        let (orig_w, orig_h) = img.dimensions();
        let (letterboxed, scale, _, pad_x, pad_y) =
            letterbox_resize(img, INPUT_SIZE, INPUT_SIZE);

        let input_tensor =
            image_to_tensor(&letterboxed, [127.5, 127.5, 127.5], [128.0, 128.0, 128.0]);

        let shape: Vec<usize> = input_tensor.shape().to_vec();
        let (data, _offset) = input_tensor.into_raw_vec_and_offset();
        let ort_tensor = Tensor::from_array((shape, data.into_boxed_slice()))?;

        let input_name = self.session.inputs()[0].name().to_string();
        let outputs = self.session.run(ort::inputs![input_name.as_str() => ort_tensor])?;

        // Copy output data into owned buffers to release borrow on session
        let num_outputs = outputs.len();
        let mut owned_outputs: Vec<(Vec<usize>, Vec<f32>)> = Vec::with_capacity(num_outputs);
        for i in 0..num_outputs {
            let (shape, data) = outputs[i].try_extract_tensor::<f32>()?;
            let shape_vec: Vec<usize> = shape.iter().map(|&d| d as usize).collect();
            let data_vec: Vec<f32> = data.to_vec();
            // Debug: log first output tensor stats
            if i < 3 {
                let max_val = data_vec.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                let min_val = data_vec.iter().copied().fold(f32::INFINITY, f32::min);
                let above_thresh = data_vec.iter().filter(|&&v| v > 0.5).count();
                tracing::info!(
                    "Output[{}] shape={:?} len={} min={:.4} max={:.4} above_0.5={}",
                    i, shape_vec, data_vec.len(), min_val, max_val, above_thresh
                );
            }
            owned_outputs.push((shape_vec, data_vec));
        }
        drop(outputs);

        let mut all_faces: Vec<DetectedFace> = Vec::new();
        let conf_threshold = self.confidence_threshold;

        if num_outputs >= 9 {
            for (stride_idx, &stride) in STRIDES.iter().enumerate() {
                let (ref scores_shape, ref scores_data) = owned_outputs[stride_idx];
                let (_, ref bboxes_data) = owned_outputs[stride_idx + 3];
                let (_, ref kps_data) = owned_outputs[stride_idx + 6];

                decode_stride(
                    scores_shape,
                    scores_data,
                    bboxes_data,
                    Some(kps_data.as_slice()),
                    stride,
                    scale,
                    pad_x,
                    pad_y,
                    orig_w,
                    orig_h,
                    conf_threshold,
                    &mut all_faces,
                );
            }
        } else if num_outputs >= 6 {
            for (stride_idx, &stride) in STRIDES.iter().enumerate() {
                let (ref scores_shape, ref scores_data) = owned_outputs[stride_idx];
                let (_, ref bboxes_data) = owned_outputs[stride_idx + 3];

                decode_stride(
                    scores_shape,
                    scores_data,
                    bboxes_data,
                    None,
                    stride,
                    scale,
                    pad_x,
                    pad_y,
                    orig_w,
                    orig_h,
                    conf_threshold,
                    &mut all_faces,
                );
            }
        }

        let pre_nms = all_faces.len();
        let faces = nms(&mut all_faces, self.nms_threshold);

        debug!("Detected {} faces (pre-NMS: {})", faces.len(), pre_nms);
        Ok(faces)
    }
}

#[allow(clippy::too_many_arguments)]
fn decode_stride(
    scores_shape: &[usize],
    scores_data: &[f32],
    bboxes_data: &[f32],
    kps_data: Option<&[f32]>,
    stride: u32,
    scale: f32,
    pad_x: f32,
    pad_y: f32,
    orig_w: u32,
    orig_h: u32,
    confidence_threshold: f32,
    out: &mut Vec<DetectedFace>,
) {
    let feat_w = INPUT_SIZE / stride;
    let num_anchors = 2;

    // Shape is [N, 1] where N = feat_h * feat_w * num_anchors
    let total_anchors = scores_shape[0];

    for idx in 0..total_anchors {
        let score = scores_data[idx];

        if score < confidence_threshold {
            continue;
        }

        let grid_idx = idx / num_anchors;
        let grid_y = grid_idx / feat_w as usize;
        let grid_x = grid_idx % feat_w as usize;

        let cx = (grid_x as f32 + 0.5) * stride as f32;
        let cy = (grid_y as f32 + 0.5) * stride as f32;

        let base = idx * 4;
        if base + 3 >= bboxes_data.len() {
            continue;
        }
        let dx = bboxes_data[base];
        let dy = bboxes_data[base + 1];
        let dw = bboxes_data[base + 2];
        let dh = bboxes_data[base + 3];

        let x1 = (cx - dx * stride as f32 - pad_x) / scale;
        let y1 = (cy - dy * stride as f32 - pad_y) / scale;
        let x2 = (cx + dw * stride as f32 - pad_x) / scale;
        let y2 = (cy + dh * stride as f32 - pad_y) / scale;

        let x = x1.max(0.0).min(orig_w as f32);
        let y = y1.max(0.0).min(orig_h as f32);
        let w = (x2 - x1).max(0.0).min(orig_w as f32 - x);
        let h = (y2 - y1).max(0.0).min(orig_h as f32 - y);

        let mut lms = [0.0f32; 10];
        if let Some(kps) = kps_data {
            let kps_base = idx * 10;
            if kps_base + 9 < kps.len() {
                for k in 0..5 {
                    let lx = kps[kps_base + k * 2];
                    let ly = kps[kps_base + k * 2 + 1];
                    lms[k * 2] = (lx * stride as f32 + cx - pad_x) / scale;
                    lms[k * 2 + 1] = (ly * stride as f32 + cy - pad_y) / scale;
                }
            }
        }

        out.push(DetectedFace {
            bbox: [x, y, w, h],
            confidence: score,
            landmarks: lms,
        });
    }
}

fn nms(detections: &mut [DetectedFace], iou_threshold: f32) -> Vec<DetectedFace> {
    detections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

    let mut keep: Vec<DetectedFace> = Vec::new();
    for det in detections.iter() {
        let suppressed = keep.iter().any(|k| iou(&det.bbox, &k.bbox) > iou_threshold);
        if !suppressed {
            keep.push(det.clone());
        }
    }
    keep
}

fn iou(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    let x1 = a[0].max(b[0]);
    let y1 = a[1].max(b[1]);
    let x2 = (a[0] + a[2]).min(b[0] + b[2]);
    let y2 = (a[1] + a[3]).min(b[1] + b[3]);

    let inter = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
    let area_a = a[2] * a[3];
    let area_b = b[2] * b[3];
    let union = area_a + area_b - inter;

    if union <= 0.0 { 0.0 } else { inter / union }
}
