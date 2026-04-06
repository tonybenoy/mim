# Mim User Guide

## Building from Source

### Prerequisites
- **Rust** (stable, edition 2024) — [rustup.rs](https://rustup.rs)
- **Node.js 18+** — [nodejs.org](https://nodejs.org)
- **System dependencies:**
  - **Arch Linux:** `sudo pacman -S webkit2gtk-4.1 clang cmake`
  - **Ubuntu/Debian:** `sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev clang cmake`
  - **Windows:** Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/), then `choco install cmake llvm nodejs strawberryperl`
  - **macOS:** `xcode-select --install && brew install cmake`

### Build Steps

```bash
# 1. Clone the repo
git clone https://github.com/tonybenoy/mim.git
cd mim

# 2. Install frontend dependencies (REQUIRED before first build)
cd mim-tauri/frontend
npm install
cd ../..

# 3. Run in development mode
cargo tauri dev

# 4. Or build for production
cargo tauri build

# 5. With CUDA GPU acceleration (optional, needs CUDA toolkit)
cargo tauri build --features mim-ml/cuda
```

**Important:** You must run `npm install` in `mim-tauri/frontend/` before your first build. The Tauri build process runs `npm run build` which requires `vite` and other dependencies to be installed.

## Getting Started

### Adding Your First Folder

1. Launch Mim
2. Click **+ Add** in the sidebar
3. Enter the full path to your photo folder:
   - Linux: `/home/username/Photos`
   - Windows (WSL2): `/mnt/c/Users/username/Pictures`
   - External drive: `/mnt/d/Photos`
4. Mim scans the folder, extracts EXIF data, and generates thumbnails
5. Your photos appear in the Library view

### Browsing Photos

- **Timeline** -- Photos grouped by date, scroll to navigate through time
- **Grid** -- Simple grid layout with adjustable thumbnail size
- **Masonry** -- Pinterest-style varying-height layout
- Use the **size slider** in the toolbar to adjust thumbnail size
- Click any photo to open the **detail overlay** with full EXIF info

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Escape | Close photo detail / modal |
| Left/Right Arrow | Navigate between photos |

## AI Features

### Face Detection & People

1. Select a folder in the sidebar
2. Click **Faces** to start face detection
3. Mim downloads SCRFD (~17 MB) and ArcFace (~174 MB) on first run
4. Faces are detected, embeddings extracted, and clustered into identities
5. Switch to the **People** tab to see detected people
6. Click a person to see all their photos
7. **Rename** to give them a real name
8. **Merge** if the same person was split into two groups

### AI Tagging & Descriptions

1. Click **Tag** on a folder in the sidebar
2. Mim downloads Gemma 4 (~6.3 GB total) on first run
3. Each photo gets:
   - A natural language description
   - 5-10 keyword tags
4. These become searchable in the Search tab

### Photo Chat

1. Go to the **Chat** tab (bottom bar)
2. Type a natural language query:
   - "sunset photos"
   - "screenshots from January"
   - "photos with dogs"
3. Mim searches your library and shows matching photos
4. For specific photos, Gemma provides detailed descriptions

### Photo Analysis

Run **Analyze** on a folder to compute:
- Dominant colors (5-color palette)
- Blur score (flags blurry photos)
- Screenshot detection
- Time-of-day classification
- Perceptual hash (for finding similar photos)
- Event clustering (groups photos into events by time/location)
- Reverse geocoding (GPS to city names)

## Phone Sync

### Setup

1. Go to **Settings** (gear icon)
2. Enable **Phone Sync**
3. Mim downloads and starts Syncthing in the background (~15 MB)
4. Install Syncthing on your phone:
   - **Android**: [Syncthing on F-Droid](https://f-droid.org/packages/com.nutomic.syncthingandroid/) or Play Store
   - **iOS**: [Mobius Sync](https://apps.apple.com/app/mobius-sync/id1539203216) (paid)
5. In your phone's Syncthing app, add a new device using Mim's device ID
6. Share your Camera folder with Mim
7. Photos sync automatically whenever your phone and PC are on the same network (or via Syncthing relays over the internet)

### How It Works

- Mim runs Syncthing as an invisible background process
- Photos are synced to a local folder on your PC
- Mim's file watcher detects new photos and auto-imports them
- Works over WiFi, cellular, or across different networks via relay servers

## Security

### Locking a Folder

1. Right-click a folder in the sidebar (or use per-folder settings)
2. Choose **Lock Folder**
3. Set a password
4. The folder's database is encrypted with AES-256 (SQLCipher)
5. You'll need the password each time you access that folder

### How Encryption Works

- Password is hashed with **Argon2id** (memory-hard, resistant to GPU cracking)
- The raw password is used as the **SQLCipher key** for the sidecar database
- Even if someone copies the `.mim/mim.db` file, they can't read it without the password
- The central database can also be encrypted with a master password

## Albums

1. Go to the **Albums** tab
2. Click **+ New Album**
3. Name your album
4. Add photos from the photo detail view
5. Albums support rename and delete

## Search

The search bar (top of the screen) searches across:
- Filenames
- AI descriptions (run tagging first)
- Tags
- Location names

Type naturally -- "beach vacation", "birthday party", "code screenshots".

## Settings

Access via the gear icon (top right):
- **Vision Model** -- Choose AI model size vs speed tradeoff
- **Face Detection** -- Choose accuracy vs speed
- **GPU** -- Enable CUDA for faster AI processing
- **About** -- Version info and project description
- **Exit** -- Quit Mim

## Troubleshooting

### Photos not showing
- Ensure the asset protocol has access: check that thumbnails exist in `{folder}/.mim/thumbnails/`
- Try rescanning the folder (click Scan on the folder)

### Slow scanning
- WSL2's `/mnt/c/` filesystem is slow due to the 9P protocol bridge
- For better performance, copy photos to the Linux filesystem (`~/Photos/`)

### Face detection finds 0 faces
- Ensure photos are actual camera photos (not tiny icons)
- Check the logs at `~/.local/share/mim/logs/`

### AI tagging fails to download
- Check internet connection
- Verify HuggingFace is accessible
- Check logs for specific HTTP errors
- Delete partial downloads in `~/.local/share/mim/models/*.tmp`
