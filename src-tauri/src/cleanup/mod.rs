//! Temizlik taraması ve yürütmesi.
//!
//! Akış: tarama → önizleme (ön yüz) → onay → yürütme → rapor + geçmiş.
//! Yürütme sırasında her dosya yeniden denetlenir: taramadan sonra
//! değiştirilmiş, korunan bir konumda bulunan veya artık hedef klasörün içinde
//! olmayan dosyalar atlanır.

pub mod rules;

use crate::error::{AppError, AppResult};
use crate::scan::walker::{self, FileEntry, WalkOptions, WalkProgress};
use rules::{Category, Kind, Risk};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupItem {
    pub id: String,
    pub group: String,
    pub name: String,
    pub description: String,
    pub consequence: String,
    pub risk: Risk,
    pub locations: Vec<String>,
    pub size: u64,
    pub files: u64,
    pub errors: u64,
    pub needs_admin: bool,
    /// Atlanma nedeni (ör. "Google Chrome açık").
    pub blocked: Option<String>,
    pub default_selected: bool,
    /// Kalıcı silme mi, yoksa karantinaya taşıma mı yapılacak?
    pub restorable: bool,
    pub permanent_note: String,
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub path: PathBuf,
    pub size: u64,
    pub modified: i64,
    pub root: PathBuf,
}

pub struct CleanupScan {
    pub items: Vec<CleanupItem>,
    pub files: HashMap<String, Vec<Candidate>>,
    pub created_at: i64,
    pub recycle_bin: Option<(u64, u64)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupScanSummary {
    pub items: Vec<CleanupItem>,
    pub created_at: i64,
    pub total_size: u64,
    pub safe_size: u64,
}

impl CleanupScan {
    pub fn summary(&self) -> CleanupScanSummary {
        CleanupScanSummary {
            items: self.items.clone(),
            created_at: self.created_at,
            total_size: self.items.iter().filter(|i| i.blocked.is_none()).map(|i| i.size).sum(),
            safe_size: self.items.iter().filter(|i| i.blocked.is_none() && i.risk == Risk::Safe).map(|i| i.size).sum(),
        }
    }
}

pub fn running_process_names() -> HashSet<String> {
    let mut sys = match sysinfo::System::new() {
        Ok(s) => s,
        Err(_) => return HashSet::new(),
    };
    sys.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, sysinfo::ProcessRefreshKind::nothing());
    sys.processes().values().map(|p| p.name().to_string_lossy().to_lowercase()).collect()
}

fn matches_target(t: &rules::Target, f: &FileEntry, now: i64) -> bool {
    if let Some(prefix) = t.name_prefix {
        if !f.name.to_lowercase().starts_with(prefix) {
            return false;
        }
    }
    if !t.extensions.is_empty() && !t.extensions.contains(&f.extension()) {
        return false;
    }
    if t.min_age_hours > 0 {
        // Değiştirilme zamanı bilinmiyorsa güvenli tarafta kal: seçme.
        if f.modified <= 0 || now - f.modified < t.min_age_hours as i64 * 3600 {
            return false;
        }
    }
    true
}

pub fn scan(
    temp_min_age_hours: u64,
    user_rules: &[crate::settings::UserCleanupRule],
    excluded_categories: &[String],
    exclusions: &crate::settings::Exclusions,
    use_quarantine: bool,
    cancel: &AtomicBool,
    progress: &WalkProgress,
) -> AppResult<CleanupScan> {
    let cats: Vec<Category> = rules::categories(temp_min_age_hours, user_rules)
        .into_iter()
        .filter(|c| !excluded_categories.contains(&c.id))
        .collect();
    scan_categories(cats, exclusions, use_quarantine, cancel, progress, &running_process_names())
}

pub fn scan_categories(
    cats: Vec<Category>,
    exclusions: &crate::settings::Exclusions,
    use_quarantine: bool,
    cancel: &AtomicBool,
    progress: &WalkProgress,
    running: &HashSet<String>,
) -> AppResult<CleanupScan> {
    let now = chrono::Utc::now().timestamp();
    let elevated = crate::platform::is_elevated();
    let mut items = Vec::new();
    let mut files_map = HashMap::new();
    let mut recycle = None;

    for cat in cats {
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::Cancelled);
        }
        let blocked = cat
            .blocking_processes
            .iter()
            .find(|p| running.contains(&p.to_lowercase()))
            .map(|p| format!("{} çalışıyor; temizlemek için önce uygulamayı kapatın", p));

        let permanent_note = if cat.kind == Kind::RecycleBin {
            "Kalıcı olarak silinir, geri alınamaz.".to_string()
        } else if use_quarantine {
            "Karantinaya taşınır; saklama süresi boyunca geri yüklenebilir.".to_string()
        } else {
            "Kalıcı olarak silinir. Bu veriler uygulamalar tarafından yeniden oluşturulur.".to_string()
        };

        let (size, count, errors, cands) = match cat.kind {
            Kind::RecycleBin => {
                let info = crate::platform::recycle_bin_info();
                recycle = info;
                let (s, n) = info.unwrap_or((0, 0));
                (s, n, 0, vec![])
            }
            Kind::Files => {
                let mut cands = Vec::new();
                let mut errors = 0;
                for t in &cat.targets {
                    if !t.path.exists() {
                        continue;
                    }
                    let mut opts = WalkOptions::new(1).with_exclusions(exclusions);
                    if !t.recursive {
                        opts.max_depth = Some(0);
                    }
                    let (found, outcome) = walker::collect_files(&t.path, &opts, cancel, progress, |_, f| {
                        matches_target(t, f, now) && !f.is_cloud_placeholder()
                    });
                    errors += outcome.error_count;
                    cands.extend(found.into_iter().map(|(p, f)| Candidate {
                        path: p,
                        size: f.size,
                        modified: f.modified,
                        root: t.path.clone(),
                    }));
                }
                let size = cands.iter().map(|c| c.size).sum();
                (size, cands.len() as u64, errors, cands)
            }
        };

        if count == 0 && errors == 0 && cat.kind == Kind::Files {
            continue; // Boş kategorileri listelemeyiz.
        }
        let blocked = blocked.or_else(|| {
            if cat.needs_admin && !elevated && count == 0 && errors > 0 {
                Some("Bu konum yönetici hakları gerektiriyor".to_string())
            } else {
                None
            }
        });
        let default_selected = cat.risk == Risk::Safe && blocked.is_none() && count > 0;
        items.push(CleanupItem {
            id: cat.id.clone(),
            group: cat.group.to_string(),
            name: cat.name.clone(),
            description: cat.description.to_string(),
            consequence: cat.consequence.to_string(),
            risk: cat.risk,
            locations: cat
                .targets
                .iter()
                .filter(|t| t.path.exists())
                .map(|t| t.path.to_string_lossy().into_owned())
                .collect(),
            size,
            files: count,
            errors,
            needs_admin: cat.needs_admin && !elevated,
            blocked,
            default_selected,
            restorable: use_quarantine && cat.kind == Kind::Files,
            permanent_note,
        });
        files_map.insert(cat.id, cands);
    }
    Ok(CleanupScan { items, files: files_map, created_at: now, recycle_bin: recycle })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupFile {
    pub path: String,
    pub size: u64,
    pub modified: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupItemFiles {
    pub id: String,
    pub files: Vec<CleanupFile>,
    pub total: u64,
    pub total_bytes: u64,
}

impl CleanupScan {
    /// Bir kategorinin aday dosyaları, büyükten küçüğe.
    pub fn item_files(&self, id: &str, limit: usize) -> Option<CleanupItemFiles> {
        let cands = self.files.get(id)?;
        let mut v: Vec<&Candidate> = cands.iter().collect();
        v.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.path.cmp(&b.path)));
        Some(CleanupItemFiles {
            id: id.to_string(),
            total: cands.len() as u64,
            total_bytes: cands.iter().map(|c| c.size).sum(),
            files: v
                .into_iter()
                .take(limit)
                .map(|c| CleanupFile {
                    path: c.path.to_string_lossy().into_owned(),
                    size: c.size,
                    modified: c.modified,
                })
                .collect(),
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemResult {
    pub id: String,
    pub name: String,
    pub deleted: u64,
    pub quarantined: u64,
    pub bytes: u64,
    pub skipped_changed: u64,
    pub locked: u64,
    pub denied: u64,
    pub missing: u64,
    pub failed: u64,
    pub protected: u64,
    pub sample_errors: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupReport {
    pub items: Vec<ItemResult>,
    pub total_bytes: u64,
    pub total_files: u64,
    pub total_problems: u64,
    pub cancelled: bool,
    pub free_before: Option<u64>,
    pub free_after: Option<u64>,
    pub used_quarantine: bool,
}

pub trait Remover {
    fn remove(&self, c: &Candidate) -> AppResult<bool>;
}

/// Kalıcı silme. `Ok(false)` dönmez; karantina true döner.
pub struct DeleteRemover;
impl Remover for DeleteRemover {
    fn remove(&self, c: &Candidate) -> AppResult<bool> {
        std::fs::remove_file(&c.path).map_err(|e| AppError::io(e, &c.path.to_string_lossy()))?;
        Ok(false)
    }
}

pub struct QuarantineRemover<'a> {
    pub q: crate::quarantine::Quarantine<'a>,
    pub retention_days: u32,
    /// Karantinaya alınan öğelerin kimlikleri (geri alma için).
    pub ids: parking_lot::Mutex<Vec<String>>,
}
impl<'a> QuarantineRemover<'a> {
    pub fn new(q: crate::quarantine::Quarantine<'a>, retention_days: u32) -> Self {
        Self { q, retention_days, ids: Default::default() }
    }
}
impl Remover for QuarantineRemover<'_> {
    fn remove(&self, c: &Candidate) -> AppResult<bool> {
        let item = self.q.add(&c.path, "cleanup", self.retention_days)?;
        self.ids.lock().push(item.id);
        Ok(true)
    }
}

fn file_mtime(p: &Path) -> Option<(i64, u64, bool)> {
    let md = std::fs::symlink_metadata(p).ok()?;
    let m = md.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
    Some((m, md.len(), md.is_file()))
}

/// Seçilen öğeleri temizler.
pub fn execute(
    scan: &CleanupScan,
    selected: &[String],
    remover: &dyn Remover,
    cancel: &AtomicBool,
    on_progress: &dyn Fn(&str, u64),
) -> CleanupReport {
    let mut report = CleanupReport::default();
    for id in selected {
        let Some(item) = scan.items.iter().find(|i| &i.id == id) else { continue };
        if item.blocked.is_some() {
            continue;
        }
        let mut r = ItemResult { id: item.id.clone(), name: item.name.clone(), ..Default::default() };
        if cancel.load(Ordering::Relaxed) {
            report.cancelled = true;
            break;
        }
        if id == "recycle-bin" {
            let before = crate::platform::recycle_bin_info();
            match crate::platform::empty_recycle_bin() {
                Ok(()) => {
                    let after = crate::platform::recycle_bin_info().unwrap_or((0, 0));
                    let (bs, bn) = before.unwrap_or((0, 0));
                    r.bytes = bs.saturating_sub(after.0);
                    r.deleted = bn.saturating_sub(after.1);
                }
                Err(e) => {
                    r.failed += 1;
                    r.sample_errors.push(e.to_string());
                }
            }
            on_progress(&item.name, r.deleted);
            report.items.push(r);
            continue;
        }
        let cands = scan.files.get(id).map(Vec::as_slice).unwrap_or(&[]);
        let mut roots: HashSet<PathBuf> = HashSet::new();
        for c in cands {
            if cancel.load(Ordering::Relaxed) {
                report.cancelled = true;
                break;
            }
            let path_str = c.path.to_string_lossy();
            // Hedef kökün içinde değilse (ör. yol değişmişse) dokunma.
            if !crate::paths::is_within(&path_str, &c.root.to_string_lossy()) {
                r.protected += 1;
                continue;
            }
            // Korunan konumlar: yalnızca açıkça izin verilen Windows\Temp
            // ve WER istisnaları kurallarda tanımlıdır.
            if rules::is_protected_path(&path_str) {
                r.protected += 1;
                continue;
            }
            match file_mtime(&c.path) {
                None => {
                    r.missing += 1;
                    continue;
                }
                Some((m, len, is_file)) => {
                    if !is_file || m != c.modified || len != c.size {
                        r.skipped_changed += 1;
                        continue;
                    }
                }
            }
            match remover.remove(c) {
                Ok(q) => {
                    if q {
                        r.quarantined += 1;
                    } else {
                        r.deleted += 1;
                    }
                    r.bytes += c.size;
                    roots.insert(c.root.clone());
                }
                Err(e) => {
                    match &e {
                        AppError::PermissionDenied(_) => r.denied += 1,
                        AppError::NotFound(_) => r.missing += 1,
                        AppError::System(m) if m.contains("kullanılıyor") => r.locked += 1,
                        AppError::Protected(_) => r.protected += 1,
                        _ => r.failed += 1,
                    }
                    if r.sample_errors.len() < 5 {
                        r.sample_errors.push(e.to_string());
                    }
                }
            }
            on_progress(&item.name, 1);
        }
        for root in roots {
            remove_empty_dirs(&root);
        }
        report.items.push(r);
        if report.cancelled {
            break;
        }
    }
    for r in &report.items {
        report.total_bytes += r.bytes;
        report.total_files += r.deleted + r.quarantined;
        report.total_problems += r.locked + r.denied + r.failed + r.skipped_changed + r.protected;
    }
    report
}

/// Hedef kökün altındaki boş klasörleri siler; kökün kendisine dokunmaz ve
/// bağlantı noktalarını izlemez.
pub fn remove_empty_dirs(root: &Path) {
    fn walk(p: &Path, is_root: bool) -> bool {
        let Ok(rd) = std::fs::read_dir(p) else { return false };
        let mut empty = true;
        for e in rd.flatten() {
            let Ok(md) = e.metadata() else {
                empty = false;
                continue;
            };
            #[cfg(windows)]
            let reparse = {
                use std::os::windows::fs::MetadataExt;
                md.file_attributes() & walker::FILE_ATTRIBUTE_REPARSE_POINT != 0
            };
            #[cfg(not(windows))]
            let reparse = md.file_type().is_symlink();
            if md.is_dir() && !reparse {
                if !walk(&e.path(), false) {
                    empty = false;
                }
            } else {
                empty = false;
            }
        }
        if empty && !is_root {
            return std::fs::remove_dir(p).is_ok();
        }
        empty && !is_root
    }
    walk(root, true);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use rules::Target;

    fn cat(id: &str, root: &Path, risk: Risk, procs: Vec<&'static str>) -> Category {
        Category {
            id: id.into(),
            group: "Test",
            name: id.into(),
            description: "",
            consequence: "",
            risk,
            kind: Kind::Files,
            targets: vec![Target::dir(root.to_path_buf())],
            blocking_processes: procs,
            needs_admin: false,
        }
    }

    fn fixture() -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("c/sub/deep")).unwrap();
        std::fs::write(t.path().join("c/a.tmp"), vec![0u8; 10]).unwrap();
        std::fs::write(t.path().join("c/sub/b.tmp"), vec![0u8; 20]).unwrap();
        std::fs::write(t.path().join("c/sub/deep/c.tmp"), vec![0u8; 30]).unwrap();
        t
    }

    fn scan_one(c: Category, running: &[&str]) -> CleanupScan {
        let running: HashSet<String> = running.iter().map(|s| s.to_string()).collect();
        scan_categories(
            vec![c],
            &Default::default(),
            false,
            &AtomicBool::new(false),
            &WalkProgress::default(),
            &running,
        )
        .unwrap()
    }

    #[test]
    fn scans_and_deletes_and_prunes_empty_dirs() {
        let t = fixture();
        let root = t.path().join("c");
        let s = scan_one(cat("x", &root, Risk::Safe, vec![]), &[]);
        assert_eq!(s.items.len(), 1);
        assert_eq!(s.items[0].size, 60);
        assert_eq!(s.items[0].files, 3);
        assert!(s.items[0].default_selected);
        let rep = execute(&s, &["x".into()], &DeleteRemover, &AtomicBool::new(false), &|_, _| {});
        assert_eq!(rep.total_bytes, 60);
        assert_eq!(rep.total_files, 3);
        assert!(root.exists(), "hedef kök silinmemeli");
        assert!(!root.join("sub").exists(), "boş alt klasörler temizlenmeli");
    }

    #[test]
    fn skips_files_changed_after_scan() {
        let t = fixture();
        let root = t.path().join("c");
        let s = scan_one(cat("x", &root, Risk::Safe, vec![]), &[]);
        std::fs::write(root.join("a.tmp"), vec![1u8; 999]).unwrap();
        let rep = execute(&s, &["x".into()], &DeleteRemover, &AtomicBool::new(false), &|_, _| {});
        assert_eq!(rep.items[0].skipped_changed, 1);
        assert!(root.join("a.tmp").exists());
        assert_eq!(rep.total_files, 2);
    }

    #[test]
    fn blocked_when_process_running() {
        let t = fixture();
        let s = scan_one(cat("b", &t.path().join("c"), Risk::Safe, vec!["chrome.exe"]), &["chrome.exe"]);
        assert!(s.items[0].blocked.is_some());
        assert!(!s.items[0].default_selected);
        let rep = execute(&s, &["b".into()], &DeleteRemover, &AtomicBool::new(false), &|_, _| {});
        assert_eq!(rep.total_files, 0);
        assert!(t.path().join("c/a.tmp").exists());
    }

    #[test]
    fn review_items_not_selected_by_default() {
        let t = fixture();
        let s = scan_one(cat("r", &t.path().join("c"), Risk::Review, vec![]), &[]);
        assert!(!s.items[0].default_selected);
    }

    #[test]
    fn respects_min_age() {
        let t = fixture();
        let mut c = cat("a", &t.path().join("c"), Risk::Safe, vec![]);
        c.targets[0].min_age_hours = 24;
        let s = scan_one(c, &[]);
        assert!(s.items.is_empty(), "yeni dosyalar seçilmemeli");
    }

    #[test]
    fn quarantine_mode_is_restorable() {
        let t = fixture();
        let qdir = tempfile::tempdir().unwrap();
        let db = Db::open_in_memory().unwrap();
        let root = t.path().join("c");
        let s = scan_one(cat("q", &root, Risk::Safe, vec![]), &[]);
        let rem = QuarantineRemover::new(crate::quarantine::Quarantine::new(&db, qdir.path().to_path_buf()), 7);
        let rep = execute(&s, &["q".into()], &rem, &AtomicBool::new(false), &|_, _| {});
        assert_eq!(rep.items[0].quarantined, 3);
        assert_eq!(rem.ids.lock().len(), 3);
        let q = crate::quarantine::Quarantine::new(&db, qdir.path().to_path_buf());
        let items = q.list(false).unwrap();
        assert_eq!(items.len(), 3);
        for i in items {
            q.restore(&i.id, false).unwrap();
        }
        assert!(root.join("sub/deep/c.tmp").exists());
    }

    #[test]
    fn item_files_sorted_by_size() {
        let t = fixture();
        let s = scan_one(cat("x", &t.path().join("c"), Risk::Safe, vec![]), &[]);
        let f = s.item_files("x", 2).unwrap();
        assert_eq!(f.total, 3);
        assert_eq!(f.total_bytes, 60);
        assert_eq!(f.files.len(), 2);
        assert_eq!(f.files[0].size, 30);
        assert_eq!(f.files[1].size, 20);
        assert!(s.item_files("yok", 10).is_none());
    }

    #[test]
    fn cancellation_stops_execution() {
        let t = fixture();
        let s = scan_one(cat("x", &t.path().join("c"), Risk::Safe, vec![]), &[]);
        let rep = execute(&s, &["x".into()], &DeleteRemover, &AtomicBool::new(true), &|_, _| {});
        assert!(rep.cancelled);
        assert_eq!(rep.total_files, 0);
    }
}
