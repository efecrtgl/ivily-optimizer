//! Yerel SQLite veritabanı. Ayarlar, işlem geçmişi, anlık görüntüler ve
//! karantina kayıtları burada saklanır.

use crate::error::{AppError, AppResult};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub struct Db {
    conn: Mutex<Connection>,
}

const SCHEMA_VERSION: i64 = 1;

const MIGRATION_1: &str = r#"
CREATE TABLE IF NOT EXISTS kv (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS oplog (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    ts         INTEGER NOT NULL,
    kind       TEXT NOT NULL,
    title      TEXT NOT NULL,
    outcome    TEXT NOT NULL,
    reversible INTEGER NOT NULL DEFAULT 0,
    detail     TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX IF NOT EXISTS oplog_ts ON oplog(ts);
CREATE TABLE IF NOT EXISTS snapshots (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    ts          INTEGER NOT NULL,
    root        TEXT NOT NULL,
    status      TEXT NOT NULL,
    total_size  INTEGER NOT NULL,
    file_count  INTEGER NOT NULL,
    dir_count   INTEGER NOT NULL,
    error_count INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    volume_total INTEGER,
    volume_free  INTEGER
);
CREATE INDEX IF NOT EXISTS snapshots_root ON snapshots(root, ts);
CREATE TABLE IF NOT EXISTS snapshot_dirs (
    snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    path        TEXT NOT NULL,
    depth       INTEGER NOT NULL,
    size        INTEGER NOT NULL,
    files       INTEGER NOT NULL,
    dirs        INTEGER NOT NULL,
    PRIMARY KEY (snapshot_id, path)
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS quarantine (
    id            TEXT PRIMARY KEY,
    ts            INTEGER NOT NULL,
    original_path TEXT NOT NULL,
    stored_path   TEXT NOT NULL,
    size          INTEGER NOT NULL,
    modified      INTEGER,
    hash          TEXT,
    state         TEXT NOT NULL,
    source        TEXT NOT NULL,
    expires_at    INTEGER
);
CREATE INDEX IF NOT EXISTS quarantine_state ON quarantine(state);
"#;

impl Db {
    pub fn open(path: &Path) -> AppResult<Self> {
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> AppResult<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> AppResult<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(AppError::Storage(
                "veritabanı daha yeni bir sürüm tarafından oluşturulmuş".into(),
            ));
        }
        if version < 1 {
            conn.execute_batch(MIGRATION_1)?;
            conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> AppResult<T> {
        let conn = self.conn.lock();
        Ok(f(&conn)?)
    }

    pub fn with_mut<T>(&self, f: impl FnOnce(&mut Connection) -> rusqlite::Result<T>) -> AppResult<T> {
        let mut conn = self.conn.lock();
        Ok(f(&mut conn)?)
    }

    pub fn kv_get(&self, key: &str) -> AppResult<Option<String>> {
        self.with(|c| {
            c.query_row("SELECT value FROM kv WHERE key = ?1", [key], |r| r.get(0))
                .optional()
        })
    }

    pub fn kv_set(&self, key: &str, value: &str) -> AppResult<()> {
        self.with(|c| {
            c.execute(
                "INSERT INTO kv(key, value) VALUES(?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map(|_| ())
        })
    }
}

pub fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kv_roundtrip() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.kv_get("a").unwrap(), None);
        db.kv_set("a", "1").unwrap();
        db.kv_set("a", "2").unwrap();
        assert_eq!(db.kv_get("a").unwrap().as_deref(), Some("2"));
    }
}
