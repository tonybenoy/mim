use crate::state::AppState;
use serde::Serialize;
use tauri::State;
use tracing::info;

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub filename: String,
    pub size: u64,
    pub exists: bool,
    pub purpose: String,
    pub is_custom: bool,
}

#[tauri::command]
pub async fn list_models(
    state: State<'_, AppState>,
) -> Result<Vec<ModelInfo>, String> {
    let models_dir = &state.config.models_dir;
    let mut models = Vec::new();

    let known_models = [
        ("SCRFD-10G", "scrfd_10g_bnkps.onnx", "Face detection"),
        ("ArcFace w600k", "w600k_r50.onnx", "Face recognition"),
        ("Gemma 4 E4B", "gemma-4-e4b-it-Q4_K_M.gguf", "Vision AI + chat"),
        ("Gemma 4 mmproj", "mmproj-gemma-4-e4b-it-f16.gguf", "Image understanding"),
        ("PP-OCRv4 det", "ppocr_det.onnx", "Text detection"),
    ];

    for (name, filename, purpose) in &known_models {
        let path = models_dir.join(filename);
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        models.push(ModelInfo {
            name: name.to_string(),
            filename: filename.to_string(),
            size,
            exists: path.exists(),
            purpose: purpose.to_string(),
            is_custom: false,
        });
    }

    // Scan for custom models (any .onnx or .gguf not in the known list)
    if let Ok(entries) = std::fs::read_dir(models_dir) {
        let known_filenames: Vec<&str> = known_models.iter().map(|(_, f, _)| *f).collect();
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if (ext == "onnx" || ext == "gguf") && !known_filenames.contains(&path.file_name().unwrap().to_str().unwrap_or("")) {
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                    models.push(ModelInfo {
                        name: filename.clone(),
                        filename,
                        size,
                        exists: true,
                        purpose: "Custom model".to_string(),
                        is_custom: true,
                    });
                }
            }
        }
    }

    Ok(models)
}

#[tauri::command]
pub async fn delete_model(
    filename: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path = state.config.models_dir.join(&filename);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("Failed to delete: {e}"))?;
        info!("Deleted model: {}", filename);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_models_dir(
    state: State<'_, AppState>,
) -> Result<String, String> {
    Ok(state.config.models_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn import_custom_model(
    source_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let source = std::path::Path::new(&source_path);
    if !source.exists() {
        return Err("File not found".into());
    }

    let filename = source.file_name()
        .ok_or("Invalid filename")?
        .to_string_lossy()
        .to_string();

    let dest = state.config.models_dir.join(&filename);
    std::fs::copy(source, &dest).map_err(|e| format!("Copy failed: {e}"))?;

    info!("Imported custom model: {} -> {}", source_path, dest.display());
    Ok(filename)
}
