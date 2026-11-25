//! Yerel SQLite veritabanı. Ayarlar, işlem geçmişi, anlık görüntüler ve
//! karantina kayıtları burada saklanır.

use crate::error::{AppError, AppResult};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub struct Db {
    conn: Mutex<Connection>,
}

const SCHEMA_VERSION: i64 = 2;

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

/// İşlem geçmişi v2: hedef, önceki/yeni durum, onay ve geri alma bilgileri.
const MIGRATION_2: &str = r#"
ALTER TABLE oplog ADD COLUMN target TEXT;
ALTER TABLE oplog ADD COLUMN prev_state TEXT;
ALTER TABLE oplog ADD COLUMN new_state TEXT;
ALTER TABLE oplog ADD COLUMN confirmed INTEGER NOT NULL DEFAULT 0;
ALTER TABLE oplog ADD COLUMN restore_status TEXT NOT NULL DEFAULT 'none';
ALTER TABLE oplog ADD COLUMN restored_at INTEGER;
ALTER TABLE oplog ADD COLUMN restore_op INTEGER;
ALTER TABLE oplog ADD COLUMN error TEXT;
CREATE INDEX IF NOT EXISTS oplog_kind ON oplog(kind);
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
            conn.pragma_update(None, "user_version", 1)?;
        }
        if version < 2 {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(MIGRATION_2)?;
            // Eski kayıtlar için türetilmiş alanları doldur.
            crate::oplog::backfill_derived(&tx)?;
            tx.pragma_update(None, "user_version", 2)?;
            tx.commit()?;
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

    #[test]
    fn upgrades_v1_database_to_v2() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eski.db");
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch(MIGRATION_1).unwrap();
            c.pragma_update(None, "user_version", 1).unwrap();
            c.execute(
                "INSERT INTO oplog(ts, kind, title, outcome, reversible, detail) VALUES(1,'startup','Başlangıç öğesi devre dışı bırakıldı: X','success',1,?1)",
                [r#"{"id":"hkcu_run:X","name":"X","previousEnabled":true,"newEnabled":false}"#],
            )
            .unwrap();
            c.execute(
                "INSERT INTO oplog(ts, kind, title, outcome, reversible, detail) VALUES(2,'files','1 dosya karantinaya taşındı','success',1,'{\"mode\":\"quarantine\"}')",
                [],
            )
            .unwrap();
        }
        let db = Db::open(&path).unwrap();
        let v: i64 = db.with(|c| c.pragma_query_value(None, "user_version", |r| r.get(0))).unwrap();
        assert_eq!(v, 2);
        let rows: Vec<(String, Option<String>, String, i64)> = db
            .with(|c| {
                let mut st = c.prepare("SELECT kind, target, restore_status, confirmed FROM oplog ORDER BY id")?;
                let r = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
                r.collect()
            })
            .unwrap();
        assert_eq!(rows[0], ("startup".into(), Some("X".into()), "available".into(), 1));
        // Karantina kimlikleri kaydedilmemiş eski dosya kaydı geri alınamaz.
        assert_eq!(rows[1].2, "none");
        drop(db);
        // İkinci açılış göçü tekrar çalıştırmamalı.
        let db = Db::open(&path).unwrap();
        assert_eq!(db.with(|c| c.query_row("SELECT COUNT(*) FROM oplog", [], |r| r.get::<_, i64>(0))).unwrap(), 2);
    }
}
