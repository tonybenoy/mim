use crate::error::Result;
use crate::preprocessing::{align_face, image_to_tensor};
use image::{DynamicImage, RgbImage};
use ort::session::Session;
use ort::value::Tensor;
use std::path::Path;
use tracing::debug;

pub struct FaceRecognizer {
    session: Session,
}

impl FaceRecognizer {
    pub fn new(model_path: &Path) -> Result<Self> {
        let mut builder = Session::builder()?;

        #[cfg(feature = "cuda")]
        {
            use ort::execution_providers::CUDAExecutionProvider;
            builder = builder
                .with_execution_providers([CUDAExecutionProvider::default().build()])
                .map_err(|e| crate::error::MlError::Other(format!("CUDA provider: {e}")))?;
            debug!("CUDA execution provider requested for ArcFace");
        }

        let session = builder.commit_from_file(model_path)?;

        tracing::info!(
            "ArcFace loaded: {} inputs, {} outputs, input_name={}",
            session.inputs().len(),
            session.outputs().len(),
            session.inputs()[0].name()
        );

        Ok(Self { session })
    }

    pub fn get_embedding(&mut self, img: &DynamicImage, landmarks: &[f32; 10]) -> Result<Vec<f32>> {
        let aligned = align_face(img, landmarks);
        self.get_embedding_from_crop(&aligned)
    }

    pub fn get_embedding_from_crop(&mut self, face_crop: &RgbImage) -> Result<Vec<f32>> {
        let input_tensor = image_to_tensor(face_crop, [127.5, 127.5, 127.5], [127.5, 127.5, 127.5]);

        let shape: Vec<usize> = input_tensor.shape().to_vec();
        let (data, _offset) = input_tensor.into_raw_vec_and_offset();
        let ort_tensor = Tensor::from_array((shape, data.into_boxed_slice()))?;

        let input_name = self.session.inputs()[0].name().to_string();
        let outputs = self.session.run(ort::inputs![input_name.as_str() => ort_tensor])?;

        let (_shape, raw_data) = outputs[0].try_extract_tensor::<f32>()?;
        let raw: Vec<f32> = raw_data.to_vec();

        // L2 normalize
        let norm: f32 = raw.iter().map(|x| x * x).sum::<f32>().sqrt();
        let embedding: Vec<f32> = if norm > 1e-10 {
            raw.iter().map(|x| x / norm).collect()
        } else {
            raw
        };

        debug!("Embedding dim: {}", embedding.len());
        Ok(embedding)
    }
}
