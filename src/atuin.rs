use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;

pub struct HistoryEntry {
    pub timestamp_ns: i64,
    pub command: String,
    pub cwd: String,
    /// The atuin session the command ran in (one per shell startup).
    pub session: String,
}

impl HistoryEntry {
    /// Seconds since epoch, as shell history files store timestamps.
    pub fn when(&self) -> i64 {
        self.timestamp_ns / 1_000_000_000
    }
}

pub fn read_history<P: AsRef<Path>>(db: P) -> Result<Vec<HistoryEntry>> {
    let conn = Connection::open(db).context("failed to open atuin history.db")?;
    let mut stmt = conn
        .prepare(
            "SELECT timestamp, command, cwd, session FROM history WHERE deleted_at IS NULL ORDER BY timestamp ASC",
        )
        .context("failed to prepare query")?;

    let rows = stmt
        .query_map([], |row| {
            let (ts, cmd, cwd, session): (i64, String, String, String) =
                (row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?);
            Ok(HistoryEntry {
                timestamp_ns: ts,
                command: cmd,
                cwd,
                session,
            })
        })
        .context("failed to query history")?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row.context("failed to read row")?);
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn read_history_excludes_deleted_entries() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("history.db");
        let conn = Connection::open(&db).unwrap();
        conn.execute(
            "CREATE TABLE history (
                id text primary key, timestamp integer not null, command text not null,
                cwd text not null, duration integer not null, exit integer not null,
                session text not null, hostname text not null, deleted_at integer
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO history VALUES ('a', 100, 'keep me', '/x', 0, 0, 's', 'h', NULL)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO history VALUES ('b', 200, 'deleted', '/x', 0, 0, 's', 'h', 300)",
            [],
        )
        .unwrap();
        drop(conn);

        let entries = read_history(&db).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command, "keep me");
    }
}
