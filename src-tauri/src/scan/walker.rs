//! Genel amaçlı, paralel ve iptal edilebilir dizin gezgini.
//!
//! Güvenlik ve sağlamlık kuralları:
//! - Yeniden ayrıştırma noktaları (junction, sembolik bağlantı, bağlama
//!   noktası) **izlenmez**; bu sayede sonsuz döngüler ve hedef dışına taşan
//!   gezinmeler engellenir. Bu tür klasörler sayılır ve raporlanır.
//! - Erişilemeyen klasörler taramayı durdurmaz; hata olarak kaydedilir.
//! - Hariç tutulan yollar ve uzantılar atlanır.
//! - Uzun yollar Rust standart kütüphanesi tarafından (`\\?\` önekiyle)
//!   otomatik olarak desteklenir.
//! - İptal bayrağı her klasörde denetlenir.

use crate::paths;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

pub const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
pub const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
pub const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
pub const FILE_ATTRIBUTE_OFFLINE: u32 = 0x1000;
pub const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x40000;
pub const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x400000;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub size: u64,
    /// Unix saniyesi; bilinmiyorsa 0.
    pub modified: i64,
    pub created: i64,
    pub attrs: u32,
}

impl FileEntry {
    pub fn extension(&self) -> String {
        extension_of(&self.name)
    }
    /// OneDrive gibi bulut yer tutucuları: boyut yerel diskte olmayabilir.
    pub fn is_cloud_placeholder(&self) -> bool {
        self.attrs & (FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_RECALL_ON_OPEN | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
            != 0
    }
}

pub fn extension_of(name: &str) -> String {
    match name.rfind('.') {
        Some(i) if i > 0 && i + 1 < name.len() => name[i + 1..].to_lowercase(),
        _ => String::new(),
    }
}

/// Gezgin tarafından her klasör için çağrılan ziyaretçiye verilen bilgi.
pub struct DirVisit<'a> {
    pub path: &'a Path,
    pub name: &'a str,
    pub parent: u32,
    pub depth: u16,
    pub modified: i64,
    pub files: Vec<FileEntry>,
    /// Bu klasörde bulunan ve izlenmeyen bağlantı/junction sayısı.
    pub skipped_links: u32,
    /// Bu klasörün içeriği okunamadıysa true.
    pub unreadable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanErrorItem {
    pub path: String,
    pub reason: String,
}

#[derive(Default)]
pub struct WalkProgress {
    pub files: AtomicU64,
    pub dirs: AtomicU64,
    pub bytes: AtomicU64,
    pub errors: AtomicU64,
    pub current: Mutex<String>,
}

pub struct WalkOptions {
    pub threads: usize,
    /// Normalleştirilmiş hariç tutma yolları.
    pub excluded_paths: Vec<String>,
    pub excluded_exts: HashSet<String>,
    pub max_depth: Option<u16>,
}

impl WalkOptions {
    pub fn new(threads: usize) -> Self {
        Self { threads, excluded_paths: vec![], excluded_exts: HashSet::new(), max_depth: None }
    }

    pub fn with_exclusions(mut self, ex: &crate::settings::Exclusions) -> Self {
        self.excluded_paths = ex.paths.iter().map(|p| paths::normalize(p)).collect();
        self.excluded_exts = ex.extensions.iter().cloned().collect();
        self
    }

    fn is_path_excluded(&self, p: &Path) -> bool {
        if self.excluded_paths.is_empty() {
            return false;
        }
        let s = p.to_string_lossy();
        self.excluded_paths.iter().any(|ex| paths::is_within(&s, ex))
    }
}

pub struct WalkOutcome {
    pub cancelled: bool,
    pub errors: Vec<ScanErrorItem>,
    pub error_count: u64,
}

const MAX_ERRORS_KEPT: usize = 500;

fn ts(t: std::io::Result<std::time::SystemTime>) -> i64 {
    t.ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[cfg(windows)]
fn attrs_of(md: &std::fs::Metadata) -> u32 {
    use std::os::windows::fs::MetadataExt;
    md.file_attributes()
}
#[cfg(not(windows))]
fn attrs_of(md: &std::fs::Metadata) -> u32 {
    if md.file_type().is_symlink() {
        FILE_ATTRIBUTE_REPARSE_POINT
    } else {
        0
    }
}

fn describe_io(e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::PermissionDenied => "Erişim reddedildi".into(),
        std::io::ErrorKind::NotFound => "Bulunamadı (tarama sırasında silinmiş olabilir)".into(),
        _ => match e.raw_os_error() {
            Some(32) | Some(33) => "Başka bir uygulama tarafından kilitlenmiş".into(),
            Some(1920) => "Sistem tarafından erişilemiyor".into(),
            _ => format!("Okuma hatası ({e})"),
        },
    }
}

struct Shared<'v, V> {
    opts: &'v WalkOptions,
    cancel: &'v AtomicBool,
    progress: &'v WalkProgress,
    visitor: &'v V,
    errors: Mutex<Vec<ScanErrorItem>>,
}

impl<V> Shared<'_, V> {
    fn push_error(&self, path: &Path, reason: String) {
        self.progress.errors.fetch_add(1, Ordering::Relaxed);
        let mut errs = self.errors.lock();
        if errs.len() < MAX_ERRORS_KEPT {
            errs.push(ScanErrorItem { path: path.to_string_lossy().into_owned(), reason });
        }
    }
}

/// `root` altını gezer. Ziyaretçi her klasör için bir kez çağrılır ve klasöre
/// verdiği kimliği döndürür; alt klasörler bu kimliği `parent` olarak alır.
pub fn walk<V>(
    root: &Path,
    opts: &WalkOptions,
    cancel: &AtomicBool,
    progress: &WalkProgress,
    visitor: &V,
) -> WalkOutcome
where
    V: Fn(DirVisit) -> u32 + Sync,
{
    let shared = Shared { opts, cancel, progress, visitor, errors: Mutex::new(Vec::new()) };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(opts.threads.max(1))
        .thread_name(|i| format!("ivily-scan-{i}"))
        .build();

    let root_md = std::fs::metadata(root);
    let root_modified = root_md.as_ref().map(|m| ts(m.modified())).unwrap_or(0);
    let root_name = root.to_string_lossy().into_owned();

    match pool {
        Ok(pool) => pool.scope(|s| {
            visit_dir(s, &shared, root.to_path_buf(), root_name, u32::MAX, 0, root_modified);
        }),
        Err(_) => {
            // İş parçacığı havuzu kurulamazsa sıralı olarak devam et.
            rayon::scope(|s| visit_dir(s, &shared, root.to_path_buf(), root_name, u32::MAX, 0, root_modified));
        }
    }

    let errors = shared.errors.into_inner();
    WalkOutcome {
        cancelled: cancel.load(Ordering::Relaxed),
        error_count: progress.errors.load(Ordering::Relaxed),
        errors,
    }
}

fn visit_dir<'s, V>(
    scope: &rayon::Scope<'s>,
    sh: &'s Shared<'s, V>,
    path: PathBuf,
    name: String,
    parent: u32,
    depth: u16,
    modified: i64,
) where
    V: Fn(DirVisit) -> u32 + Sync,
{
    if sh.cancel.load(Ordering::Relaxed) {
        return;
    }
    sh.progress.dirs.fetch_add(1, Ordering::Relaxed);
    if let Some(mut cur) = sh.progress.current.try_lock() {
        *cur = path.to_string_lossy().into_owned();
    }

    let mut files = Vec::new();
    let mut subdirs: Vec<(PathBuf, String, i64)> = Vec::new();
    let mut skipped_links = 0u32;
    let mut unreadable = false;

    match std::fs::read_dir(&path) {
        Ok(rd) => {
            for entry in rd {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        sh.push_error(&path, describe_io(&e));
                        continue;
                    }
                };
                // Windows'ta DirEntry::metadata bağlantıları izlemez ve
                // FindNextFile verisinden geldiği için ucuzdur.
                let md = match entry.metadata() {
                    Ok(m) => m,
                    Err(e) => {
                        sh.push_error(&entry.path(), describe_io(&e));
                        continue;
                    }
                };
                let attrs = attrs_of(&md);
                let fname = entry.file_name().to_string_lossy().into_owned();
                if md.is_dir() {
                    if attrs & FILE_ATTRIBUTE_REPARSE_POINT != 0 || md.file_type().is_symlink() {
                        skipped_links += 1;
                        continue;
                    }
                    let p = entry.path();
                    if sh.opts.is_path_excluded(&p) {
                        continue;
                    }
                    subdirs.push((p, fname, ts(md.modified())));
                } else {
                    if md.file_type().is_symlink() {
                        skipped_links += 1;
                        continue;
                    }
                    if !sh.opts.excluded_exts.is_empty() && sh.opts.excluded_exts.contains(&extension_of(&fname)) {
                        continue;
                    }
                    if !sh.opts.excluded_paths.is_empty() && sh.opts.is_path_excluded(&entry.path()) {
                        continue;
                    }
                    let size = md.len();
                    sh.progress.files.fetch_add(1, Ordering::Relaxed);
                    sh.progress.bytes.fetch_add(size, Ordering::Relaxed);
                    files.push(FileEntry {
                        name: fname,
                        size,
                        modified: ts(md.modified()),
                        created: ts(md.created()),
                        attrs,
                    });
                }
            }
        }
        Err(e) => {
            unreadable = true;
            sh.push_error(&path, describe_io(&e));
        }
    }

    let id =
        (sh.visitor)(DirVisit { path: &path, name: &name, parent, depth, modified, files, skipped_links, unreadable });

    if let Some(max) = sh.opts.max_depth {
        if depth >= max {
            return;
        }
    }
    for (p, n, m) in subdirs {
        scope.spawn(move |s| visit_dir(s, sh, p, n, id, depth.saturating_add(1), m));
    }
}

/// Basit yardımcı: bir klasör altındaki tüm dosyaları tam yollarıyla toplar.
pub fn collect_files(
    root: &Path,
    opts: &WalkOptions,
    cancel: &AtomicBool,
    progress: &WalkProgress,
    mut filter: impl FnMut(&Path, &FileEntry) -> bool + Send,
) -> (Vec<(PathBuf, FileEntry)>, WalkOutcome) {
    let out: Mutex<Vec<(PathBuf, FileEntry)>> = Mutex::new(Vec::new());
    let filter_lock = Mutex::new(&mut filter);
    let next = AtomicU64::new(0);
    let outcome = walk(root, opts, cancel, progress, &|v: DirVisit| {
        let mut keep = Vec::new();
        {
            let mut f = filter_lock.lock();
            for fe in v.files {
                if (*f)(v.path, &fe) {
                    keep.push((v.path.join(&fe.name), fe));
                }
            }
        }
        if !keep.is_empty() {
            out.lock().extend(keep);
        }
        next.fetch_add(1, Ordering::Relaxed) as u32
    });
    (out.into_inner(), outcome)
}

pub type Cancel = Arc<AtomicBool>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tree() -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        fs::create_dir_all(r.join("a/b/c")).unwrap();
        fs::create_dir_all(r.join("skip")).unwrap();
        fs::write(r.join("root.txt"), b"12345").unwrap();
        fs::write(r.join("a/x.log"), vec![0u8; 100]).unwrap();
        fs::write(r.join("a/b/y.bin"), vec![0u8; 1000]).unwrap();
        fs::write(r.join("a/b/c/z.txt"), b"z").unwrap();
        fs::write(r.join("skip/big.bin"), vec![0u8; 5000]).unwrap();
        t
    }

    fn total(root: &Path, opts: &WalkOptions) -> (u64, u64, u64) {
        let cancel = AtomicBool::new(false);
        let prog = WalkProgress::default();
        let next = AtomicU64::new(0);
        walk(root, opts, &cancel, &prog, &|_v: DirVisit| next.fetch_add(1, Ordering::Relaxed) as u32);
        (prog.files.load(Ordering::Relaxed), prog.dirs.load(Ordering::Relaxed), prog.bytes.load(Ordering::Relaxed))
    }

    #[test]
    fn counts_everything() {
        let t = tree();
        let (f, d, b) = total(t.path(), &WalkOptions::new(2));
        assert_eq!(f, 5);
        assert_eq!(d, 5); // kök, a, b, c, skip
        assert_eq!(b, 5 + 100 + 1000 + 1 + 5000);
    }

    #[test]
    fn respects_path_and_extension_exclusions() {
        let t = tree();
        let mut o = WalkOptions::new(1);
        o.excluded_paths = vec![paths::normalize(&t.path().join("skip").to_string_lossy())];
        o.excluded_exts.insert("log".into());
        let (f, d, b) = total(t.path(), &o);
        assert_eq!(f, 3);
        assert_eq!(d, 4);
        assert_eq!(b, 5 + 1000 + 1);
    }

    #[test]
    fn honours_cancellation() {
        let t = tree();
        let cancel = AtomicBool::new(true);
        let prog = WalkProgress::default();
        let out = walk(t.path(), &WalkOptions::new(1), &cancel, &prog, &|_v: DirVisit| 0);
        assert!(out.cancelled);
        assert_eq!(prog.files.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn missing_root_is_error_not_panic() {
        let cancel = AtomicBool::new(false);
        let prog = WalkProgress::default();
        let out = walk(Path::new("Z:\\bu\\yol\\yok\\ivily"), &WalkOptions::new(1), &cancel, &prog, &|_v: DirVisit| 0);
        assert_eq!(out.error_count, 1);
    }

    #[test]
    fn extension_parsing() {
        assert_eq!(extension_of("a.TXT"), "txt");
        assert_eq!(extension_of(".gitignore"), "");
        assert_eq!(extension_of("noext"), "");
        assert_eq!(extension_of("a.tar.gz"), "gz");
    }

    #[cfg(windows)]
    #[test]
    fn does_not_follow_junctions() {
        let t = tree();
        let link = t.path().join("loop");
        // Kökü gösteren bir junction: izlenseydi sonsuz döngü oluşurdu.
        let status = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(&link).arg(t.path()).output();
        if !matches!(status, Ok(ref o) if o.status.success()) {
            return; // junction oluşturulamadıysa testi atla
        }
        let (f, _, _) = total(t.path(), &WalkOptions::new(2));
        assert_eq!(f, 5);
    }
}
