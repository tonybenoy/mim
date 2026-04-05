# Mim Technical Documentation

## Architecture Overview

Mim is a Cargo workspace with three crates and a SvelteKit frontend:

```
mim/
  mim-core/          Core library (models, DB, scanner, thumbnails, sync, crypto)
  mim-ml/            ML pipeline (face detection, recognition, clustering, Gemma, OCR, analysis)
  mim-tauri/         Tauri v2 desktop application
    src/             Rust backend (commands, state)
    frontend/        SvelteKit 5 + Tailwind CSS 4
```

## Crate: mim-core

### Database (mim-core/src/db/)

**Dual-database architecture:**
- **Central DB** at `~/.local/share/mim/mim_central.db` -- stores folder sources, face identities, settings
- **Sidecar DBs** at `{folder}/.mim/mim.db` -- per-folder photo metadata, faces, tags, albums

Both use SQLite with WAL journaling. Optional SQLCipher encryption (AES-256-CBC with PBKDF2).

**Connection pooling:** `DbPool` provides a single write connection (mutex-protected) and multiple read connections (4 for sidecar, 2 for central).

**Tables:**
| Table | DB | Purpose |
|-------|----|---------|
| `photos` | Sidecar | Photo metadata, EXIF, AI fields, processing flags (41 columns) |
| `faces` | Sidecar | Face detections with bounding boxes, landmarks, embeddings |
| `tags` | Sidecar | Tag definitions (name, category) |
| `photo_tags` | Sidecar | Many-to-many photo-tag associations with confidence |
| `albums` | Sidecar | Album definitions (manual, smart, favorites) |
| `album_photos` | Sidecar | Album membership with ordering |
| `events` | Sidecar | Auto-detected event clusters |
| `folders` | Sidecar | Hierarchical folder structure |
| `folder_sources` | Central | Monitored directories with lock status |
| `face_identities` | Central | Named face clusters with representative embeddings |
| `settings` | Central | Key-value configuration store |

### Scanner (mim-core/src/scanner/)

- `discover_files()` -- Walks directory tree, filters by 40+ image and 7 video extensions
- `scan_folder()` -- Parallel processing via rayon: BLAKE3 hash, EXIF extraction, dimension reading
- `extract_exif()` -- Parses camera make/model, lens, focal length, aperture, shutter speed, ISO, GPS
- Streaming BLAKE3 hash (64KB chunks) for memory efficiency on large files

### Thumbnails (mim-core/src/thumbnail/)

- Three sizes: Micro (64px), Grid (256px), Preview (1024px)
- Aspect-ratio-preserving resize with Lanczos3 filtering
- WebP output with PNG fallback
- Path format: `{folder}/.mim/thumbnails/{hash_prefix}/{hash}_{size}.webp`
- Face crop generation: 128x128 square crops centered on detected faces

### Sync (mim-core/src/sync.rs)

Manages an embedded Syncthing instance:
- Downloads Syncthing binary on first use (~15MB)
- Configures as receive-only node
- REST API for device management and folder sharing
- Auto-starts in background, stops on app exit

### File Watcher (mim-core/src/watcher.rs)

- Uses `notify` crate for cross-platform filesystem events
- Batches events over 2-second windows to avoid rapid-fire processing
- Filters hidden files and `.mim` directory
- Emits `FilesChanged` and `FilesRemoved` events

### Security (mim-core/src/crypto.rs)

- Password hashing: Argon2id with random salt
- SQLCipher integration: per-database encryption keys via `PRAGMA key`
- Locked folders: password hash in central DB, raw password as SQLCipher key for sidecar

## Crate: mim-ml

### Face Detection (mim-ml/src/detection.rs)

- **Model:** SCRFD-10G with BNKPS landmarks (17MB ONNX)
- **Input:** 640x640 letterboxed RGB image
- **Output:** 9 tensors (3 strides x {scores, bboxes, landmarks})
- **Post-processing:** Anchor decode, confidence filter (0.5), NMS (IoU 0.4)
- **Output format:** Bounding box [x,y,w,h] + 5 landmarks + confidence per face

### Face Recognition (mim-ml/src/recognition.rs)

- **Model:** ArcFace w600k_r50 (174MB ONNX)
- **Input:** 112x112 aligned face crop
- **Alignment:** Procrustes similarity transform from 5 detected landmarks to canonical reference points
- **Output:** 512-dim L2-normalized embedding vector
- **Storage:** Embeddings serialized as BLOB via bytemuck

### Face Clustering (mim-ml/src/clustering.rs)

- **Algorithm:** DBSCAN (manual implementation, ~100 lines)
- **Distance:** L2 distance on L2-normalized embeddings (equivalent to cosine distance)
- **Parameters:** eps=1.0 (cosine similarity >= 0.5), min_samples=2
- **Output:** Cluster IDs, face assignments, centroid embeddings
- **Merge support:** Weighted average embedding recomputation on identity merge

### Gemma Vision (mim-ml/src/gemma.rs)

- **Model:** Gemma 4 E4B (5.3GB GGUF Q4_K_M quantized)
- **Projector:** mmproj-f16 (990MB) for image understanding
- **Runtime:** llama.cpp via llama-cpp-2 Rust bindings
- **Multimodal:** Uses mtmd API -- `MtmdBitmap::from_file` + `MtmdContext::tokenize` + `eval_chunks`
- **Capabilities:** Image description, tagging, OCR, visual Q&A, scene understanding
- **Sampling:** Temperature 0.7, top-p 0.9, max 512 tokens

### Photo Analysis (mim-ml/src/analysis.rs)

Pure algorithmic analysis (no model downloads):

| Feature | Method | Output |
|---------|--------|--------|
| Dominant colors | K-means (k=5) on 64x64 resized | 5 hex color codes |
| Perceptual hash | 32x32 DCT, 8x8 coefficient threshold | 16-char hex string |
| Blur detection | Laplacian variance | 0-1 score |
| Screenshot detection | Filename + aspect ratio + status bar check | boolean |
| Time of day | EXIF hour + brightness fallback | dawn/morning/midday/afternoon/golden_hour/blue_hour/night |
| Event clustering | 3-hour time gap + 50km GPS split | Event IDs |
| Reverse geocoding | Nearest of 130 major cities within 100km | City, Country |

### Pipeline Orchestration (mim-ml/src/pipeline.rs)

`FacePipeline` coordinates detect -> recognize -> store:
1. Load image from disk
2. SCRFD detection (via Mutex<FaceDetector>)
3. For each face: align crop via landmarks, ArcFace embedding (via Mutex<FaceRecognizer>)
4. Insert Face records to sidecar DB
5. Mark photo as `faces_processed = true`
6. DBSCAN clustering across all embeddings
7. Create/update FaceIdentity records in central DB

### Model Management (mim-ml/src/models.rs)

- Auto-downloads from HuggingFace on first use
- Streaming download with progress reporting via `tokio::sync::watch`
- Atomic rename (`.tmp` -> final) to prevent corrupt partial downloads
- Storage: `~/.local/share/mim/models/`

## Crate: mim-tauri

### State Management (mim-tauri/src/state.rs)

`AppState` is the central state container:
- `central_db: DbPool` -- always open
- `sidecar_dbs: Arc<Mutex<HashMap>>` -- lazy-opened per folder
- `face_pipeline: OnceCell<Arc<FacePipeline>>` -- lazy ML init
- `gemma: OnceCell<Arc<GemmaVision>>` -- lazy Gemma init
- `sync_manager: Mutex<SyncManager>` -- Syncthing sidecar
- `watchers: Mutex<HashMap>` -- folder watchers

### Commands (mim-tauri/src/commands/)

| Module | Commands |
|--------|----------|
| `mod.rs` | add_folder, get_folders, remove_folder, scan_folder, get_photos, get_photo_detail, get_photo_count, get_thumbnail_url, search_photos, find_duplicates, share_photo |
| `faces.rs` | process_faces, cluster_faces, get_faces_for_photo, get_identities, get_identities_with_avatars, rename_identity, merge_identities |
| `gemma.rs` | tag_photos, chat_about_photo |
| `albums.rs` | create_album, get_albums, add_to_album, remove_from_album, get_album_photos, delete_album, rename_album |
| `secure.rs` | lock_folder, unlock_folder, verify_folder_password, open_locked_folder |
| `analysis.rs` | analyze_folder, find_similar_photos, get_events, get_photo_colors |
| `sync.rs` | setup_sync, get_sync_status, stop_sync, add_sync_folder |
| `watch.rs` | watch_folder, unwatch_folder |

### Frontend (mim-tauri/frontend/)

Built with SvelteKit 5 (Svelte 5 runes), Tailwind CSS 4.

**Components:**
| Component | Purpose |
|-----------|---------|
| `TopBar.svelte` | Logo, search bar, settings button |
| `BottomBar.svelte` | 5-tab navigation with animated indicator |
| `Sidebar.svelte` | Folder tree with per-folder actions and progress |
| `PhotoGrid.svelte` | Timeline/grid/masonry views with scroll date indicator |
| `PhotoDetail.svelte` | Full photo overlay with EXIF, faces, chat |
| `PeopleView.svelte` | Face identity grid with avatars, merge, rename |
| `AlbumsView.svelte` | Album CRUD with photo grid |
| `SearchView.svelte` | Debounced search with result grid |
| `ChatView.svelte` | Library-wide natural language photo search |
| `Settings.svelte` | Model selection, GPU, about, exit |

**Design System:**
- Warm pearl palette (light) / deep ocean palette (dark)
- Neumorphic shadows (raised, pressed, soft)
- Glass morphism (blur + saturation + transparency)
- Spring animations via Svelte motion
- Photo cards with lift/glow hover effects
- Accent gradient for brand elements

## Performance Considerations

- **WSL2 filesystem:** `/mnt/c/` is slow (9P protocol). Recommend native Linux paths.
- **Streaming hash:** BLAKE3 hashes files in 64KB chunks, not loading entire file.
- **Parallel thumbnails:** rayon parallelizes thumbnail generation across CPU cores.
- **Lazy ML init:** ONNX/Gemma models loaded only on first use, not at startup.
- **Non-blocking logging:** tracing-appender uses non-blocking writer.
- **Batch events:** File watcher batches events over 2-second windows.
- **Connection pooling:** Multiple SQLite read connections reduce lock contention.
