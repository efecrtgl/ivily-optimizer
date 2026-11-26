//! Yerel SQLite veritabanı. Ayarlar, işlem geçmişi, anlık görüntüler ve
//! karantina kayıtları burada saklanır.

use crate::error::{AppError, AppResult};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub struct Db {
    conn: Mutex<Connection>,
}

pub const SCHEMA_VERSION: i64 = 2;

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

/// Veritabanı açılırken oluşabilecek durumlar. Daha yeni bir sürümün
/// oluşturduğu veritabanı bozuk değildir; ayrı ele alınır ve yerinde bırakılır.
#[derive(Debug)]
pub enum OpenError {
    /// Dosya bu sürümün tanımadığı daha yeni bir şemaya sahip.
    NewerSchema(i64),
    Other(AppError),
}

impl From<rusqlite::Error> for OpenError {
    fn from(e: rusqlite::Error) -> Self {
        OpenError::Other(e.into())
    }
}

impl From<AppError> for OpenError {
    fn from(e: AppError) -> Self {
        OpenError::Other(e)
    }
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::NewerSchema(v) => write!(f, "veritabanı daha yeni bir sürüm tarafından oluşturulmuş (şema {v})"),
            OpenError::Other(e) => write!(f, "{e}"),
        }
    }
}

impl Db {
    /// Dosyadaki veritabanını açar ve gerekirse şemayı yükseltir. Var olan bir
    /// veritabanı yükseltilmeden önce `<ad>.v<eski>-<zaman>.bak` adıyla
    /// tutarlı bir yedeği alınır.
    pub fn open(path: &Path) -> Result<Self, OpenError> {
        let conn = Connection::open(path)?;
        Self::init(conn, Some(path))
    }

    pub fn open_in_memory() -> AppResult<Self> {
        Self::init(Connection::open_in_memory()?, None).map_err(|e| match e {
            OpenError::Other(e) => e,
            OpenError::NewerSchema(_) => AppError::Storage("beklenmeyen şema".into()),
        })
    }

    /// Göç öncesi yedeğin yolu.
    pub fn backup_path(path: &Path, from_version: i64) -> std::path::PathBuf {
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "ivily.db".into());
        path.with_file_name(format!("{name}.v{from_version}-{stamp}.bak"))
    }

    fn init(conn: Connection, path: Option<&Path>) -> Result<Self, OpenError> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(OpenError::NewerSchema(version));
        }
        // Var olan bir veritabanı yükseltilecekse önce yedekle. `VACUUM INTO`
        // WAL içeriği dahil tutarlı tek bir dosya üretir. Yedek alınamazsa
        // göç yapılmaz.
        if let Some(path) = path {
            if (1..SCHEMA_VERSION).contains(&version) {
                let bak = Self::backup_path(path, version);
                conn.execute("VACUUM INTO ?1", [bak.to_string_lossy()])
                    .map_err(|e| AppError::Storage(format!("göç öncesi yedek alınamadı: {e}")))?;
                tracing::info!(backup = %bak.display(), from = version, to = SCHEMA_VERSION, "şema yükseltmesi öncesi yedek alındı");
            }
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
        self.with(|c| c.query_row("SELECT value FROM kv WHERE key = ?1", [key], |r| r.get(0)).optional())
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

/// Veritabanını açar; açılamazsa uygulamanın yine de başlayabilmesi için
/// kurtarma yapar ve kullanıcıya gösterilecek bir bildirim döndürür.
///
/// - Daha yeni bir sürümün veritabanı: dosyaya **dokunulmaz**; oturum bellek
///   içi veritabanıyla çalışır (değişiklikler kaydedilmez).
/// - Açılamayan/bozuk veritabanı: dosya, `-wal` ve `-shm` eşleriyle birlikte
///   `<ad>.bozuk-<zaman>` adıyla kenara alınır ve yeni bir veritabanı oluşturulur.
pub fn open_or_recover(path: &Path) -> (Db, Option<String>) {
    let err = match Db::open(path) {
        Ok(db) => return (db, None),
        Err(e) => e,
    };
    let memory = || Db::open_in_memory().expect("bellek içi veritabanı");
    if let OpenError::NewerSchema(v) = err {
        tracing::warn!(schema = v, "veritabanı daha yeni bir sürüme ait; dosyaya dokunulmadı");
        return (
            memory(),
            Some(
                "Verileriniz Ivily Optimizer'ın daha yeni bir sürümü tarafından kaydedilmiş. Veriler korunuyor, ancak bu \
                 oturumdaki değişiklikler kaydedilmeyecek. Lütfen en son sürümü yeniden kurun."
                    .into(),
            ),
        );
    }
    tracing::error!(error = %err, "veritabanı açılamadı, kenara alınıp yeniden oluşturuluyor");
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "ivily.db".into());
    let bak = path.with_file_name(format!("{name}.bozuk-{}", chrono::Local::now().format("%Y%m%d-%H%M%S")));
    // SQLite eş dosyaları `<ad>-wal` ve `<ad>-shm` adlarını kullanır; yedek
    // aynı düzende taşınır ki gerekirse WAL içeriğiyle birlikte açılabilsin.
    for suffix in ["", "-wal", "-shm"] {
        let from = path.with_file_name(format!("{name}{suffix}"));
        if from.exists() {
            let to = bak.with_file_name(format!("{}{suffix}", bak.file_name().unwrap_or_default().to_string_lossy()));
            if let Err(e) = std::fs::rename(&from, &to) {
                tracing::error!(error = %e, file = %from.display(), "bozuk veritabanı dosyası taşınamadı");
            }
        }
    }
    match Db::open(path) {
        Ok(db) => (
            db,
            Some(format!(
                "Veritabanı açılamadı; eski dosya \"{}\" olarak yedeklendi ve yeni bir veritabanı oluşturuldu. Ayarlar \
                 varsayılana döndü ve işlem geçmişi boş başlıyor.",
                bak.display()
            )),
        ),
        Err(e) => {
            tracing::error!(error = %e, "yeni veritabanı da oluşturulamadı; bellek içi çalışılıyor");
            (memory(), Some("Veritabanı oluşturulamadı; bu oturumdaki değişiklikler kaydedilmeyecek.".into()))
        }
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
        drop(db);

        // Göç öncesi tutarlı bir yedek alınmış olmalı ve eski şemayı taşımalı.
        let backups: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with("eski.db.v1-"))
            .collect();
        assert_eq!(backups.len(), 1, "{backups:?}");
        let c = Connection::open(&backups[0]).unwrap();
        let v: i64 = c.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(v, 1);
        assert_eq!(c.query_row("SELECT COUNT(*) FROM oplog", [], |r| r.get::<_, i64>(0)).unwrap(), 2);
    }

    #[test]
    fn fresh_database_is_not_backed_up() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("yeni.db");
        drop(Db::open(&path).unwrap());
        let names: Vec<String> =
            std::fs::read_dir(dir.path()).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into()).collect();
        assert!(names.iter().all(|n| !n.contains(".bak")), "{names:?}");
    }

    #[test]
    fn newer_schema_is_left_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ivily.db");
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch("CREATE TABLE gelecek(x); INSERT INTO gelecek VALUES (42);").unwrap();
            c.pragma_update(None, "user_version", SCHEMA_VERSION + 1).unwrap();
        }
        assert!(matches!(Db::open(&path), Err(OpenError::NewerSchema(v)) if v == SCHEMA_VERSION + 1));
        let (db, notice) = open_or_recover(&path);
        assert!(notice.unwrap().contains("daha yeni"));
        // Oturum bellek içi çalışır; dosya ve verisi yerinde kalır.
        db.kv_set("a", "1").unwrap();
        drop(db);
        let c = Connection::open(&path).unwrap();
        assert_eq!(c.query_row("SELECT x FROM gelecek", [], |r| r.get::<_, i64>(0)).unwrap(), 42);
        let names: Vec<String> =
            std::fs::read_dir(dir.path()).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into()).collect();
        assert!(names.iter().all(|n| !n.contains("bozuk")), "{names:?}");
    }

    #[test]
    fn corrupt_database_is_moved_aside_with_companions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ivily.db");
        std::fs::write(&path, b"bu bir sqlite veritabani degil, yalnizca bozuk bir dosya").unwrap();
        std::fs::write(dir.path().join("ivily.db-wal"), b"eski wal").unwrap();
        std::fs::write(dir.path().join("ivily.db-shm"), b"eski shm").unwrap();
        let (db, notice) = open_or_recover(&path);
        assert!(notice.unwrap().contains("yedeklendi"));
        db.kv_set("a", "1").unwrap();
        assert_eq!(db.kv_get("a").unwrap().as_deref(), Some("1"));
        drop(db);
        let names: Vec<String> =
            std::fs::read_dir(dir.path()).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into()).collect();
        // Bozuk dosya silinmez, kenara alınır. (Geçersiz -wal/-shm eşlerini
        // SQLite başarısız açılışta kendisi atabilir; varsa onlar da taşınır.)
        let moved: Vec<&String> = names.iter().filter(|n| n.starts_with("ivily.db.bozuk-")).collect();
        assert!(moved.iter().any(|n| !n.ends_with("-wal") && !n.ends_with("-shm")), "{names:?}");
        let kept = dir.path().join(moved.iter().find(|n| !n.ends_with("-wal") && !n.ends_with("-shm")).unwrap());
        assert!(std::fs::read(kept).unwrap().starts_with(b"bu bir sqlite"));
        // Eski WAL yeni veritabanına karışmamış olmalı.
        assert_ne!(std::fs::read(dir.path().join("ivily.db-wal")).ok().as_deref(), Some(&b"eski wal"[..]));
    }
}
