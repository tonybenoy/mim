use rusqlite::Connection;
use crate::Result;

pub fn run_sidecar_migrations(conn: &mut Connection) -> Result<()> {
    conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
    ")?;

    conn.execute_batch(SIDECAR_SCHEMA)?;

    // Incremental migrations for existing databases
    run_sidecar_incremental(conn)?;

    Ok(())
}

pub fn run_central_migrations(conn: &mut Connection) -> Result<()> {
    conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
    ")?;

    conn.execute_batch(CENTRAL_SCHEMA)?;
    run_central_incremental(conn)?;
    Ok(())
}

fn run_central_incremental(conn: &mut Connection) -> Result<()> {
    let columns: Vec<String> = conn
        .prepare("PRAGMA table_info(folder_sources)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();

    let migrations: &[(&str, &str)] = &[
        ("is_locked", "ALTER TABLE folder_sources ADD COLUMN is_locked INTEGER NOT NULL DEFAULT 0"),
        ("password_hash", "ALTER TABLE folder_sources ADD COLUMN password_hash TEXT"),
    ];

    for (col, sql) in migrations {
        if !columns.iter().any(|c| c == col) {
            conn.execute_batch(sql)?;
        }
    }

    Ok(())
}

/// Incremental migrations for columns added after initial schema.
fn run_sidecar_incremental(conn: &mut Connection) -> Result<()> {
    // Check which columns exist and add missing ones
    let columns: Vec<String> = conn
        .prepare("PRAGMA table_info(photos)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();

    let migrations: &[(&str, &str)] = &[
        // User metadata
        ("rating", "ALTER TABLE photos ADD COLUMN rating INTEGER NOT NULL DEFAULT 0"),
        ("is_favorite", "ALTER TABLE photos ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0"),
        ("is_trashed", "ALTER TABLE photos ADD COLUMN is_trashed INTEGER NOT NULL DEFAULT 0"),
        ("trashed_at", "ALTER TABLE photos ADD COLUMN trashed_at TEXT"),
        // Analysis columns
        ("aesthetic_score", "ALTER TABLE photos ADD COLUMN aesthetic_score REAL"),
        ("blur_score", "ALTER TABLE photos ADD COLUMN blur_score REAL"),
        ("scene_type", "ALTER TABLE photos ADD COLUMN scene_type TEXT"),
        ("dominant_colors", "ALTER TABLE photos ADD COLUMN dominant_colors TEXT"),
        ("perceptual_hash", "ALTER TABLE photos ADD COLUMN perceptual_hash TEXT"),
        ("is_screenshot", "ALTER TABLE photos ADD COLUMN is_screenshot INTEGER DEFAULT 0"),
        ("is_nsfw", "ALTER TABLE photos ADD COLUMN is_nsfw INTEGER DEFAULT 0"),
        ("ocr_text", "ALTER TABLE photos ADD COLUMN ocr_text TEXT"),
        ("weather", "ALTER TABLE photos ADD COLUMN weather TEXT"),
        ("time_of_day", "ALTER TABLE photos ADD COLUMN time_of_day TEXT"),
        ("event_id", "ALTER TABLE photos ADD COLUMN event_id TEXT"),
        ("analysis_processed", "ALTER TABLE photos ADD COLUMN analysis_processed INTEGER NOT NULL DEFAULT 0"),
    ];

    for (col, sql) in migrations {
        if !columns.iter().any(|c| c == col) {
            conn.execute_batch(sql)?;
        }
    }

    Ok(())
}

const SIDECAR_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS photos (
    id              TEXT PRIMARY KEY,
    relative_path   TEXT NOT NULL UNIQUE,
    filename        TEXT NOT NULL,
    file_size       INTEGER NOT NULL,
    content_hash    TEXT NOT NULL,
    width           INTEGER,
    height          INTEGER,
    format          TEXT,
    media_type      TEXT NOT NULL DEFAULT 'photo',

    -- EXIF metadata
    taken_at        TEXT,
    camera_make     TEXT,
    camera_model    TEXT,
    lens_model      TEXT,
    focal_length    REAL,
    aperture        REAL,
    shutter_speed   TEXT,
    iso             INTEGER,

    -- GPS
    latitude        REAL,
    longitude       REAL,
    altitude        REAL,
    location_name   TEXT,

    -- AI
    ai_description  TEXT,
    ai_processed_at TEXT,

    -- Processing state
    thumbnail_generated INTEGER NOT NULL DEFAULT 0,
    faces_processed     INTEGER NOT NULL DEFAULT 0,
    ai_processed        INTEGER NOT NULL DEFAULT 0,

    -- User metadata
    rating            INTEGER NOT NULL DEFAULT 0,
    is_favorite       INTEGER NOT NULL DEFAULT 0,

    -- Trash
    is_trashed        INTEGER NOT NULL DEFAULT 0,
    trashed_at        TEXT,

    -- Analysis
    aesthetic_score    REAL,
    blur_score        REAL,
    scene_type        TEXT,
    dominant_colors   TEXT,
    perceptual_hash   TEXT,
    is_screenshot     INTEGER DEFAULT 0,
    is_nsfw           INTEGER DEFAULT 0,
    ocr_text          TEXT,
    weather           TEXT,
    time_of_day       TEXT,
    event_id          TEXT,
    analysis_processed INTEGER NOT NULL DEFAULT 0,

    -- Timestamps
    file_modified_at TEXT NOT NULL,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_photos_taken_at ON photos(taken_at);
CREATE INDEX IF NOT EXISTS idx_photos_content_hash ON photos(content_hash);
CREATE INDEX IF NOT EXISTS idx_photos_location ON photos(latitude, longitude);
CREATE INDEX IF NOT EXISTS idx_photos_processing ON photos(faces_processed, ai_processed, thumbnail_generated);
CREATE INDEX IF NOT EXISTS idx_photos_perceptual_hash ON photos(perceptual_hash);
CREATE INDEX IF NOT EXISTS idx_photos_event_id ON photos(event_id);
CREATE INDEX IF NOT EXISTS idx_photos_analysis ON photos(analysis_processed);
CREATE INDEX IF NOT EXISTS idx_photos_favorite ON photos(is_favorite);
CREATE INDEX IF NOT EXISTS idx_photos_rating ON photos(rating);
CREATE INDEX IF NOT EXISTS idx_photos_trashed ON photos(is_trashed);

CREATE TABLE IF NOT EXISTS events (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    start_time  TEXT,
    end_time    TEXT,
    location_name TEXT,
    photo_count INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS tags (
    id       TEXT PRIMARY KEY,
    name     TEXT NOT NULL,
    category TEXT,
    UNIQUE(name, category)
);

CREATE TABLE IF NOT EXISTS photo_tags (
    photo_id   TEXT NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
    tag_id     TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    confidence REAL,
    source     TEXT NOT NULL,
    PRIMARY KEY (photo_id, tag_id)
);

CREATE TABLE IF NOT EXISTS folders (
    id             TEXT PRIMARY KEY,
    relative_path  TEXT NOT NULL UNIQUE,
    name           TEXT NOT NULL,
    parent_id      TEXT REFERENCES folders(id),
    photo_count    INTEGER NOT NULL DEFAULT 0,
    cover_photo_id TEXT REFERENCES photos(id)
);

CREATE TABLE IF NOT EXISTS faces (
    id                   TEXT PRIMARY KEY,
    photo_id             TEXT NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
    bbox_x               REAL NOT NULL,
    bbox_y               REAL NOT NULL,
    bbox_width           REAL NOT NULL,
    bbox_height          REAL NOT NULL,
    detection_confidence REAL NOT NULL,
    landmarks            BLOB,
    embedding            BLOB,
    identity_id          TEXT,
    identity_confidence  REAL,
    created_at           TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_faces_photo ON faces(photo_id);
CREATE INDEX IF NOT EXISTS idx_faces_identity ON faces(identity_id);

CREATE TABLE IF NOT EXISTS albums (
    id             TEXT PRIMARY KEY,
    name           TEXT NOT NULL,
    cover_photo_id TEXT REFERENCES photos(id),
    album_type     TEXT NOT NULL DEFAULT 'manual',
    rules          TEXT,
    photo_count    INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS album_photos (
    album_id TEXT NOT NULL REFERENCES albums(id) ON DELETE CASCADE,
    photo_id TEXT NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (album_id, photo_id)
);
";

const CENTRAL_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS folder_sources (
    id              TEXT PRIMARY KEY,
    path            TEXT NOT NULL UNIQUE,
    label           TEXT,
    drive_serial    TEXT,
    is_available    INTEGER NOT NULL DEFAULT 1,
    is_locked       INTEGER NOT NULL DEFAULT 0,
    password_hash   TEXT,
    last_scanned_at TEXT,
    created_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS face_identities (
    id                       TEXT PRIMARY KEY,
    name                     TEXT NOT NULL,
    representative_embedding BLOB,
    photo_count              INTEGER NOT NULL DEFAULT 0,
    created_at               TEXT NOT NULL,
    updated_at               TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";
