use crate::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub name: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub location_name: Option<String>,
    pub photo_count: u32,
}

pub struct EventsDb;

impl EventsDb {
    pub fn upsert(conn: &Arc<Mutex<Connection>>, event: &Event) -> Result<()> {
        let conn = conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO events (id, name, start_time, end_time, location_name, photo_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.id,
                event.name,
                event.start_time,
                event.end_time,
                event.location_name,
                event.photo_count,
            ],
        )?;
        Ok(())
    }

    pub fn list(conn: &Arc<Mutex<Connection>>) -> Result<Vec<Event>> {
        let conn = conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, start_time, end_time, location_name, photo_count
             FROM events ORDER BY start_time DESC",
        )?;
        let events = stmt
            .query_map([], |row| {
                Ok(Event {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    start_time: row.get(2)?,
                    end_time: row.get(3)?,
                    location_name: row.get(4)?,
                    photo_count: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(events)
    }

    pub fn get_by_id(conn: &Arc<Mutex<Connection>>, id: &str) -> Result<Option<Event>> {
        let conn = conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, start_time, end_time, location_name, photo_count
             FROM events WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Event {
                id: row.get(0)?,
                name: row.get(1)?,
                start_time: row.get(2)?,
                end_time: row.get(3)?,
                location_name: row.get(4)?,
                photo_count: row.get(5)?,
            })
        })?;
        match rows.next() {
            Some(Ok(e)) => Ok(Some(e)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }
}
