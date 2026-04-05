use crate::error::{MlError, Result};
use serde::Serialize;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct ClusterResult {
    pub identity_id: String,
    pub face_ids: Vec<String>,
    pub centroid: Vec<f32>,
}

pub struct FaceClusterer {
    /// Max L2 distance between faces in the same cluster.
    /// For L2-normalized vectors: dist² = 2(1 - cos_sim)
    /// eps=1.0 → cos_sim ≥ 0.5, eps=0.8 → cos_sim ≥ 0.68
    eps: f32,
    min_samples: usize,
}

impl FaceClusterer {
    pub fn new(eps: f32, min_samples: usize) -> Self {
        Self { eps, min_samples }
    }

    pub fn cluster(
        &self,
        face_ids: &[String],
        embeddings: &[Vec<f32>],
    ) -> Result<Vec<ClusterResult>> {
        let n = embeddings.len();
        if n == 0 {
            return Err(MlError::NoFaces);
        }

        info!(
            "Clustering {} faces, eps={}, min_samples={}",
            n, self.eps, self.min_samples
        );

        // DBSCAN
        // Labels: None = unvisited/noise, Some(id) = cluster id
        let mut labels: Vec<Option<usize>> = vec![None; n];
        let mut cluster_id = 0usize;

        for i in 0..n {
            if labels[i].is_some() {
                continue;
            }

            let neighbors = self.range_query(embeddings, i);

            if neighbors.len() < self.min_samples {
                // noise — leave as None
                continue;
            }

            labels[i] = Some(cluster_id);
            let mut seed_set: Vec<usize> = neighbors;
            let mut j = 0;

            while j < seed_set.len() {
                let q = seed_set[j];
                j += 1;

                if labels[q].is_none() {
                    labels[q] = Some(cluster_id);

                    let q_neighbors = self.range_query(embeddings, q);
                    if q_neighbors.len() >= self.min_samples {
                        for &nn in &q_neighbors {
                            if !seed_set.contains(&nn) {
                                seed_set.push(nn);
                            }
                        }
                    }
                }
            }

            cluster_id += 1;
        }

        // Build results
        let mut results = Vec::new();
        let noise_count = labels.iter().filter(|l| l.is_none()).count();

        for cid in 0..cluster_id {
            let members: Vec<usize> = labels
                .iter()
                .enumerate()
                .filter(|(_, l)| **l == Some(cid))
                .map(|(i, _)| i)
                .collect();

            if members.is_empty() {
                continue;
            }

            let dim = embeddings[0].len();
            let cluster_face_ids: Vec<String> =
                members.iter().map(|&i| face_ids[i].clone()).collect();

            // Compute L2-normalized centroid
            let mut centroid = vec![0.0f32; dim];
            for &idx in &members {
                for (j, val) in embeddings[idx].iter().enumerate() {
                    centroid[j] += val;
                }
            }
            let count = members.len() as f32;
            for val in centroid.iter_mut() {
                *val /= count;
            }
            let norm: f32 = centroid.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 1e-10 {
                for val in centroid.iter_mut() {
                    *val /= norm;
                }
            }

            results.push(ClusterResult {
                identity_id: Uuid::new_v4().to_string(),
                face_ids: cluster_face_ids,
                centroid,
            });
        }

        info!(
            "Clustering complete: {} clusters, {} noise faces",
            results.len(),
            noise_count
        );

        Ok(results)
    }

    /// Find all points within eps L2 distance of point i.
    fn range_query(&self, embeddings: &[Vec<f32>], i: usize) -> Vec<usize> {
        let mut neighbors = Vec::new();
        let ei = &embeddings[i];
        let eps_sq = self.eps * self.eps;

        for (j, ej) in embeddings.iter().enumerate() {
            if i == j {
                continue;
            }
            let dist_sq: f32 = ei.iter().zip(ej.iter()).map(|(a, b)| (a - b) * (a - b)).sum();
            if dist_sq <= eps_sq {
                neighbors.push(j);
            }
        }

        neighbors
    }
}

impl Default for FaceClusterer {
    fn default() -> Self {
        Self::new(1.0, 2)
    }
}
