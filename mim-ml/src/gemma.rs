use crate::error::{MlError, Result};
use crate::models::ModelManager;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::mtmd::{MtmdBitmap, MtmdContext, MtmdContextParams, MtmdInputText};
use llama_cpp_2::sampling::LlamaSampler;
use serde::Serialize;
use std::path::Path;
use tracing::info;

pub struct GemmaModelConfig {
    pub model_url: &'static str,
    pub mmproj_url: &'static str,
    pub model_filename: &'static str,
    pub mmproj_filename: &'static str,
}

pub const GEMMA_3_4B: GemmaModelConfig = GemmaModelConfig {
    model_url: "https://huggingface.co/ggml-org/gemma-3-4b-it-GGUF/resolve/main/gemma-3-4b-it-Q4_K_M.gguf",
    mmproj_url: "https://huggingface.co/ggml-org/gemma-3-4b-it-GGUF/resolve/main/mmproj-model-f16.gguf",
    model_filename: "gemma-3-4b-it-Q4_K_M.gguf",
    mmproj_filename: "mmproj-gemma3-4b-f16.gguf",
};

pub const GEMMA_3_1B: GemmaModelConfig = GemmaModelConfig {
    model_url: "https://huggingface.co/ggml-org/gemma-3-1b-it-GGUF/resolve/main/gemma-3-1b-it-Q4_K_M.gguf",
    mmproj_url: "https://huggingface.co/ggml-org/gemma-3-4b-it-GGUF/resolve/main/mmproj-model-f16.gguf",
    model_filename: "gemma-3-1b-it-Q4_K_M.gguf",
    mmproj_filename: "mmproj-gemma3-1b-f16.gguf",
};

pub fn get_model_config(model_id: &str) -> &'static GemmaModelConfig {
    match model_id {
        "gemma-3-1b" => &GEMMA_3_1B,
        _ => &GEMMA_3_4B, // default
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageAnalysis {
    pub description: String,
    pub tags: Vec<String>,
}

pub struct GemmaVision {
    backend: LlamaBackend,
    model: LlamaModel,
    mmproj_path: String,
}

impl GemmaVision {
    pub async fn new(models_dir: &Path, model_id: &str) -> Result<Self> {
        Self::new_with_progress(models_dir, model_id, None).await
    }

    pub async fn new_with_progress(
        models_dir: &Path,
        model_id: &str,
        progress_tx: Option<tokio::sync::watch::Sender<Option<crate::models::DownloadProgress>>>,
    ) -> Result<Self> {
        let config = get_model_config(model_id);

        let mut manager = ModelManager::new(models_dir.to_path_buf());
        if let Some(tx) = progress_tx {
            manager = manager.with_progress(tx);
        }

        info!("Ensuring Gemma model files (config: {})...", model_id);
        let model_path = manager
            .ensure_model(config.model_filename, config.model_url)
            .await?;
        let mmproj_path = manager
            .ensure_model(config.mmproj_filename, config.mmproj_url)
            .await?;

        info!("Loading Gemma model into memory (this may take a minute)...");

        let model_path_owned = model_path.to_path_buf();
        let mmproj_str = mmproj_path.to_string_lossy().to_string();

        // Load on a blocking thread — model loading reads 5+ GB from disk
        let (backend, model) = tokio::task::spawn_blocking(move || {
            let backend = LlamaBackend::init()
                .map_err(|e| MlError::Other(format!("backend init: {e}")))?;

            let model_params = LlamaModelParams::default();
            let model = LlamaModel::load_from_file(&backend, &model_path_owned, &model_params)
                .map_err(|e| MlError::Other(format!("model load: {e}")))?;

            Ok::<_, MlError>((backend, model))
        })
        .await
        .map_err(|e| MlError::Other(format!("spawn blocking: {e}")))??;

        info!("Gemma vision model loaded");
        Ok(Self {
            backend,
            model,
            mmproj_path: mmproj_str,
        })
    }

    /// Describe an image and extract tags in one pass.
    pub fn analyze_image(&self, image_path: &Path) -> Result<ImageAnalysis> {
        let response = self.generate_with_image(
            image_path,
            "Describe this image in detail (2-3 sentences). Then list 5-10 single-word tags separated by commas on a new line starting with 'Tags:'.",
        )?;

        // Parse response into description and tags
        let (description, tags) = if let Some(tags_idx) = response.to_lowercase().find("tags:") {
            let desc = response[..tags_idx].trim().to_string();
            let tags_str = &response[tags_idx + 5..];
            let tags: Vec<String> = tags_str
                .split(',')
                .map(|t| t.trim().to_lowercase().trim_matches('.').to_string())
                .filter(|t| !t.is_empty() && t.len() < 30)
                .collect();
            (desc, tags)
        } else {
            (response.clone(), Vec::new())
        };

        Ok(ImageAnalysis { description, tags })
    }

    /// Chat about a photo with a custom question.
    pub fn chat_about_image(&self, image_path: &Path, question: &str) -> Result<String> {
        self.generate_with_image(image_path, question)
    }

    /// Core multimodal generation: image + text prompt → text response.
    fn generate_with_image(&self, image_path: &Path, prompt: &str) -> Result<String> {
        // Create mtmd context with the multimodal projector
        let mtmd_params = MtmdContextParams::default();
        let mtmd_ctx = MtmdContext::init_from_file(&self.mmproj_path, &self.model, &mtmd_params)
            .map_err(|e| MlError::Other(format!("mtmd init: {e}")))?;

        if !mtmd_ctx.support_vision() {
            return Err(MlError::Other("Model does not support vision".into()));
        }

        // Load image as bitmap
        let image_path_str = image_path.to_string_lossy();
        let bitmap = MtmdBitmap::from_file(&mtmd_ctx, &image_path_str)
            .map_err(|e| MlError::Other(format!("bitmap load: {e}")))?;

        // Build prompt with media marker
        let full_prompt = format!(
            "<start_of_turn>user\n<__media__>\n{}<end_of_turn>\n<start_of_turn>model\n",
            prompt
        );

        let input_text = MtmdInputText {
            text: full_prompt,
            add_special: true,
            parse_special: true,
        };

        // Tokenize with image
        let chunks = mtmd_ctx
            .tokenize(input_text, &[&bitmap])
            .map_err(|e| MlError::Other(format!("tokenize: {e}")))?;

        // Create LLM context
        let n_tokens = chunks.total_tokens();
        let ctx_size = (n_tokens + 512).max(2048);
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(std::num::NonZero::new(ctx_size as u32));

        let mut ctx = self
            .model
            .new_context(&self.backend, ctx_params)
            .map_err(|e| MlError::Other(format!("context: {e}")))?;

        // Eval image+text chunks through the model
        let n_past = chunks
            .eval_chunks(&mtmd_ctx, &ctx, 0, 0, 512, true)
            .map_err(|e| MlError::Other(format!("eval chunks: {e}")))?;

        // Generate response tokens
        let mut output = String::new();
        let max_tokens = 512;

        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(0.7),
            LlamaSampler::top_p(0.9, 1),
            LlamaSampler::dist(42),
        ]);

        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut n_cur = n_past;

        for _ in 0..max_tokens {
            let token = sampler.sample(&ctx, -1);

            if self.model.is_eog_token(token) {
                break;
            }

            let piece = self
                .model
                .token_to_piece(token, &mut decoder, true, None)
                .map_err(|e| MlError::Other(format!("detokenize: {e}")))?;
            output.push_str(&piece);

            if output.ends_with("<end_of_turn>") {
                output = output.trim_end_matches("<end_of_turn>").to_string();
                break;
            }

            // Prepare next token
            let mut batch = llama_cpp_2::llama_batch::LlamaBatch::new(1, 1);
            batch
                .add(token, n_cur, &[0], true)
                .map_err(|e| MlError::Other(format!("batch: {e}")))?;
            n_cur += 1;

            ctx.decode(&mut batch)
                .map_err(|e| MlError::Other(format!("decode: {e}")))?;
        }

        Ok(output.trim().to_string())
    }
}
