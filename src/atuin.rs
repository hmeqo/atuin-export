use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;

pub struct HistoryEntry {
    pub timestamp_ns: i64,
    pub command: String,
}

pub fn read_history<P: AsRef<Path>>(db: P) -> Result<Vec<HistoryEntry>> {
    let conn = Connection::open(db).context("failed to open atuin history.db")?;
    let mut stmt = conn
        .prepare("SELECT timestamp, command FROM history ORDER BY timestamp ASC")
        .context("failed to prepare query")?;

    let rows = stmt
        .query_map([], |row| {
            let (ts, cmd): (i64, String) = (row.get(0)?, row.get(1)?);
            Ok(HistoryEntry {
                timestamp_ns: ts,
                command: cmd,
            })
        })
        .context("failed to query history")?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row.context("failed to read row")?);
    }

    Ok(entries)
}
