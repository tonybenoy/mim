use directories::ProjectDirs;
use std::path::PathBuf;

pub struct Config {
    pub data_dir: PathBuf,
    pub central_db_path: PathBuf,
    pub models_dir: PathBuf,
    pub thumbnail_cache_dir: PathBuf,
}

impl Config {
    pub fn new() -> Self {
        let dirs = ProjectDirs::from("com", "mim", "mim")
            .expect("could not determine app data directory");

        let data_dir = dirs.data_dir().to_path_buf();

        Config {
            central_db_path: data_dir.join("mim_central.db"),
            models_dir: data_dir.join("models"),
            thumbnail_cache_dir: data_dir.join("thumbnail_cache"),
            data_dir,
        }
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.models_dir)?;
        std::fs::create_dir_all(&self.thumbnail_cache_dir)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
