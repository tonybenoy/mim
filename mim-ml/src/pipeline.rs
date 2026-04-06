use crate::clustering::{ClusterResult, FaceClusterer};
use crate::detection::FaceDetector;
use crate::error::Result;
use crate::models::ModelManager;
use crate::recognition::FaceRecognizer;
use mim_core::db::{DbPool, FaceIdentitiesDb, FacesDb, PhotosDb};
use mim_core::models::{Face, FaceIdentity, Photo};
use parking_lot::Mutex;
use serde::Serialize;
use std::path::Path;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize)]
pub struct ProcessingProgress {
    pub total: usize,
    pub processed: usize,
    pub faces_found: usize,
}

pub struct FacePipeline {
    detector: Mutex<FaceDetector>,
    recognizer: Mutex<FaceRecognizer>,
    clusterer: FaceClusterer,
}

impl FacePipeline {
    pub async fn new(models_dir: &Path) -> Result<Self> {
        Self::new_with_options(models_dir, None, None).await
    }

    pub async fn new_with_progress(
        models_dir: &Path,
        progress_tx: Option<tokio::sync::watch::Sender<Option<crate::models::DownloadProgress>>>,
    ) -> Result<Self> {
        Self::new_with_options(models_dir, progress_tx, None).await
    }

    pub async fn new_with_options(
        models_dir: &Path,
        progress_tx: Option<tokio::sync::watch::Sender<Option<crate::models::DownloadProgress>>>,
        scrfd_model_id: Option<&str>,
    ) -> Result<Self> {
        let mut manager = ModelManager::new(models_dir.to_path_buf());
        if let Some(id) = scrfd_model_id {
            manager = manager.with_scrfd_model(id);
        }
        if let Some(tx) = progress_tx {
            manager = manager.with_progress(tx);
        }

        let scrfd_path = manager.ensure_scrfd().await?;
        let arcface_path = manager.ensure_arcface().await?;

        let detector = FaceDetector::new(&scrfd_path)?;
        let recognizer = FaceRecognizer::new(&arcface_path)?;
        let clusterer = FaceClusterer::default();

        info!("Face pipeline initialized");
        Ok(Self {
            detector: Mutex::new(detector),
            recognizer: Mutex::new(recognizer),
            clusterer,
        })
    }

    pub fn process_photo(
        &self,
        photo: &Photo,
        root: &Path,
        db: &DbPool,
    ) -> Result<usize> {
        let source_path = root.join(&photo.relative_path);
        if !source_path.exists() {
            warn!("Photo file missing: {}", source_path.display());
            return Ok(0);
        }

        let img = image::open(&source_path)?;
        let detected = self.detector.lock().detect(&img)?;

        if detected.is_empty() {
            PhotosDb::mark_faces_processed(db.writer(), &photo.id)?;
            return Ok(0);
        }

        let mut recognizer = self.recognizer.lock();
        for det in &detected {
            let mut face = Face::new(
                photo.id.clone(),
                det.bbox[0],
                det.bbox[1],
                det.bbox[2],
                det.bbox[3],
                det.confidence,
            );

            face.landmarks = Some(det.landmarks.to_vec());

            match recognizer.get_embedding(&img, &det.landmarks) {
                Ok(embedding) => {
                    face.embedding = Some(embedding);
                }
                Err(e) => {
                    warn!("Failed to get embedding for face in {}: {}", photo.filename, e);
                }
            }

            FacesDb::insert(db.writer(), &face)?;
        }

        PhotosDb::mark_faces_processed(db.writer(), &photo.id)?;
        Ok(detected.len())
    }

    pub fn process_folder(
        &self,
        root: &Path,
        db: &DbPool,
        progress_callback: impl Fn(&ProcessingProgress),
    ) -> Result<ProcessingProgress> {
        let photos = PhotosDb::list_unprocessed_faces(db.reader())?;
        let total = photos.len();

        info!("Processing {} unprocessed photos for faces", total);

        let mut progress = ProcessingProgress {
            total,
            processed: 0,
            faces_found: 0,
        };

        for photo in &photos {
            match self.process_photo(photo, root, db) {
                Ok(count) => {
                    progress.faces_found += count;
                }
                Err(e) => {
                    warn!("Failed to process {}: {}", photo.filename, e);
                    let _ = PhotosDb::mark_faces_processed(db.writer(), &photo.id);
                }
            }
            progress.processed += 1;
            progress_callback(&progress);
        }

        info!(
            "Face processing complete: {}/{} photos, {} faces found",
            progress.processed, progress.total, progress.faces_found
        );

        Ok(progress)
    }

    pub fn cluster_folder(
        &self,
        sidecar_db: &DbPool,
        central_db: &DbPool,
    ) -> Result<Vec<ClusterResult>> {
        let faces = FacesDb::list_all_with_embeddings(sidecar_db.reader())?;

        if faces.is_empty() {
            info!("No faces with embeddings to cluster");
            return Ok(Vec::new());
        }

        let face_ids: Vec<String> = faces.iter().map(|f| f.id.clone()).collect();
        let embeddings: Vec<Vec<f32>> = faces
            .iter()
            .filter_map(|f| f.embedding.clone())
            .collect();

        if embeddings.len() != face_ids.len() {
            warn!("Some faces missing embeddings, clustering subset");
        }

        let clusters = self.clusterer.cluster(&face_ids, &embeddings)?;

        FacesDb::clear_identities(sidecar_db.writer())?;
        FaceIdentitiesDb::delete_all(central_db.writer())?;

        for cluster in &clusters {
            let mut identity = FaceIdentity::new(format!(
                "Person {}",
                &cluster.identity_id[..6].to_uppercase()
            ));
            identity.id = cluster.identity_id.clone();
            identity.representative_embedding = Some(cluster.centroid.clone());
            identity.photo_count = cluster.face_ids.len() as u32;

            FaceIdentitiesDb::insert(central_db.writer(), &identity)?;

            for face_id in &cluster.face_ids {
                FacesDb::update_identity(
                    sidecar_db.writer(),
                    face_id,
                    &cluster.identity_id,
                    1.0,
                )?;
            }
        }

        info!("Created {} face identities", clusters.len());
        Ok(clusters)
    }
}
