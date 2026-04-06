# Mim

**A local-first, AI-powered photo library manager.**

Named after Mimir, the Norse god of wisdom and memory. Mim runs entirely on your machine -- no cloud, no subscriptions, no data leaving your computer.

![Rust](https://img.shields.io/badge/Rust-2024-orange) ![Tauri](https://img.shields.io/badge/Tauri-v2-blue) ![License](https://img.shields.io/badge/License-MIT-green)

## Features

### Photo Management
- **Multi-folder library** -- Add any folder, including external drives and network mounts
- **Auto-scanning** -- Detects new photos automatically via filesystem watching
- **EXIF extraction** -- Camera make/model, GPS, focal length, aperture, ISO, shutter speed
- **Thumbnails** -- Three sizes (64/256/1024px) generated in parallel with WebP output
- **Timeline/Grid/Masonry views** -- Flexible photo browsing with size slider
- **Albums** -- Create, rename, delete albums; add/remove photos
- **Deduplication** -- BLAKE3 content hash for exact dupes, perceptual hash (pHash) for near-duplicates
- **Search** -- Full-text search across filenames, AI descriptions, tags, and locations

### AI Vision (Local, Offline)
- **Face detection** -- SCRFD-10G via ONNX Runtime, detects faces with bounding boxes and landmarks
- **Face recognition** -- ArcFace embeddings (512-dim), DBSCAN clustering into identities
- **People management** -- Rename people, merge mistaken splits, face crop avatars
- **Gemma 4 vision** -- Multimodal image understanding via llama.cpp (Q4_K_M quantized)
- **AI tagging** -- Auto-generate descriptions and tags for every photo
- **Photo chat** -- Ask questions about your photos in natural language
- **OCR detection** -- PP-OCRv4 text detection to identify photos containing text
- **Scene classification** -- Beach, restaurant, concert, wedding, etc.
- **Aesthetic scoring** -- Photo quality rating (composition, lighting, sharpness)
- **Blur detection** -- Flag blurry photos for cleanup
- **Screenshot detection** -- Separate screenshots from camera photos
- **Dominant colors** -- Extract color palette per photo
- **Event clustering** -- Group photos by time + location into events
- **Reverse geocoding** -- GPS coordinates to city/country names (offline)

### Phone Sync
- **Syncthing integration** -- Embedded Syncthing sidecar for automatic phone-to-PC photo sync
- **Universal** -- Works over WiFi, cellular, across networks
- **File watching** -- Detects new files from any sync tool (Dropbox, OneDrive, etc.)

### Security
- **SQLCipher encryption** -- AES-256 encrypted databases
- **Secure folders** -- Per-folder password protection with Argon2id hashing
- **Locked folder UI** -- Password prompt to access protected photos

### Other
- **Logging** -- Daily rotating log files with 7-day retention
- **Settings** -- Model selection, GPU toggle, about info
- **Cross-platform** -- Linux, macOS, Windows via Tauri v2
- **CUDA support** -- Optional GPU acceleration for face detection and Gemma

## Quick Start

### Prerequisites

- Rust (stable, edition 2024)
- Node.js 18+
- System dependencies:
  ```bash
  # Arch Linux
  sudo pacman -S webkit2gtk-4.1 clang cmake

  # Ubuntu/Debian
  sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev clang cmake

  # Windows (PowerShell)
  winget install Kitware.CMake LLVM.LLVM StrawberryPerl.StrawberryPerl

  # Windows with CUDA GPU support (optional)
  winget install Nvidia.CUDA

  # macOS
  xcode-select --install && brew install cmake
  ```

### Build & Run

```bash
git clone https://github.com/tonybenoy/mim.git
cd mim

# Install frontend dependencies (REQUIRED before first build)
cd mim-tauri/frontend
npm install
cd ../..

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build

# Build with CUDA GPU acceleration (optional)
cargo tauri build --features mim-ml/cuda
```

> **Note:** You must run `npm install` in `mim-tauri/frontend/` before building. Without it, `vite` won't be found and the build will fail.

### First Use

1. Click **+ Add** in the sidebar to add a photo folder
2. Enter the folder path (e.g., `/home/user/Photos`)
3. Photos are scanned and thumbnails generated automatically
4. Click **Faces** to detect faces, **Tag** to generate AI descriptions
5. Use the **Chat** tab to ask questions about your photo library

## Architecture

```
mim/
  mim-core/       Rust library: models, database, scanner, thumbnails, sync
  mim-ml/         ML pipeline: face detection, recognition, clustering, Gemma, OCR, analysis
  mim-tauri/      Tauri desktop app
    src/           Rust backend: commands, state management
    frontend/      SvelteKit 5 + Tailwind CSS 4 frontend
```

### Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop | Tauri v2 (Rust + WebView) |
| Frontend | SvelteKit 5, Svelte 5, Tailwind CSS 4 |
| Database | SQLite via rusqlite (SQLCipher for encryption) |
| Face Detection | SCRFD-10G via ONNX Runtime |
| Face Recognition | ArcFace (w600k_r50) via ONNX Runtime |
| Vision AI | Gemma 4 E4B via llama.cpp (GGUF) |
| OCR | PP-OCRv4 via ONNX Runtime |
| Photo Sync | Syncthing (embedded sidecar) |
| Hashing | BLAKE3 (dedup), pHash (similarity) |
| Design | Neumorphism + Glass Morphism |

### Database Architecture

- **Central DB** (`~/.local/share/mim/mim_central.db`) -- Folder sources, face identities, settings
- **Sidecar DBs** (`{folder}/.mim/mim.db`) -- Per-folder photo metadata, faces, tags, albums

This dual-DB design means photo metadata travels with the folder -- plug in an external drive and Mim instantly has all the metadata without re-scanning.

### ML Models (Auto-Downloaded)

| Model | Size | Purpose |
|-------|------|---------|
| SCRFD-10G | 17 MB | Face detection |
| ArcFace w600k_r50 | 174 MB | Face embeddings |
| Gemma 4 E4B Q4_K_M | 5.3 GB | Vision AI + chat |
| Gemma 4 mmproj | 990 MB | Image understanding |
| PP-OCRv4 det | ~5 MB | Text detection |

Models are downloaded on first use to `~/.local/share/mim/models/`.

## Configuration

Settings are accessible via the gear icon in the top bar:

- **Vision Model** -- Choose between Gemma 4 E4B (best), E2B (lighter), or Gemma 3 4B (legacy)
- **Face Detection Model** -- SCRFD-10G (most accurate), 2.5G (balanced), 500M (fastest)
- **GPU** -- Enable CUDA acceleration when available
- **Phone Sync** -- Enable/configure Syncthing integration

## CUDA Support

To enable GPU acceleration:

```bash
# Build with CUDA features
cargo build --features mim-ml/cuda
```

Requires NVIDIA GPU with CUDA toolkit installed. Falls back to CPU gracefully when GPU is unavailable.

## License

MIT -- see [LICENSE](LICENSE).
