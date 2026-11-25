//! Dosya gezgini, boş klasörler, özet hesaplama, klasör karşılaştırma,
//! envanter dışa aktarma ve depolama içgörüleri.
//!
//! Güvenlik kuralları:
//! - Bağlantı noktaları (junction, sembolik bağlantı) hiçbir zaman izlenmez.
//! - Korunan konumlar (`is_protected_path`) değiştirilmez.
//! - Boş klasör silme yalnızca `remove_dir` ile yapılır; içinde dosya olan
//!   bir klasör hiçbir koşulda silinmez.

use crate::cleanup::rules::is_protected_path;
use crate::error::{AppError, AppResult};
use crate::jobs::JobCtl;
use crate::scan::classify;
use crate::scan::tree::DiskScan;
use crate::scan::walker::{
    self, extension_of, DirVisit, FileEntry, WalkOptions, WalkProgress, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_OFFLINE,
    FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, FILE_ATTRIBUTE_RECALL_ON_OPEN, FILE_ATTRIBUTE_REPARSE_POINT,
    FILE_ATTRIBUTE_SYSTEM,
};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

const FILE_ATTRIBUTE_READONLY: u32 = 0x1;
const FILE_ATTRIBUTE_ARCHIVE: u32 = 0x20;
const FILE_ATTRIBUTE_COMPRESSED: u32 = 0x800;
const FILE_ATTRIBUTE_ENCRYPTED: u32 = 0x4000;

// ---------------------------------------------------------------------------
// Ortak yardımcılar
// ---------------------------------------------------------------------------

pub fn ts_of(t: std::io::Result<std::time::SystemTime>) -> i64 {
    t.ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(windows)]
pub fn attrs_of(md: &std::fs::Metadata) -> u32 {
    use std::os::windows::fs::MetadataExt;
    md.file_attributes()
}
#[cfg(not(windows))]
pub fn attrs_of(md: &std::fs::Metadata) -> u32 {
    if md.file_type().is_symlink() {
        FILE_ATTRIBUTE_REPARSE_POINT
    } else {
        0
    }
}

fn is_reparse(md: &std::fs::Metadata) -> bool {
    md.file_type().is_symlink() || attrs_of(md) & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

/// CSV alanı: `;`, `"` veya satır sonu içeriyorsa tırnak içine alınır.
pub fn csv_field(s: &str) -> String {
    if s.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Unix saniyesini yerel saatle "YYYY-AA-GG SS:DD:ss" biçimine çevirir; 0 → "".
pub fn fmt_local(ts: i64) -> String {
    use chrono::TimeZone;
    if ts <= 0 {
        return String::new();
    }
    chrono::Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

/// Türkçe biçimli boyut: "12,3 GB".
pub fn fmt_bytes(b: u64) -> String {
    const U: [&str; 5] = ["bayt", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{b} bayt")
    } else {
        format!("{v:.1} {}", U[i]).replace('.', ",")
    }
}

/// Dışa aktarma dosyaları için klasör: İndirilenler, yoksa Masaüstü.
pub fn export_dir() -> AppResult<PathBuf> {
    dirs::download_dir()
        .or_else(dirs::desktop_dir)
        .ok_or_else(|| AppError::NotFound("İndirilenler klasörü".into()))
}

pub fn stamp() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

fn validate_abs_dir(p: &str) -> AppResult<PathBuf> {
    let path = PathBuf::from(p.trim());
    if !path.is_absolute() {
        return Err(AppError::InvalidInput("tam bir klasör yolu girin (ör. C:\\Users)".into()));
    }
    if !path.is_dir() {
        return Err(AppError::NotFound(format!("{} klasörü bulunamadı", path.display())));
    }
    Ok(path)
}

// ---------------------------------------------------------------------------
// a) Klasör listeleme
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub dir_size: Option<u64>,
    pub dir_files: Option<u64>,
    pub modified: i64,
    pub created: i64,
    pub hidden: bool,
    pub system: bool,
    pub readonly: bool,
    pub reparse: bool,
    pub ext: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirListing {
    pub path: String,
    pub parent: Option<String>,
    pub entries: Vec<FsEntry>,
    pub errors: u32,
    pub scan_sizes_available: bool,
}

pub fn list_dir(path: &str, show_hidden: bool, scan: Option<&DiskScan>) -> AppResult<DirListing> {
    let p = validate_abs_dir(path)?;
    let rd = std::fs::read_dir(&p).map_err(|e| AppError::io(e, &p.to_string_lossy()))?;
    // Taramadaki karşılık gelen düğümün çocukları (küçük harfli ad → düğüm).
    let scan_children: Option<HashMap<String, u32>> = scan.and_then(|s| {
        if !crate::paths::is_within(&p.to_string_lossy(), &s.root.to_string_lossy()) {
            return None;
        }
        let id = s.find(&p.to_string_lossy())?;
        Some(s.nodes[id as usize].children.iter().map(|&c| (s.nodes[c as usize].name.to_lowercase(), c)).collect())
    });
    let mut entries = Vec::new();
    let mut errors = 0u32;
    for e in rd {
        let Ok(e) = e else {
            errors += 1;
            continue;
        };
        let Ok(md) = e.metadata() else {
            errors += 1;
            continue;
        };
        let attrs = attrs_of(&md);
        let hidden = attrs & FILE_ATTRIBUTE_HIDDEN != 0;
        if hidden && !show_hidden {
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        let is_dir = md.is_dir();
        let reparse = is_reparse(&md);
        let (dir_size, dir_files) = if is_dir && !reparse {
            match scan_children.as_ref().and_then(|m| m.get(&name.to_lowercase())) {
                Some(&id) => {
                    let n = &scan.unwrap().nodes[id as usize];
                    (Some(n.size), Some(n.files))
                }
                None => (None, None),
            }
        } else {
            (None, None)
        };
        let ext = if is_dir { String::new() } else { extension_of(&name) };
        entries.push(FsEntry {
            category: if is_dir { "folder".into() } else { classify::category_of(&ext, md.len()).to_string() },
            ext,
            path: e.path().to_string_lossy().into_owned(),
            name,
            is_dir,
            size: if is_dir { None } else { Some(md.len()) },
            dir_size,
            dir_files,
            modified: ts_of(md.modified()),
            created: ts_of(md.created()),
            hidden,
            system: attrs & FILE_ATTRIBUTE_SYSTEM != 0,
            readonly: attrs & FILE_ATTRIBUTE_READONLY != 0,
            reparse,
        });
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(DirListing {
        path: p.to_string_lossy().into_owned(),
        parent: p.parent().map(|x| x.to_string_lossy().into_owned()),
        entries,
        errors,
        scan_sizes_available: scan_children.is_some(),
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsRoot {
    pub path: String,
    pub label: String,
}

pub fn user_folders() -> Vec<(&'static str, &'static str, Option<PathBuf>)> {
    vec![
        ("desktop", "Masaüstü", dirs::desktop_dir()),
        ("documents", "Belgeler", dirs::document_dir()),
        ("downloads", "İndirilenler", dirs::download_dir()),
        ("pictures", "Resimler", dirs::picture_dir()),
        ("videos", "Videolar", dirs::video_dir()),
        ("music", "Müzik", dirs::audio_dir()),
    ]
}

pub fn roots(disks: &[crate::monitor::DiskInfo]) -> Vec<FsRoot> {
    let mut out: Vec<FsRoot> = Vec::new();
    for d in disks {
        let mount = d.mount.trim_end_matches('\\').to_string();
        let label = if d.name.trim().is_empty() {
            format!("Yerel Disk ({mount})")
        } else {
            format!("{} ({mount})", d.name.trim())
        };
        if !out.iter().any(|r| crate::paths::normalize(&r.path) == crate::paths::normalize(&d.mount)) {
            out.push(FsRoot { path: d.mount.clone(), label });
        }
    }
    for (_, label, p) in user_folders() {
        if let Some(p) = p.filter(|p| p.is_dir()) {
            out.push(FsRoot { path: p.to_string_lossy().into_owned(), label: label.into() });
        }
    }
    out
}

// ---------------------------------------------------------------------------
// b) Ayrıntılı bilgiler
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub product_name: Option<String>,
    pub file_version: Option<String>,
    pub company_name: Option<String>,
    pub file_description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsMetadata {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    /// Dosyalar için gerçek boyut; klasörler için tarama verisi varsa alt ağaç
    /// toplamı, yoksa 0.
    pub size: u64,
    pub modified: i64,
    pub created: i64,
    pub accessed: i64,
    pub attributes: Vec<String>,
    pub ext: String,
    pub category: String,
    pub protected: bool,
    pub version_info: Option<VersionInfo>,
}

pub fn attribute_labels(attrs: u32) -> Vec<String> {
    let mut v = Vec::new();
    let table: [(u32, &str); 7] = [
        (FILE_ATTRIBUTE_HIDDEN, "Gizli"),
        (FILE_ATTRIBUTE_SYSTEM, "Sistem"),
        (FILE_ATTRIBUTE_READONLY, "Salt okunur"),
        (FILE_ATTRIBUTE_ARCHIVE, "Arşiv"),
        (FILE_ATTRIBUTE_COMPRESSED, "Sıkıştırılmış"),
        (FILE_ATTRIBUTE_ENCRYPTED, "Şifreli"),
        (FILE_ATTRIBUTE_REPARSE_POINT, "Bağlantı noktası"),
    ];
    for (bit, label) in table {
        if attrs & bit != 0 {
            v.push(label.to_string());
        }
    }
    if attrs & (FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_RECALL_ON_OPEN | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS) != 0 {
        v.push("Çevrimdışı (bulut)".to_string());
    }
    v
}

pub fn metadata(path: &str, scan: Option<&DiskScan>) -> AppResult<FsMetadata> {
    let p = PathBuf::from(path.trim());
    if !p.is_absolute() {
        return Err(AppError::InvalidInput("tam bir yol gerekli".into()));
    }
    let md = std::fs::symlink_metadata(&p).map_err(|e| AppError::io(e, &p.to_string_lossy()))?;
    let attrs = attrs_of(&md);
    let is_dir = md.is_dir();
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.to_string_lossy().into_owned());
    let ext = if is_dir { String::new() } else { extension_of(&name) };
    let size = if is_dir {
        scan.and_then(|s| s.find(&p.to_string_lossy()).map(|id| s.nodes[id as usize].size)).unwrap_or(0)
    } else {
        md.len()
    };
    let version_info = if !is_dir && matches!(ext.as_str(), "exe" | "dll" | "sys" | "ocx" | "cpl" | "scr" | "drv" | "mui") {
        version_info(&p)
    } else {
        None
    };
    Ok(FsMetadata {
        path: p.to_string_lossy().into_owned(),
        category: if is_dir { "folder".into() } else { classify::category_of(&ext, size).to_string() },
        name,
        is_dir,
        size,
        modified: ts_of(md.modified()),
        created: ts_of(md.created()),
        accessed: ts_of(md.accessed()),
        attributes: attribute_labels(attrs),
        ext,
        protected: is_protected_path(&p.to_string_lossy()),
        version_info,
    })
}

#[cfg(windows)]
pub fn version_info(path: &Path) -> Option<VersionInfo> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
    let w = crate::sysmgmt::wide(&path.to_string_lossy());
    unsafe {
        let size = GetFileVersionInfoSizeW(PCWSTR(w.as_ptr()), None);
        if size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        GetFileVersionInfoW(PCWSTR(w.as_ptr()), None, size, data.as_mut_ptr() as *mut _).ok()?;
        let mut langs: Vec<(u16, u16)> = Vec::new();
        let tr = crate::sysmgmt::wide("\\VarFileInfo\\Translation");
        let mut ptr: *mut core::ffi::c_void = std::ptr::null_mut();
        let mut len = 0u32;
        if VerQueryValueW(data.as_ptr() as *const _, PCWSTR(tr.as_ptr()), &mut ptr, &mut len).as_bool() && len >= 4 {
            let n = (len / 4) as usize;
            let s = std::slice::from_raw_parts(ptr as *const u16, n * 2);
            for i in 0..n {
                langs.push((s[i * 2], s[i * 2 + 1]));
            }
        }
        langs.push((0x0409, 0x04B0));
        langs.push((0x0409, 0x04E4));
        let query = |key: &str| -> Option<String> {
            for (lang, cp) in &langs {
                let q = crate::sysmgmt::wide(&format!("\\StringFileInfo\\{lang:04x}{cp:04x}\\{key}"));
                let mut p: *mut core::ffi::c_void = std::ptr::null_mut();
                let mut l = 0u32;
                if VerQueryValueW(data.as_ptr() as *const _, PCWSTR(q.as_ptr()), &mut p, &mut l).as_bool() && l > 0 {
                    let s = std::slice::from_raw_parts(p as *const u16, l as usize);
                    let v = String::from_utf16_lossy(s).trim_end_matches('\0').trim().to_string();
                    if !v.is_empty() {
                        return Some(v);
                    }
                }
            }
            None
        };
        let vi = VersionInfo {
            product_name: query("ProductName"),
            file_version: query("FileVersion"),
            company_name: query("CompanyName"),
            file_description: query("FileDescription"),
        };
        if vi.product_name.is_none() && vi.file_version.is_none() && vi.company_name.is_none() && vi.file_description.is_none() {
            None
        } else {
            Some(vi)
        }
    }
}

#[cfg(not(windows))]
pub fn version_info(_path: &Path) -> Option<VersionInfo> {
    None
}

// ---------------------------------------------------------------------------
// c) Boş klasörler
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyFolder {
    pub id: u32,
    pub path: String,
    pub modified: i64,
    pub nested_dirs: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyFolders {
    pub items: Vec<EmptyFolder>,
    pub total: u64,
}

pub const EMPTY_LIMIT: usize = 2000;

/// Taramadan, tüm alt ağacında hiç dosya olmayan en üst düzey klasörleri
/// bulur. Okunamayan veya bağlantı içeren alt ağaçlar, tarama kökü, korunan
/// konumlar ve `$` ile başlayan sistem klasörleri dahil edilmez.
pub fn find_empty_folders(scan: &DiskScan) -> AppResult<EmptyFolders> {
    if scan.status != crate::scan::tree::ScanStatus::Complete {
        return Err(AppError::Conflict(
            "boş klasör analizi için tamamlanmış bir disk taraması gerekir; kısmi taramada klasörler yanlışlıkla boş görünebilir".into(),
        ));
    }
    let nodes = &scan.nodes;
    let n = nodes.len();
    // clean[i]: alt ağaç dosyasız, okunabilir ve bağlantısız.
    let mut clean = vec![false; n];
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(nodes[i].depth));
    for &i in &order {
        let nd = &nodes[i];
        clean[i] = nd.files == 0
            && !nd.unreadable
            && nd.skipped_links == 0
            && nd.children.iter().all(|&c| clean[c as usize]);
    }
    // eligible[i]: kök değil, `$` ataları yok, korunmuyor. Derinliğe göre
    // artan sırayla hesaplanır ki üst düğüm önce bilinsin.
    let mut dollar = vec![false; n];
    let mut eligible = vec![false; n];
    let mut items = Vec::new();
    let mut asc = order;
    asc.reverse();
    for &i in &asc {
        let nd = &nodes[i];
        if nd.parent == u32::MAX {
            continue;
        }
        let parent = nd.parent as usize;
        dollar[i] = dollar[parent] || nd.name.starts_with('$');
        if !clean[i] || dollar[i] {
            continue;
        }
        let path = scan.path_of(i as u32);
        let ps = path.to_string_lossy().into_owned();
        if is_protected_path(&ps) {
            continue;
        }
        eligible[i] = true;
        if eligible[parent] {
            continue; // üst klasör zaten listede
        }
        items.push(EmptyFolder { id: i as u32, path: ps, modified: nd.modified, nested_dirs: nd.dirs });
    }
    items.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));
    let total = items.len() as u64;
    items.truncate(EMPTY_LIMIT);
    Ok(EmptyFolders { items, total })
}

/// Alt ağacı gerçekten (diskte) dosyasız mı? Klasörleri alttan üste sırayla
/// döndürür; dosya, bağlantı veya okunamayan klasör varsa hata verir.
fn verify_empty_tree(p: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let rd = std::fs::read_dir(p).map_err(|e| format!("Klasör okunamadı: {}", AppError::io(e, &p.to_string_lossy())))?;
    for e in rd {
        let e = e.map_err(|_| "Klasör içeriği okunamadı".to_string())?;
        let md = e.metadata().map_err(|_| "Klasör içeriği okunamadı".to_string())?;
        if is_reparse(&md) {
            return Err("Klasör bir bağlantı noktası içeriyor".into());
        }
        if md.is_dir() {
            verify_empty_tree(&e.path(), out)?;
        } else {
            return Err("Klasörde dosya var".into());
        }
    }
    out.push(p.to_path_buf());
    Ok(())
}

/// Bir boş klasör alt ağacını siler. Silinen klasörleri (üstten alta) döndürür.
pub fn remove_empty_tree(path: &str) -> Result<Vec<String>, String> {
    let p = PathBuf::from(path.trim());
    if !p.is_absolute() {
        return Err("Geçersiz yol".into());
    }
    if is_protected_path(&p.to_string_lossy()) {
        return Err("Korunan sistem konumu".into());
    }
    let md = std::fs::symlink_metadata(&p).map_err(|e| AppError::io(e, &p.to_string_lossy()).to_string())?;
    if is_reparse(&md) {
        return Err("Bağlantı noktaları silinmez".into());
    }
    if !md.is_dir() {
        return Err("Klasör değil".into());
    }
    let mut order = Vec::new();
    verify_empty_tree(&p, &mut order)?;
    let mut removed = Vec::new();
    for d in &order {
        // remove_dir boş olmayan klasörde başarısız olur: arada dosya
        // eklenmişse hiçbir şey kaybolmaz.
        match std::fs::remove_dir(d) {
            Ok(()) => removed.push(d.to_string_lossy().into_owned()),
            Err(e) => {
                removed.reverse();
                return if removed.is_empty() {
                    Err(AppError::io(e, &d.to_string_lossy()).to_string())
                } else {
                    Err(format!("Klasör yalnızca kısmen silinebildi: {}", AppError::io(e, &d.to_string_lossy())))
                };
            }
        }
    }
    removed.reverse();
    Ok(removed)
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyRemoveResult {
    pub removed: Vec<String>,
    pub failed: Vec<(String, String)>,
    /// İç içe olanlar dahil silinen tüm klasörler (geri alma için).
    #[serde(skip)]
    pub all_removed: Vec<String>,
}

pub fn remove_empty_folders(paths: &[String]) -> EmptyRemoveResult {
    let mut r = EmptyRemoveResult::default();
    for p in paths {
        match remove_empty_tree(p) {
            Ok(dirs) => {
                r.removed.push(p.clone());
                r.all_removed.extend(dirs);
            }
            Err(e) => r.failed.push((p.clone(), e)),
        }
    }
    r
}

// ---------------------------------------------------------------------------
// d) Özet (hash) hesaplama
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashResult {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub blake3: String,
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

const CHUNK: usize = 1 << 20;

pub fn hash_file(path: &Path, ctl: &JobCtl) -> AppResult<HashResult> {
    use sha2::Digest;
    let md = std::fs::symlink_metadata(path).map_err(|e| AppError::io(e, &path.to_string_lossy()))?;
    if !md.is_file() || is_reparse(&md) {
        return Err(AppError::InvalidInput("yalnızca normal dosyaların özeti hesaplanabilir".into()));
    }
    ctl.set_phase("Özet hesaplanıyor", md.len());
    *ctl.progress.current.lock() = path.to_string_lossy().into_owned();
    let mut f = std::fs::File::open(path).map_err(|e| AppError::io(e, &path.to_string_lossy()))?;
    let mut sha = sha2::Sha256::new();
    let mut b3 = blake3::Hasher::new();
    let mut buf = vec![0u8; CHUNK];
    let mut size = 0u64;
    loop {
        if ctl.cancelled() {
            return Err(AppError::Cancelled);
        }
        let n = f.read(&mut buf).map_err(|e| AppError::io(e, &path.to_string_lossy()))?;
        if n == 0 {
            break;
        }
        sha.update(&buf[..n]);
        b3.update(&buf[..n]);
        size += n as u64;
        ctl.add_done(n as u64);
        ctl.progress.bytes.fetch_add(n as u64, Ordering::Relaxed);
    }
    Ok(HashResult {
        path: path.to_string_lossy().into_owned(),
        size,
        sha256: hex(&sha.finalize()),
        blake3: b3.finalize().to_hex().to_string(),
    })
}

// ---------------------------------------------------------------------------
// e) Klasör karşılaştırma
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareOnly {
    pub rel: String,
    pub size: u64,
    pub modified: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareDiff {
    pub rel: String,
    pub size_a: u64,
    pub size_b: u64,
    pub modified_a: i64,
    pub modified_b: i64,
    /// "size" | "content" | "modified"
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareReport {
    pub a: String,
    pub b: String,
    pub content: bool,
    pub only_a: Vec<CompareOnly>,
    pub only_b: Vec<CompareOnly>,
    pub different: Vec<CompareDiff>,
    pub identical: u64,
    pub only_a_bytes: u64,
    pub only_b_bytes: u64,
    pub errors: u64,
    pub cancelled: bool,
    pub truncated: bool,
}

pub const COMPARE_LIMIT: usize = 5000;

fn rel_map(root: &Path, files: Vec<(PathBuf, FileEntry)>) -> HashMap<String, (String, FileEntry, PathBuf)> {
    let mut m = HashMap::with_capacity(files.len());
    for (p, f) in files {
        let rel = p.strip_prefix(root).map(|r| r.to_string_lossy().into_owned()).unwrap_or_else(|_| f.name.clone());
        m.insert(rel.to_lowercase(), (rel, f, p));
    }
    m
}

fn blake3_file(p: &Path, ctl: &JobCtl) -> Option<blake3::Hash> {
    let mut f = std::fs::File::open(p).ok()?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; CHUNK];
    loop {
        if ctl.cancelled() {
            return None;
        }
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        ctl.add_done(n as u64);
    }
    Some(h.finalize())
}

pub fn same_dir(a: &Path, b: &Path) -> bool {
    crate::paths::normalize(&a.to_string_lossy()) == crate::paths::normalize(&b.to_string_lossy())
}

pub fn compare_folders(a: &Path, b: &Path, content: bool, opts: &WalkOptions, ctl: &JobCtl) -> CompareReport {
    ctl.set_phase("Klasörler taranıyor", 0);
    let (fa, oa) = walker::collect_files(a, opts, &ctl.cancel, &ctl.progress, |_, _| true);
    let (fb, ob) = walker::collect_files(b, opts, &ctl.cancel, &ctl.progress, |_, _| true);
    let mut rep = CompareReport {
        a: a.to_string_lossy().into_owned(),
        b: b.to_string_lossy().into_owned(),
        content,
        only_a: vec![],
        only_b: vec![],
        different: vec![],
        identical: 0,
        only_a_bytes: 0,
        only_b_bytes: 0,
        errors: oa.error_count + ob.error_count,
        cancelled: oa.cancelled || ob.cancelled,
        truncated: false,
    };
    if rep.cancelled {
        return rep;
    }
    let ma = rel_map(a, fa);
    let mb = rel_map(b, fb);
    let mut same_size: Vec<(&str, &FileEntry, &PathBuf, &FileEntry, &PathBuf)> = Vec::new();
    for (k, (rel, f, p)) in &ma {
        match mb.get(k) {
            None => {
                rep.only_a_bytes += f.size;
                rep.only_a.push(CompareOnly { rel: rel.clone(), size: f.size, modified: f.modified });
            }
            Some((_, g, q)) => {
                if f.size != g.size {
                    rep.different.push(CompareDiff {
                        rel: rel.clone(),
                        size_a: f.size,
                        size_b: g.size,
                        modified_a: f.modified,
                        modified_b: g.modified,
                        reason: "size".into(),
                    });
                } else {
                    same_size.push((rel, f, p, g, q));
                }
            }
        }
    }
    for (k, (rel, g, _)) in &mb {
        if !ma.contains_key(k) {
            rep.only_b_bytes += g.size;
            rep.only_b.push(CompareOnly { rel: rel.clone(), size: g.size, modified: g.modified });
        }
    }
    if content {
        let total: u64 = same_size.iter().map(|(_, f, ..)| f.size * 2).sum();
        ctl.set_phase("İçerik karşılaştırılıyor", total);
    }
    for (rel, f, p, g, q) in same_size {
        if ctl.cancelled() {
            rep.cancelled = true;
            break;
        }
        let reason = if content {
            if let Some(mut cur) = ctl.progress.current.try_lock() {
                *cur = rel.to_string();
            }
            match (blake3_file(p, ctl), blake3_file(q, ctl)) {
                (Some(x), Some(y)) => (x != y).then_some("content"),
                _ => {
                    if ctl.cancelled() {
                        rep.cancelled = true;
                        break;
                    }
                    rep.errors += 1;
                    continue;
                }
            }
        } else if (f.modified - g.modified).abs() > 2 {
            // FAT/exFAT 2 saniyelik zaman çözünürlüğü tolere edilir.
            Some("modified")
        } else {
            None
        };
        match reason {
            Some(r) => rep.different.push(CompareDiff {
                rel: rel.to_string(),
                size_a: f.size,
                size_b: g.size,
                modified_a: f.modified,
                modified_b: g.modified,
                reason: r.into(),
            }),
            None => rep.identical += 1,
        }
    }
    let key = |s: &str| s.to_lowercase();
    rep.only_a.sort_by_key(|x| key(&x.rel));
    rep.only_b.sort_by_key(|x| key(&x.rel));
    rep.different.sort_by_key(|x| key(&x.rel));
    rep.truncated =
        rep.only_a.len() > COMPARE_LIMIT || rep.only_b.len() > COMPARE_LIMIT || rep.different.len() > COMPARE_LIMIT;
    rep.only_a.truncate(COMPARE_LIMIT);
    rep.only_b.truncate(COMPARE_LIMIT);
    rep.different.truncate(COMPARE_LIMIT);
    rep
}

// ---------------------------------------------------------------------------
// f) Envanter dışa aktarma
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryResult {
    pub path: String,
    pub rows: u64,
    pub truncated: bool,
}

pub const INVENTORY_LIMIT: u64 = 500_000;

/// `root` altındaki dosyaları CSV olarak `out` dosyasına yazar.
pub fn write_inventory(root: &Path, recursive: bool, mut opts: WalkOptions, out: &Path, cap: u64) -> AppResult<InventoryResult> {
    if !recursive {
        opts.max_depth = Some(0);
    }
    let file = std::fs::File::create(out).map_err(|e| AppError::io(e, &out.to_string_lossy()))?;
    let w = parking_lot::Mutex::new(std::io::BufWriter::new(file));
    {
        let mut g = w.lock();
        g.write_all("\u{feff}Yol;Ad;Uzantı;Kategori;Boyut (bayt);Değiştirilme;Oluşturulma\r\n".as_bytes())
            .map_err(|e| AppError::io(e, &out.to_string_lossy()))?;
    }
    let rows = AtomicU64::new(0);
    let stop = AtomicBool::new(false);
    let truncated = AtomicBool::new(false);
    let write_err = AtomicBool::new(false);
    let next = AtomicU64::new(0);
    let progress = WalkProgress::default();
    let out_norm = crate::paths::normalize(&out.to_string_lossy());
    walker::walk(root, &opts, &stop, &progress, &|v: DirVisit| {
        let mut buf = String::new();
        let mut n = 0u64;
        for f in &v.files {
            let full = v.path.join(&f.name);
            let full_s = full.to_string_lossy();
            if crate::paths::normalize(&full_s) == out_norm {
                continue; // yazılmakta olan envanter dosyasının kendisi
            }
            let ext = f.extension();
            let line = [
                csv_field(&full_s),
                csv_field(&f.name),
                csv_field(&ext),
                csv_field(classify::category_label(classify::category_of(&ext, f.size))),
                f.size.to_string(),
                fmt_local(f.modified),
                fmt_local(f.created),
            ]
            .join(";");
            buf.push_str(&line);
            buf.push_str("\r\n");
            n += 1;
        }
        if n > 0 && !stop.load(Ordering::Relaxed) {
            // Sınırı aşmayacak kadar satır yaz.
            let before = rows.fetch_add(n, Ordering::SeqCst);
            let allowed = cap.saturating_sub(before).min(n);
            if allowed < n {
                truncated.store(true, Ordering::Relaxed);
                stop.store(true, Ordering::Relaxed);
                rows.fetch_sub(n - allowed, Ordering::SeqCst);
            }
            if allowed > 0 {
                let text: String = if allowed == n {
                    buf
                } else {
                    buf.split_inclusive("\r\n").take(allowed as usize).collect()
                };
                if w.lock().write_all(text.as_bytes()).is_err() {
                    write_err.store(true, Ordering::Relaxed);
                    stop.store(true, Ordering::Relaxed);
                }
            }
        }
        next.fetch_add(1, Ordering::Relaxed) as u32
    });
    let mut g = w.into_inner();
    if write_err.load(Ordering::Relaxed) || g.flush().is_err() {
        return Err(AppError::System("envanter dosyası yazılamadı".into()));
    }
    Ok(InventoryResult {
        path: out.to_string_lossy().into_owned(),
        rows: rows.load(Ordering::SeqCst),
        truncated: truncated.load(Ordering::Relaxed),
    })
}

// ---------------------------------------------------------------------------
// g) Depolama içgörüleri
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFolderStat {
    pub key: String,
    pub label: String,
    pub path: String,
    pub size: Option<u64>,
    pub files: Option<u64>,
    pub found: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStorageItem {
    pub name: String,
    pub path: String,
    pub location: String,
    pub size: u64,
    pub files: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TempLikeDir {
    pub path: String,
    pub size: u64,
    pub files: u64,
    /// "temp" | "cache" | "logs"
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bottleneck {
    /// "info" | "warn"
    pub level: String,
    pub title: String,
    pub text: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInsights {
    pub root: String,
    pub user_folders: Vec<UserFolderStat>,
    pub app_storage: Vec<AppStorageItem>,
    pub temp_like: Vec<TempLikeDir>,
    pub bottlenecks: Vec<Bottleneck>,
    pub coverage: String,
}

const MB: u64 = 1024 * 1024;
const GB: u64 = 1024 * MB;

fn temp_kind(name: &str) -> Option<&'static str> {
    match name.to_lowercase().as_str() {
        "temp" | "tmp" => Some("temp"),
        "cache" | "caches" => Some("cache"),
        "logs" | "log" => Some("logs"),
        _ => None,
    }
}

/// Uygulama veri kökleri: (ortam değişkeni / yol, konum etiketi).
pub fn app_roots() -> Vec<(PathBuf, &'static str)> {
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from).filter(|p| p.is_absolute());
    let mut v: Vec<(PathBuf, &'static str)> = Vec::new();
    for (p, label) in [
        (env("ProgramFiles"), "Program Files"),
        (env("ProgramFiles(x86)"), "Program Files (x86)"),
        (env("LOCALAPPDATA"), "AppData\\Local"),
        (env("APPDATA"), "AppData\\Roaming"),
        (env("ProgramData"), "ProgramData"),
    ] {
        if let Some(p) = p {
            if !v.iter().any(|(q, _)| same_dir(q, &p)) {
                v.push((p, label));
            }
        }
    }
    v
}

pub struct InsightInputs<'a> {
    pub scan: &'a DiskScan,
    /// Taranan kökün bulunduğu birim: (bağlama noktası, toplam, boş).
    pub volume: Option<(String, u64, u64)>,
    pub growth: Option<crate::snapshots::GrowthReport>,
    pub user_folders: Vec<(&'static str, &'static str, Option<PathBuf>)>,
    pub app_roots: Vec<(PathBuf, &'static str)>,
}

pub fn insights(inp: InsightInputs) -> StorageInsights {
    let scan = inp.scan;
    let root_s = scan.root.to_string_lossy().into_owned();
    let root_size = scan.nodes.first().map(|n| n.size).unwrap_or(0);
    let mut uncovered: Vec<String> = Vec::new();

    // Kullanıcı klasörleri
    let mut user_folders = Vec::new();
    for (key, label, p) in inp.user_folders {
        let Some(p) = p else { continue };
        let ps = p.to_string_lossy().into_owned();
        let id = scan.find(&ps);
        if id.is_none() {
            uncovered.push(label.to_string());
        }
        user_folders.push(UserFolderStat {
            key: key.into(),
            label: label.into(),
            size: id.map(|i| scan.nodes[i as usize].size),
            files: id.map(|i| scan.nodes[i as usize].files),
            found: id.is_some(),
            path: ps,
        });
    }

    // Uygulama depolaması
    let mut app_storage = Vec::new();
    let mut app_uncovered = Vec::new();
    for (root, location) in &inp.app_roots {
        let Some(id) = scan.find(&root.to_string_lossy()) else {
            app_uncovered.push(location.to_string());
            continue;
        };
        for &c in &scan.nodes[id as usize].children {
            let n = &scan.nodes[c as usize];
            let lname = n.name.to_lowercase();
            if *location == "AppData\\Local" && lname == "temp" {
                continue; // geçici dosyalar ayrıca değerlendirilir
            }
            let name = if *location == "AppData\\Local" && lname == "packages" {
                "Microsoft Store uygulamaları".to_string()
            } else {
                n.name.clone()
            };
            app_storage.push(AppStorageItem {
                name,
                path: scan.path_of(c).to_string_lossy().into_owned(),
                location: location.to_string(),
                size: n.size,
                files: n.files,
            });
        }
    }
    app_storage.sort_by(|a, b| b.size.cmp(&a.size));
    app_storage.truncate(40);

    // Geçici / önbellek / günlük klasörleri (en üst düzey olanlar)
    let nodes = &scan.nodes;
    let qualifies = |i: usize| nodes[i].size >= 100 * MB && temp_kind(&nodes[i].name).is_some() && nodes[i].parent != u32::MAX;
    let mut temp_like = Vec::new();
    for i in 0..nodes.len() {
        if !qualifies(i) {
            continue;
        }
        let mut cur = nodes[i].parent;
        let mut covered = false;
        while cur != u32::MAX && (cur as usize) < nodes.len() {
            if qualifies(cur as usize) {
                covered = true;
                break;
            }
            cur = nodes[cur as usize].parent;
        }
        if covered {
            continue;
        }
        temp_like.push(TempLikeDir {
            path: scan.path_of(i as u32).to_string_lossy().into_owned(),
            size: nodes[i].size,
            files: nodes[i].files,
            kind: temp_kind(&nodes[i].name).unwrap_or("temp").into(),
        });
    }
    temp_like.sort_by(|a, b| b.size.cmp(&a.size));
    let temp_total: u64 = temp_like.iter().map(|t| t.size).sum();
    let temp_count = temp_like.len();
    temp_like.truncate(25);

    // Darboğazlar — yalnızca ölçülen verilere dayanır.
    let mut bottlenecks = Vec::new();
    if let Some((mount, total, free)) = &inp.volume {
        if *total > 0 {
            let used = total.saturating_sub(*free);
            let pct = used as f64 * 100.0 / *total as f64;
            if pct >= 90.0 {
                bottlenecks.push(Bottleneck {
                    level: "warn".into(),
                    title: "Disk neredeyse dolu".into(),
                    text: "Birimde boş alan azaldı. Windows güncellemeleri, sanal bellek ve uygulamalar için boş alan gerekir; en büyük klasörleri ve geçici verileri gözden geçirin.".into(),
                    evidence: format!("{mount} birimi %{pct:.0} dolu ({} / {}, boş: {})", fmt_bytes(used), fmt_bytes(*total), fmt_bytes(*free)),
                });
            }
        }
    }
    if root_size >= GB {
        if let Some(&top) = nodes.first().and_then(|r| r.children.iter().max_by_key(|&&c| nodes[c as usize].size)) {
            let n = &nodes[top as usize];
            let pct = n.size as f64 * 100.0 / root_size as f64;
            if pct >= 40.0 {
                bottlenecks.push(Bottleneck {
                    level: "info".into(),
                    title: "Alanın büyük kısmı tek bir klasörde".into(),
                    text: "Taranan alanın önemli bir bölümü tek bir üst düzey klasörde toplanmış. Yer açmak istiyorsanız incelemeye buradan başlamak en etkili yoldur.".into(),
                    evidence: format!("{}: {} (taranan alanın %{pct:.0}'i)", scan.path_of(top).display(), fmt_bytes(n.size)),
                });
            }
        }
    }
    if temp_total >= 5 * GB {
        bottlenecks.push(Bottleneck {
            level: "warn".into(),
            title: "Geçici ve önbellek verileri çok yer kaplıyor".into(),
            text: "Geçici dosya, önbellek ve günlük klasörleri büyük bir alan kaplıyor. Temizlik bölümünü veya ilgili uygulamanın kendi temizleme seçeneğini kullanın.".into(),
            evidence: format!("{temp_count} klasörde toplam {}", fmt_bytes(temp_total)),
        });
    }
    if let Some(g) = &inp.growth {
        for it in g.items.iter().filter(|i| i.fast).take(3) {
            bottlenecks.push(Bottleneck {
                level: "info".into(),
                title: "Hızla büyüyen klasör".into(),
                text: "Bu klasör son taramalar arasında belirgin şekilde büyüdü. Hangi uygulamanın veri yazdığını kontrol edin.".into(),
                evidence: format!(
                    "{}: {} → {} (günde yaklaşık {})",
                    it.path,
                    fmt_bytes(it.old_size),
                    fmt_bytes(it.new_size),
                    fmt_bytes(it.per_day.max(0.0) as u64)
                ),
            });
        }
    }

    // Kapsam açıklaması
    let mut cov = Vec::new();
    if scan.status != crate::scan::tree::ScanStatus::Complete {
        cov.push("Tarama tamamlanmadığı için sonuçlar kısmidir.".to_string());
    }
    if !uncovered.is_empty() {
        cov.push(format!("Şu kullanıcı klasörleri taranan konumun dışında kaldı: {}.", uncovered.join(", ")));
    }
    if !app_uncovered.is_empty() {
        cov.push(format!("Şu uygulama konumları taranan konumun dışında kaldı: {}.", app_uncovered.join(", ")));
    }
    if scan.error_count > 0 {
        cov.push(format!("{} konum erişim izni veya kilit nedeniyle okunamadı; boyutları hesaba katılmadı.", scan.error_count));
    }
    if inp.volume.is_none() {
        cov.push("Taranan konumun bulunduğu birimin doluluk bilgisi alınamadı.".to_string());
    }
    match &inp.growth {
        Some(g) if g.enough_data => {}
        _ => cov.push("Büyüme analizi için bu konumun son 30 günde en az iki tamamlanmış taraması gerekir.".to_string()),
    }
    let coverage = if cov.is_empty() {
        "Taranan konumun tamamı analiz edildi.".to_string()
    } else {
        cov.join(" ")
    };

    StorageInsights { root: root_s, user_folders, app_storage, temp_like, bottlenecks, coverage }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::tree;
    use std::fs;

    fn scan_of(root: &Path) -> DiskScan {
        tree::scan(root, &WalkOptions::new(2), 1024, &AtomicBool::new(false), &WalkProgress::default())
    }

    #[test]
    fn csv_and_format_helpers() {
        assert_eq!(csv_field("a;b"), "\"a;b\"");
        assert_eq!(csv_field("a\"b"), "\"a\"\"b\"");
        assert_eq!(csv_field("düz"), "düz");
        assert_eq!(fmt_bytes(512), "512 bayt");
        assert_eq!(fmt_bytes(1536), "1,5 KB");
        assert_eq!(fmt_local(0), "");
    }

    #[test]
    fn lists_directory_with_hidden_and_scan_sizes() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        fs::create_dir_all(r.join("Alt/İç")).unwrap();
        fs::write(r.join("Alt/İç/x.bin"), vec![0u8; 300]).unwrap();
        fs::write(r.join("b.txt"), b"12345").unwrap();
        let hidden = r.join("gizli.txt");
        fs::write(&hidden, b"1").unwrap();
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("attrib").arg("+h").arg(&hidden).output();
        }
        let scan = scan_of(r);
        let l = list_dir(&r.to_string_lossy(), false, Some(&scan)).unwrap();
        assert!(l.scan_sizes_available);
        assert!(l.entries[0].is_dir, "klasörler önce gelir");
        assert_eq!(l.entries[0].name, "Alt");
        assert_eq!(l.entries[0].dir_size, Some(300));
        assert_eq!(l.entries[0].dir_files, Some(1));
        assert_eq!(l.entries[0].category, "folder");
        let b = l.entries.iter().find(|e| e.name == "b.txt").unwrap();
        assert_eq!(b.size, Some(5));
        assert_eq!(b.ext, "txt");
        assert_eq!(b.category, "document");
        let hidden_attr_applied = fs::metadata(&hidden).map(|m| attrs_of(&m) & FILE_ATTRIBUTE_HIDDEN != 0).unwrap_or(false);
        if hidden_attr_applied {
            assert!(l.entries.iter().all(|e| e.name != "gizli.txt"));
            let l2 = list_dir(&r.to_string_lossy(), true, None).unwrap();
            let h = l2.entries.iter().find(|e| e.name == "gizli.txt").unwrap();
            assert!(h.hidden);
            assert!(!l2.scan_sizes_available);
        }
        assert_eq!(l.parent.as_deref(), r.parent().map(|p| p.to_string_lossy()).as_deref());
        assert!(list_dir("goreli\\yol", false, None).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn flags_junctions_as_reparse() {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir_all(t.path().join("hedef")).unwrap();
        let link = t.path().join("baglanti");
        let ok = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(t.path().join("hedef"))
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !ok {
            return;
        }
        let l = list_dir(&t.path().to_string_lossy(), true, None).unwrap();
        let j = l.entries.iter().find(|e| e.name == "baglanti").unwrap();
        assert!(j.reparse);
        let m = metadata(&link.to_string_lossy(), None).unwrap();
        assert!(m.attributes.iter().any(|a| a == "Bağlantı noktası"));
        // Bağlantı noktası boş klasör olarak silinmez.
        assert!(remove_empty_tree(&link.to_string_lossy()).is_err());
        assert!(link.exists());
        assert!(t.path().join("hedef").exists());
    }

    #[test]
    fn metadata_of_file() {
        let t = tempfile::tempdir().unwrap();
        let f = t.path().join("rapor.pdf");
        fs::write(&f, vec![0u8; 42]).unwrap();
        let m = metadata(&f.to_string_lossy(), None).unwrap();
        assert_eq!(m.size, 42);
        assert_eq!(m.ext, "pdf");
        assert_eq!(m.category, "document");
        assert!(!m.is_dir);
        assert!(!m.protected);
        assert!(m.version_info.is_none());
        assert_eq!(attribute_labels(0x1 | 0x2 | 0x400 | 0x400000), vec!["Gizli", "Salt okunur", "Bağlantı noktası", "Çevrimdışı (bulut)"]);
    }

    #[cfg(windows)]
    #[test]
    fn reads_version_info_of_system_exe() {
        let w = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        let p = PathBuf::from(w).join("System32\\notepad.exe");
        if !p.exists() {
            return;
        }
        let m = metadata(&p.to_string_lossy(), None).unwrap();
        assert!(m.protected);
        let vi = m.version_info.expect("notepad sürüm bilgisi");
        assert!(vi.company_name.unwrap_or_default().contains("Microsoft"));
    }

    fn empty_fixture() -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        fs::create_dir_all(r.join("bos/a/b")).unwrap();
        fs::create_dir_all(r.join("bos/c")).unwrap();
        fs::create_dir_all(r.join("dolu/bos_alt")).unwrap();
        fs::write(r.join("dolu/x.txt"), b"x").unwrap();
        fs::create_dir_all(r.join("$sistem/ic")).unwrap();
        fs::create_dir_all(r.join(".gizli_bos")).unwrap();
        t
    }

    #[test]
    fn detects_top_most_empty_folders() {
        let t = empty_fixture();
        let scan = scan_of(t.path());
        let e = find_empty_folders(&scan).unwrap();
        let names: Vec<String> = e
            .items
            .iter()
            .map(|i| Path::new(&i.path).strip_prefix(t.path()).unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec![".gizli_bos", "bos", "dolu\\bos_alt"]);
        let bos = e.items.iter().find(|i| i.path.ends_with("\\bos")).unwrap();
        assert_eq!(bos.nested_dirs, 3);
        assert_eq!(e.total, 3);
    }

    #[test]
    fn removes_empty_folders_but_never_one_with_a_file() {
        let t = empty_fixture();
        let scan = scan_of(t.path());
        let e = find_empty_folders(&scan).unwrap();
        // Taramadan sonra bir klasöre dosya eklendi.
        fs::write(t.path().join("bos/a/b/yeni.txt"), b"yeni").unwrap();
        let paths: Vec<String> = e.items.iter().map(|i| i.path.clone()).collect();
        let r = remove_empty_folders(&paths);
        assert_eq!(r.removed.len(), 2);
        assert_eq!(r.failed.len(), 1);
        assert!(r.failed[0].0.ends_with("\\bos"));
        assert!(t.path().join("bos/a/b/yeni.txt").exists());
        assert!(t.path().join("bos/c").exists(), "kısmen boş ağaçtan hiçbir şey silinmemeli");
        assert!(!t.path().join("dolu/bos_alt").exists());
        assert!(t.path().join("dolu/x.txt").exists());
        assert!(!t.path().join(".gizli_bos").exists());
        // Dosya ve korunan yol reddedilir.
        assert!(remove_empty_tree(&t.path().join("dolu/x.txt").to_string_lossy()).is_err());
        assert!(remove_empty_tree("C:\\").is_err());
    }

    #[test]
    fn hashes_known_vector() {
        let t = tempfile::tempdir().unwrap();
        let f = t.path().join("abc.txt");
        fs::write(&f, b"abc").unwrap();
        let jobs = crate::jobs::Jobs::default();
        let ctl = jobs.start("hash").unwrap();
        let h = hash_file(&f, &ctl).unwrap();
        assert_eq!(h.sha256, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(h.blake3, blake3::hash(b"abc").to_hex().to_string());
        assert_eq!(h.size, 3);
        assert_eq!(ctl.done.load(Ordering::Relaxed), 3);
        ctl.cancel.store(true, Ordering::Relaxed);
        assert!(matches!(hash_file(&f, &ctl), Err(AppError::Cancelled)));
        assert!(hash_file(t.path(), &ctl).is_err());
    }

    fn set_mtime(p: &Path, secs_ago: u64) {
        let t = std::time::SystemTime::now() - std::time::Duration::from_secs(secs_ago);
        fs::File::options().write(true).open(p).unwrap().set_modified(t).unwrap();
    }

    #[test]
    fn compares_folders() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let (pa, pb) = (a.path(), b.path());
        fs::create_dir_all(pa.join("alt")).unwrap();
        fs::create_dir_all(pb.join("ALT")).unwrap();
        fs::write(pa.join("yalniz_a.txt"), b"aaaa").unwrap();
        fs::write(pb.join("yalniz_b.txt"), b"bb").unwrap();
        fs::write(pa.join("alt/ayni.txt"), b"ayni").unwrap();
        fs::write(pb.join("ALT/Ayni.txt"), b"ayni").unwrap();
        fs::write(pa.join("boyut.txt"), b"1").unwrap();
        fs::write(pb.join("boyut.txt"), b"12").unwrap();
        fs::write(pa.join("icerik.txt"), b"abc").unwrap();
        fs::write(pb.join("icerik.txt"), b"xyz").unwrap();
        fs::write(pa.join("tarih.txt"), b"t").unwrap();
        fs::write(pb.join("tarih.txt"), b"t").unwrap();
        let now = std::time::SystemTime::now();
        for p in [pa.join("alt/ayni.txt"), pb.join("ALT/Ayni.txt"), pa.join("icerik.txt"), pb.join("icerik.txt")] {
            fs::File::options().write(true).open(&p).unwrap().set_modified(now).unwrap();
        }
        set_mtime(&pa.join("tarih.txt"), 0);
        set_mtime(&pb.join("tarih.txt"), 3600);

        let jobs = crate::jobs::Jobs::default();
        let ctl = jobs.start("compare").unwrap();
        let r = compare_folders(pa, pb, false, &WalkOptions::new(2), &ctl);
        assert_eq!(r.only_a.len(), 1);
        assert_eq!(r.only_a[0].rel, "yalniz_a.txt");
        assert_eq!(r.only_a_bytes, 4);
        assert_eq!(r.only_b.len(), 1);
        assert_eq!(r.only_b_bytes, 2);
        let reason = |rel: &str| r.different.iter().find(|d| d.rel == rel).map(|d| d.reason.clone());
        assert_eq!(reason("boyut.txt").as_deref(), Some("size"));
        assert_eq!(reason("tarih.txt").as_deref(), Some("modified"));
        assert_eq!(reason("icerik.txt"), None, "içerik denetimi kapalıyken aynı boyut + aynı tarih özdeş sayılır");
        assert_eq!(r.identical, 2);
        assert!(!r.cancelled && !r.truncated);

        let r = compare_folders(pa, pb, true, &WalkOptions::new(2), &ctl);
        let reason = |rel: &str| r.different.iter().find(|d| d.rel == rel).map(|d| d.reason.clone());
        assert_eq!(reason("icerik.txt").as_deref(), Some("content"));
        assert_eq!(reason("tarih.txt"), None, "içerik aynıysa tarih farkı önemsizdir");
        assert_eq!(reason("boyut.txt").as_deref(), Some("size"));
        assert_eq!(r.identical, 2);
    }

    #[test]
    fn writes_inventory_csv() {
        let t = tempfile::tempdir().unwrap();
        let out_dir = tempfile::tempdir().unwrap();
        let r = t.path();
        fs::create_dir_all(r.join("alt")).unwrap();
        fs::write(r.join("a;b.txt"), b"123").unwrap();
        fs::write(r.join("alt/c.jpg"), b"1").unwrap();
        let out = out_dir.path().join("env.csv");
        let res = write_inventory(r, false, WalkOptions::new(1), &out, INVENTORY_LIMIT).unwrap();
        assert_eq!((res.rows, res.truncated), (1, false));
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.starts_with("\u{feff}Yol;Ad;Uzantı;Kategori;Boyut (bayt);Değiştirilme;Oluşturulma\r\n"));
        assert!(text.contains(";\"a;b.txt\";txt;Belgeler;3;"));
        let res = write_inventory(r, true, WalkOptions::new(1), &out, INVENTORY_LIMIT).unwrap();
        assert_eq!(res.rows, 2);
        assert!(fs::read_to_string(&out).unwrap().contains(";c.jpg;jpg;Görseller;1;"));
        let res = write_inventory(r, true, WalkOptions::new(1), &out, 1).unwrap();
        assert_eq!((res.rows, res.truncated), (1, true));
        assert_eq!(fs::read_to_string(&out).unwrap().lines().count(), 2);
    }

    #[test]
    fn storage_insights_from_scan() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        fs::create_dir_all(r.join("Kullanici/Belgeler")).unwrap();
        fs::create_dir_all(r.join("Local/Packages/App1")).unwrap();
        fs::create_dir_all(r.join("Local/Temp")).unwrap();
        fs::create_dir_all(r.join("Local/Uyg/cache/cache")).unwrap();
        fs::write(r.join("Kullanici/Belgeler/a.docx"), vec![0u8; 10]).unwrap();
        fs::write(r.join("Local/Packages/App1/x"), vec![0u8; 20]).unwrap();
        fs::write(r.join("Local/Temp/t"), vec![0u8; 5]).unwrap();
        // 100 MB eşiğini seyrek dosyayla aşmadan sınamak için boyutu ayarla.
        let big = fs::File::create(r.join("Local/Uyg/cache/cache/blob")).unwrap();
        big.set_len(150 * MB).unwrap();
        drop(big);
        let scan = scan_of(r);
        let ins = insights(InsightInputs {
            scan: &scan,
            volume: Some(("T:\\".into(), 100 * GB, 5 * GB)),
            growth: None,
            user_folders: vec![
                ("documents", "Belgeler", Some(r.join("Kullanici/Belgeler"))),
                ("music", "Müzik", Some(PathBuf::from("Z:\\yok\\Muzik"))),
            ],
            app_roots: vec![(r.join("Local"), "AppData\\Local"), (PathBuf::from("Z:\\yok\\PF"), "Program Files")],
        });
        let docs = &ins.user_folders[0];
        assert!(docs.found);
        assert_eq!(docs.size, Some(10));
        assert!(!ins.user_folders[1].found);
        assert!(ins.app_storage.iter().any(|a| a.name == "Microsoft Store uygulamaları" && a.size == 20));
        assert!(ins.app_storage.iter().all(|a| a.name != "Temp"));
        assert_eq!(ins.temp_like.len(), 1, "yalnızca en üst düzey önbellek klasörü");
        assert!(ins.temp_like[0].path.ends_with("Uyg\\cache"));
        assert_eq!(ins.temp_like[0].kind, "cache");
        assert!(ins.bottlenecks.iter().any(|b| b.level == "warn" && b.evidence.contains("%95")));
        assert!(ins.coverage.contains("Müzik"));
        assert!(ins.coverage.contains("Program Files"));
    }
}
