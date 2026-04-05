mod schema;
mod photos;
mod folders;
mod tags;
mod faces;
mod face_identities;
mod search;
mod albums;
mod dedupe;
mod events;

pub use photos::{PhotosDb, photo_from_row, SELECT_COLUMNS, hamming_distance_hex};
pub use folders::FoldersDb;
pub use tags::TagsDb;
pub use faces::FacesDb;
pub use face_identities::FaceIdentitiesDb;
pub use search::SearchDb;
pub use albums::AlbumsDb;
pub use dedupe::{DedupeDb, DuplicateGroup};
pub use events::{EventsDb, Event};

use crate::Result;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;

/// Apply the SQLCipher encryption key to a connection.
/// If `key` is `None`, uses an empty key (unencrypted-compatible mode).
fn apply_cipher_key(conn: &Connection, key: Option<&str>) -> Result<()> {
    if let Some(k) = key {
        // Use a parameterised PRAGMA so special chars in the key are safe.
        conn.pragma_update(None, "key", k)?;
    }
    Ok(())
}

pub struct DbPool {
    writer: Arc<Mutex<Connection>>,
    readers: Vec<Arc<Mutex<Connection>>>,
}

impl DbPool {
    /// Open a sidecar database at `root_path/.mim/mim.db`.
    /// If `encryption_key` is `Some`, the database will be encrypted with SQLCipher.
    pub fn open_sidecar(root_path: &Path) -> Result<Self> {
        Self::open_sidecar_with_key(root_path, None)
    }

    pub fn open_sidecar_with_key(root_path: &Path, encryption_key: Option<&str>) -> Result<Self> {
        let db_dir = root_path.join(".mim");
        std::fs::create_dir_all(&db_dir)?;
        let db_path = db_dir.join("mim.db");

        let mut writer = Connection::open(&db_path)?;
        apply_cipher_key(&writer, encryption_key)?;
        schema::run_sidecar_migrations(&mut writer)?;

        let mut readers = Vec::new();
        for _ in 0..4 {
            let conn = Connection::open(&db_path)?;
            apply_cipher_key(&conn, encryption_key)?;
            conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA query_only = ON;")?;
            readers.push(Arc::new(Mutex::new(conn)));
        }

        Ok(DbPool {
            writer: Arc::new(Mutex::new(writer)),
            readers,
        })
    }

    /// Open the central database. If `encryption_key` is `Some`, the database is encrypted.
    pub fn open_central(db_path: &Path) -> Result<Self> {
        Self::open_central_with_key(db_path, None)
    }

    pub fn open_central_with_key(db_path: &Path, encryption_key: Option<&str>) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut writer = Connection::open(db_path)?;
        apply_cipher_key(&writer, encryption_key)?;
        schema::run_central_migrations(&mut writer)?;

        let mut readers = Vec::new();
        for _ in 0..2 {
            let conn = Connection::open(db_path)?;
            apply_cipher_key(&conn, encryption_key)?;
            conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA query_only = ON;")?;
            readers.push(Arc::new(Mutex::new(conn)));
        }

        Ok(DbPool {
            writer: Arc::new(Mutex::new(writer)),
            readers,
        })
    }

    pub fn writer(&self) -> &Arc<Mutex<Connection>> {
        &self.writer
    }

    pub fn reader(&self) -> &Arc<Mutex<Connection>> {
        &self.readers[0]
    }
}
