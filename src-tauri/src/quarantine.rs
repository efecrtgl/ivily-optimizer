//! Karantina: dosyaları silmek yerine geri alınabilir şekilde uygulamanın
//! veri klasörüne taşır.
//!
//! - Aynı birimde taşıma `rename` ile anında yapılır.
//! - Farklı birimde önce kopyalanır, boyut doğrulanır, sonra özgün silinir;
//!   doğrulama başarısızsa özgün dosyaya dokunulmaz.
//! - Geri yüklemede hedefte dosya varsa asla üzerine yazılmaz.

use crate::db::{now_ts, Db};
use crate::error::{AppError, AppResult};
use rusqlite::params;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuarantineItem {
    pub id: String,
    pub ts: i64,
    pub original_path: String,
    pub stored_path: String,
    pub size: u64,
    pub modified: Option<i64>,
    pub state: String,
    pub source: String,
    pub expires_at: Option<i64>,
    pub original_exists: bool,
}

/// Geri yükleme sonucu: dosyanın yazıldığı yol ve hedefte doğrulanıp
/// doğrulanamadığı (dosya var ve boyutu kayıtlı boyutla aynı).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOutcome {
    pub path: String,
    pub verified: bool,
}

pub struct Quarantine<'a> {
    pub db: &'a Db,
    pub root: PathBuf,
}

fn new_id() -> String {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{:x}-{:x}-{n:x}", chrono::Utc::now().timestamp_millis(), std::process::id())
}

/// Aynı birimde değilse kopyala + doğrula + sil.
pub fn move_file(from: &Path, to: &Path) -> AppResult<()> {
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(17) || e.kind() == std::io::ErrorKind::CrossesDevices => {
            let src_len = std::fs::metadata(from).map_err(|e| AppError::io(e, &from.to_string_lossy()))?.len();
            let copied = std::fs::copy(from, to).map_err(|e| AppError::io(e, &to.to_string_lossy()))?;
            if copied != src_len {
                let _ = std::fs::remove_file(to);
                return Err(AppError::System("kopya doğrulanamadı; özgün dosyaya dokunulmadı".into()));
            }
            if let Err(e) = std::fs::remove_file(from) {
                let _ = std::fs::remove_file(to);
                return Err(AppError::io(e, &from.to_string_lossy()));
            }
            Ok(())
        }
        Err(e) => Err(AppError::io(e, &from.to_string_lossy())),
    }
}

fn row(r: &rusqlite::Row) -> rusqlite::Result<QuarantineItem> {
    let original: String = r.get(2)?;
    Ok(QuarantineItem {
        id: r.get(0)?,
        ts: r.get(1)?,
        original_exists: Path::new(&original).exists(),
        original_path: original,
        stored_path: r.get(3)?,
        size: r.get::<_, i64>(4)? as u64,
        modified: r.get(5)?,
        state: r.get(6)?,
        source: r.get(7)?,
        expires_at: r.get(8)?,
    })
}

const COLS: &str = "id, ts, original_path, stored_path, size, modified, state, source, expires_at";

impl<'a> Quarantine<'a> {
    pub fn new(db: &'a Db, root: PathBuf) -> Self {
        Self { db, root }
    }

    /// Bir dosyayı karantinaya alır.
    pub fn add(&self, path: &Path, source: &str, retention_days: u32) -> AppResult<QuarantineItem> {
        let md = std::fs::symlink_metadata(path).map_err(|e| AppError::io(e, &path.to_string_lossy()))?;
        if !md.is_file() {
            return Err(AppError::InvalidInput(format!("{} bir dosya değil", path.display())));
        }
        let path_str = path.to_string_lossy().into_owned();
        if crate::cleanup::rules::is_protected_path(&path_str) {
            return Err(AppError::Protected(path_str));
        }
        let id = new_id();
        let dir = self.root.join(&id);
        std::fs::create_dir_all(&dir).map_err(|e| AppError::io(e, "karantina klasörü"))?;
        let name = path.file_name().map(|n| n.to_os_string()).unwrap_or_else(|| "dosya".into());
        let stored = dir.join(name);
        if let Err(e) = move_file(path, &stored) {
            let _ = std::fs::remove_dir(&dir);
            return Err(e);
        }
        let modified = md
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);
        let ts = now_ts();
        let item = QuarantineItem {
            id: id.clone(),
            ts,
            original_path: path_str,
            stored_path: stored.to_string_lossy().into_owned(),
            size: md.len(),
            modified,
            state: "quarantined".into(),
            source: source.into(),
            expires_at: Some(ts + retention_days as i64 * 86_400),
            original_exists: false,
        };
        let res = self.db.with(|c| {
            c.execute(
                &format!("INSERT INTO quarantine({COLS}) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)"),
                params![
                    item.id,
                    item.ts,
                    item.original_path,
                    item.stored_path,
                    item.size as i64,
                    item.modified,
                    item.state,
                    item.source,
                    item.expires_at
                ],
            )
        });
        if let Err(e) = res {
            // Kayıt yazılamadıysa dosyayı geri koy: kayıtsız karantina olmamalı.
            let _ = move_file(&stored, path);
            let _ = std::fs::remove_dir(&dir);
            return Err(e);
        }
        Ok(item)
    }

    pub fn list(&self, include_history: bool) -> AppResult<Vec<QuarantineItem>> {
        self.db.with(|c| {
            let sql = if include_history {
                format!("SELECT {COLS} FROM quarantine ORDER BY ts DESC LIMIT 5000")
            } else {
                format!("SELECT {COLS} FROM quarantine WHERE state = 'quarantined' ORDER BY ts DESC")
            };
            let mut st = c.prepare(&sql)?;
            let rows = st.query_map([], row)?;
            rows.collect()
        })
    }

    pub fn get(&self, id: &str) -> AppResult<QuarantineItem> {
        use rusqlite::OptionalExtension;
        self.db
            .with(|c| c.query_row(&format!("SELECT {COLS} FROM quarantine WHERE id = ?1"), [id], row).optional())?
            .ok_or_else(|| AppError::NotFound("karantina öğesi".into()))
    }

    fn set_state(&self, id: &str, state: &str) -> AppResult<()> {
        self.db.with(|c| c.execute("UPDATE quarantine SET state = ?2 WHERE id = ?1", params![id, state]).map(|_| ()))
    }

    /// Geri yükler. `rename_on_conflict` false ise ve hedef doluysa
    /// `Conflict` döner; true ise yanına "(geri yüklendi)" adıyla yazar.
    pub fn restore(&self, id: &str, rename_on_conflict: bool) -> AppResult<RestoreOutcome> {
        let item = self.get(id)?;
        if item.state != "quarantined" {
            return Err(AppError::InvalidInput("bu öğe artık karantinada değil".into()));
        }
        let stored = PathBuf::from(&item.stored_path);
        if !stored.exists() {
            self.set_state(id, "missing")?;
            return Err(AppError::NotFound("karantinadaki dosya bulunamadı".into()));
        }
        let original = PathBuf::from(&item.original_path);
        let target = if original.exists() {
            if !rename_on_conflict {
                return Err(AppError::Conflict(format!(
                    "{} konumunda zaten bir dosya var. Üzerine yazılmayacak.",
                    item.original_path
                )));
            }
            alternative_name(&original)
        } else {
            original
        };
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::io(e, &parent.to_string_lossy()))?;
        }
        move_file(&stored, &target)?;
        if let Some(dir) = stored.parent() {
            let _ = std::fs::remove_dir(dir);
        }
        self.set_state(id, "restored")?;
        let verified = std::fs::symlink_metadata(&target).map(|m| m.is_file() && m.len() == item.size).unwrap_or(false);
        if !verified {
            tracing::warn!(id, target = %target.display(), "geri yüklenen dosya doğrulanamadı");
        }
        Ok(RestoreOutcome { path: target.to_string_lossy().into_owned(), verified })
    }

    /// Karantinadaki dosyayı kalıcı olarak siler.
    pub fn purge(&self, id: &str) -> AppResult<u64> {
        let item = self.get(id)?;
        if item.state != "quarantined" {
            return Ok(0);
        }
        let stored = PathBuf::from(&item.stored_path);
        // Yalnızca karantina klasörünün içindeki dosyalar silinebilir.
        if !crate::paths::is_within(&item.stored_path, &self.root.to_string_lossy()) {
            return Err(AppError::Protected("karantina dışındaki bir yol".into()));
        }
        match std::fs::remove_file(&stored) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(AppError::io(e, &item.stored_path)),
        }
        if let Some(dir) = stored.parent() {
            let _ = std::fs::remove_dir(dir);
        }
        self.set_state(id, "deleted")?;
        Ok(item.size)
    }

    /// Süresi dolan öğeleri kalıcı olarak siler.
    pub fn purge_expired(&self) -> AppResult<(usize, u64)> {
        let now = now_ts();
        let expired: Vec<String> = self
            .list(false)?
            .into_iter()
            .filter(|i| i.expires_at.map(|e| e < now).unwrap_or(false))
            .map(|i| i.id)
            .collect();
        let mut bytes = 0;
        let mut n = 0;
        for id in expired {
            if let Ok(b) = self.purge(&id) {
                bytes += b;
                n += 1;
            }
        }
        Ok((n, bytes))
    }

    pub fn total_size(&self) -> AppResult<(u64, u64)> {
        self.db.with(|c| {
            c.query_row(
                "SELECT COUNT(*), COALESCE(SUM(size),0) FROM quarantine WHERE state = 'quarantined'",
                [],
                |r| Ok((r.get::<_, i64>(0)? as u64, r.get::<_, i64>(1)? as u64)),
            )
        })
    }
}

/// "rapor.pdf" → "rapor (geri yüklendi).pdf", doluysa "(geri yüklendi 2)".
pub fn alternative_name(p: &Path) -> PathBuf {
    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let parent = p.parent().map(Path::to_path_buf).unwrap_or_default();
    for n in 1..10_000 {
        let suffix = if n == 1 { " (geri yüklendi)".to_string() } else { format!(" (geri yüklendi {n})") };
        let cand = parent.join(format!("{stem}{suffix}{ext}"));
        if !cand.exists() {
            return cand;
        }
    }
    parent.join(format!("{stem} ({}){ext}", new_id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Db, tempfile::TempDir, tempfile::TempDir) {
        (Db::open_in_memory().unwrap(), tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap())
    }

    #[test]
    fn quarantine_and_restore() {
        let (db, data, qdir) = setup();
        let q = Quarantine::new(&db, qdir.path().to_path_buf());
        let f = data.path().join("belge.txt");
        std::fs::write(&f, b"merhaba").unwrap();
        let item = q.add(&f, "test", 30).unwrap();
        assert!(!f.exists());
        assert!(Path::new(&item.stored_path).exists());
        assert_eq!(q.total_size().unwrap(), (1, 7));
        let back = q.restore(&item.id, false).unwrap();
        assert_eq!(PathBuf::from(&back.path), f);
        assert!(back.verified);
        assert_eq!(std::fs::read(&f).unwrap(), b"merhaba");
        assert_eq!(q.list(false).unwrap().len(), 0);
        assert_eq!(q.get(&item.id).unwrap().state, "restored");
    }

    #[test]
    fn restore_conflict_never_overwrites() {
        let (db, data, qdir) = setup();
        let q = Quarantine::new(&db, qdir.path().to_path_buf());
        let f = data.path().join("a.txt");
        std::fs::write(&f, b"eski").unwrap();
        let item = q.add(&f, "test", 30).unwrap();
        std::fs::write(&f, b"yeni").unwrap();
        assert!(matches!(q.restore(&item.id, false), Err(AppError::Conflict(_))));
        assert_eq!(std::fs::read(&f).unwrap(), b"yeni");
        let alt = q.restore(&item.id, true).unwrap().path;
        assert!(alt.ends_with("a (geri yüklendi).txt"));
        assert_eq!(std::fs::read(&alt).unwrap(), b"eski");
        assert_eq!(std::fs::read(&f).unwrap(), b"yeni");
    }

    #[test]
    fn restore_recreates_missing_parent() {
        let (db, data, qdir) = setup();
        let q = Quarantine::new(&db, qdir.path().to_path_buf());
        let sub = data.path().join("alt");
        std::fs::create_dir_all(&sub).unwrap();
        let f = sub.join("x.bin");
        std::fs::write(&f, b"1").unwrap();
        let item = q.add(&f, "test", 30).unwrap();
        std::fs::remove_dir(&sub).unwrap();
        q.restore(&item.id, false).unwrap();
        assert!(f.exists());
    }

    #[test]
    fn purge_and_expiry() {
        let (db, data, qdir) = setup();
        let q = Quarantine::new(&db, qdir.path().to_path_buf());
        let f = data.path().join("b.txt");
        std::fs::write(&f, b"12").unwrap();
        let item = q.add(&f, "test", 30).unwrap();
        db.with(|c| c.execute("UPDATE quarantine SET expires_at = 1", [])).unwrap();
        let (n, bytes) = q.purge_expired().unwrap();
        assert_eq!((n, bytes), (1, 2));
        assert!(!Path::new(&item.stored_path).exists());
        assert!(!f.exists());
        assert_eq!(q.get(&item.id).unwrap().state, "deleted");
    }

    #[test]
    fn restore_reports_unverified_when_size_differs() {
        let (db, data, qdir) = setup();
        let q = Quarantine::new(&db, qdir.path().to_path_buf());
        let f = data.path().join("c.txt");
        std::fs::write(&f, b"12345").unwrap();
        let item = q.add(&f, "test", 30).unwrap();
        // Karantinadaki kopya bozulmuş gibi: kayıtlı boyut artık tutmuyor.
        std::fs::write(&item.stored_path, b"12").unwrap();
        let out = q.restore(&item.id, false).unwrap();
        assert!(!out.verified);
        assert!(f.exists());
    }

    #[test]
    fn refuses_directories() {
        let (db, data, qdir) = setup();
        let q = Quarantine::new(&db, qdir.path().to_path_buf());
        assert!(q.add(data.path(), "test", 30).is_err());
    }
}
