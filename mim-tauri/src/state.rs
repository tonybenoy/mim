use mim_core::db::DbPool;
use mim_core::sync::SyncManager;
use mim_core::watcher::FolderWatcher;
use mim_core::Config;
use mim_ml::{DownloadProgress, FacePipeline, GemmaVision};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::OnceCell;

pub struct AppState {
    pub config: Config,
    pub central_db: DbPool,
    central_db_arc: Arc<DbPool>,
    pub sidecar_dbs: Arc<Mutex<HashMap<String, Arc<DbPool>>>>,
    face_pipeline: OnceCell<Arc<FacePipeline>>,
    gemma: OnceCell<Arc<GemmaVision>>,
    sync_manager: Mutex<SyncManager>,
    watchers: Mutex<HashMap<String, FolderWatcher>>,
}

impl AppState {
    pub fn new() -> mim_core::Result<Self> {
        let config = Config::new();
        config.ensure_dirs()?;

        let central_db = DbPool::open_central(&config.central_db_path)?;
        let central_db_arc = Arc::new(DbPool::open_central(&config.central_db_path)?);
        let sync_manager = SyncManager::new(&config.data_dir);

        Ok(AppState {
            config,
            central_db,
            central_db_arc,
            sidecar_dbs: Arc::new(Mutex::new(HashMap::new())),
            face_pipeline: OnceCell::new(),
            gemma: OnceCell::new(),
            sync_manager: Mutex::new(sync_manager),
            watchers: Mutex::new(HashMap::new()),
        })
    }

    // ── Sidecar DB ──────────────────────────────────────────

    pub fn get_or_open_sidecar(&self, source_path: &str) -> mim_core::Result<Arc<DbPool>> {
        self.get_or_open_sidecar_with_key(source_path, None)
    }

    pub fn get_or_open_sidecar_with_key(
        &self,
        source_path: &str,
        encryption_key: Option<&str>,
    ) -> mim_core::Result<Arc<DbPool>> {
        let mut sidecars = self.sidecar_dbs.lock();
        if let Some(db) = sidecars.get(source_path) {
            return Ok(Arc::clone(db));
        }

        let db = Arc::new(DbPool::open_sidecar_with_key(
            std::path::Path::new(source_path),
            encryption_key,
        )?);
        sidecars.insert(source_path.to_string(), Arc::clone(&db));
        Ok(db)
    }

    pub fn close_sidecar(&self, source_path: &str) {
        let mut sidecars = self.sidecar_dbs.lock();
        sidecars.remove(source_path);
    }

    pub fn central_db_arc(&self) -> Arc<DbPool> {
        Arc::clone(&self.central_db_arc)
    }

    // ── ML Pipelines ────────────────────────────────────────

    pub async fn get_or_init_pipeline_with_handle(
        &self,
        app_handle: &AppHandle,
    ) -> mim_ml::Result<Arc<FacePipeline>> {
        let models_dir = self.config.models_dir.clone();
        let handle = app_handle.clone();

        self.face_pipeline
            .get_or_try_init(|| async {
                let (tx, mut rx) = tokio::sync::watch::channel::<Option<DownloadProgress>>(None);

                let emit_handle = handle.clone();
                tokio::spawn(async move {
                    while rx.changed().await.is_ok() {
                        if let Some(ref progress) = *rx.borrow() {
                            let pct = if progress.total > 0 {
                                (progress.downloaded as f64 / progress.total as f64 * 100.0) as u32
                            } else {
                                0
                            };
                            let _ = emit_handle.emit(
                                "download-progress",
                                serde_json::json!({
                                    "filename": progress.filename,
                                    "downloaded": progress.downloaded,
                                    "total": progress.total,
                                    "pct": pct,
                                }),
                            );
                        }
                    }
                });

                let pipeline = FacePipeline::new_with_progress(&models_dir, Some(tx)).await?;
                Ok(Arc::new(pipeline))
            })
            .await
            .map(Arc::clone)
    }

    pub async fn get_or_init_gemma_with_handle(
        &self,
        app_handle: &AppHandle,
    ) -> mim_ml::Result<Arc<GemmaVision>> {
        let models_dir = self.config.models_dir.clone();
        let handle = app_handle.clone();

        self.gemma
            .get_or_try_init(|| async {
                let (tx, mut rx) = tokio::sync::watch::channel::<Option<DownloadProgress>>(None);

                let emit_handle = handle.clone();
                tokio::spawn(async move {
                    while rx.changed().await.is_ok() {
                        if let Some(ref progress) = *rx.borrow() {
                            let pct = if progress.total > 0 {
                                (progress.downloaded as f64 / progress.total as f64 * 100.0) as u32
                            } else {
                                0
                            };
                            let _ = emit_handle.emit(
                                "download-progress",
                                serde_json::json!({
                                    "filename": progress.filename,
                                    "downloaded": progress.downloaded,
                                    "total": progress.total,
                                    "pct": pct,
                                }),
                            );
                        }
                    }
                });

                let gemma = GemmaVision::new_with_progress(&models_dir, Some(tx)).await?;
                Ok(Arc::new(gemma))
            })
            .await
            .map(Arc::clone)
    }

    // ── Syncthing Sync ──────────────────────────────────────

    pub async fn ensure_sync_binary(&self) -> mim_core::Result<()> {
        // Clone what we need before awaiting
        let bin_path = self.sync_manager.lock().is_installed();
        if bin_path {
            return Ok(());
        }
        // SyncManager::ensure_binary is async so we can't hold the lock
        // Create a temporary manager with the same config
        let data_dir = self.config.data_dir.clone();
        let temp_manager = mim_core::sync::SyncManager::new(&data_dir);
        temp_manager.ensure_binary().await
    }

    pub fn start_sync(&self) -> mim_core::Result<()> {
        let mut manager = self.sync_manager.lock();
        manager.start()
    }

    pub fn stop_sync(&self) {
        let mut manager = self.sync_manager.lock();
        manager.stop();
    }

    pub fn is_sync_running(&self) -> bool {
        let mut manager = self.sync_manager.lock();
        manager.is_running()
    }

    pub fn is_sync_installed(&self) -> bool {
        let manager = self.sync_manager.lock();
        manager.is_installed()
    }

    pub async fn get_sync_device_id(&self) -> mim_core::Result<String> {
        let (api_port, api_key) = {
            let manager = self.sync_manager.lock();
            (manager.api_port(), manager.api_key().to_string())
        };
        // Query the REST API without holding the lock
        let url = format!("http://127.0.0.1:{}/rest/system/status", api_port);
        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("X-API-Key", &api_key)
            .send()
            .await
            .map_err(|e| mim_core::Error::Other(format!("api: {e}")))?;
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| mim_core::Error::Other(format!("json: {e}")))?;
        body["myID"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| mim_core::Error::Other("no device ID".into()))
    }

    pub fn sync_api_port(&self) -> u16 {
        let manager = self.sync_manager.lock();
        manager.api_port()
    }

    pub async fn add_sync_folder(&self, folder_path: &str, label: &str) -> mim_core::Result<()> {
        let (api_port, api_key) = {
            let manager = self.sync_manager.lock();
            (manager.api_port(), manager.api_key().to_string())
        };
        let folder_id = format!("mim-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let folder_config = serde_json::json!({
            "id": folder_id,
            "label": label,
            "path": folder_path,
            "type": "receiveonly",
            "rescanIntervalS": 60,
            "fsWatcherEnabled": true,
        });
        let url = format!("http://127.0.0.1:{}/rest/config/folders", api_port);
        let client = reqwest::Client::new();
        client
            .post(&url)
            .header("X-API-Key", &api_key)
            .json(&folder_config)
            .send()
            .await
            .map_err(|e| mim_core::Error::Other(format!("add folder: {e}")))?;
        tracing::info!("Added sync folder: {} -> {}", folder_id, folder_path);
        Ok(())
    }

    // ── File Watchers ───────────────────────────────────────

    pub fn add_watcher(&self, path: String, watcher: FolderWatcher) {
        let mut watchers = self.watchers.lock();
        watchers.insert(path, watcher);
    }

    pub fn remove_watcher(&self, path: &str) {
        let mut watchers = self.watchers.lock();
        watchers.remove(path);
    }
}
