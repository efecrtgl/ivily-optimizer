//! Disk analizi, büyük/küçük/eski/yinelenen dosya komutları.

use crate::error::{AppError, AppResult};
use crate::jobs::{self, JobCtl};
use crate::oplog::{self, Outcome};
use crate::scan::dupes::DupReport;
use crate::scan::tree::{self, ExtStat, LargeFile, NodeView, SmallFileHotspot};
use crate::scan::walker::ScanErrorItem;
use crate::scan::ColdReport;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

type St<'a> = State<'a, Arc<AppState>>;

fn validate_dir(root: &str) -> AppResult<PathBuf> {
    let p = PathBuf::from(root.trim());
    if !p.is_absolute() {
        return Err(AppError::InvalidInput("tam bir klasör yolu girin (ör. C:\\Users)".into()));
    }
    if !p.is_dir() {
        return Err(AppError::NotFound(format!("{} klasörü bulunamadı", p.display())));
    }
    Ok(p)
}

/// Taranan kök bir birimin köküyse kullanılan alanı tahmin için döndürür.
fn volume_of(st: &AppState, root: &Path) -> Option<(u64, u64)> {
    let r = crate::paths::normalize(&root.to_string_lossy());
    st.monitor
        .lock()
        .disks()
        .into_iter()
        .find(|d| crate::paths::normalize(&d.mount) == r)
        .map(|d| (d.total, d.available))
}

fn spawn_job<F>(app: AppHandle, ctl: JobCtl, estimate: Option<u64>, work: F)
where
    F: FnOnce(&Arc<AppState>, &JobCtl) -> (String, Option<String>, serde_json::Value) + Send + 'static,
{
    std::thread::spawn(move || {
        let st = app.state::<Arc<AppState>>().inner().clone();
        let stop = jobs::spawn_reporter(app.clone(), ctl.clone(), estimate);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(&st, &ctl)));
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        st.jobs.finish(&ctl.id);
        match result {
            Ok((status, msg, summary)) => jobs::emit_done(&app, &ctl, &status, msg, summary),
            Err(_) => {
                tracing::error!(job = %ctl.id, "iş beklenmedik şekilde sonlandı");
                jobs::emit_done(&app, &ctl, "failed", Some("İşlem beklenmedik bir hata nedeniyle durdu.".into()), serde_json::Value::Null)
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Disk taraması
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn scan_start(app: AppHandle, st: St<'_>, root: String) -> AppResult<String> {
    let root = validate_dir(&root)?;
    let ctl = st.jobs.start("disk-scan")?;
    let id = ctl.id.clone();
    let volume = volume_of(&st, &root);
    let estimate = volume.map(|(t, f)| t.saturating_sub(f));
    tracing::info!(root = %root.display(), "disk taraması başladı");
    spawn_job(app, ctl, estimate, move |st, ctl| {
        let (opts, thr) = {
            let s = st.settings.read();
            (
                crate::scan::walker::WalkOptions::new(s.scan_threads).with_exclusions(&s.exclusions),
                s.small_file_threshold_kb * 1024,
            )
        };
        let scan = tree::scan(&root, &opts, thr, &ctl.cancel, &ctl.progress);
        let status = scan.status.as_str().to_string();
        let mut snapshot_id = None;
        if scan.status != tree::ScanStatus::Failed {
            match crate::snapshots::save(&st.db, &scan, volume) {
                Ok(id) => snapshot_id = Some(id),
                Err(e) => tracing::warn!(error = %e, "anlık görüntü kaydedilemedi"),
            }
            let s = st.settings();
            let _ = crate::snapshots::prune(&st.db, s.snapshot_retention_days, s.snapshot_max_count);
        }
        let summary = serde_json::json!({
            "root": root,
            "size": scan.nodes.first().map(|n| n.size).unwrap_or(0),
            "files": scan.nodes.first().map(|n| n.files).unwrap_or(0),
            "errors": scan.error_count,
            "durationMs": scan.duration_ms,
            "snapshotId": snapshot_id,
        });
        tracing::info!(status = %status, files = scan.nodes.first().map(|n| n.files).unwrap_or(0), "disk taraması bitti");
        let msg = match scan.status {
            tree::ScanStatus::Cancelled => Some("Tarama iptal edildi. Sonuçlar kısmidir.".into()),
            tree::ScanStatus::Failed => Some("Klasör okunamadı.".into()),
            _ => None,
        };
        if !scan.nodes.is_empty() {
            *st.disk_scan.write() = Some(Arc::new(scan));
        }
        (status, msg, summary)
    });
    Ok(id)
}

#[tauri::command]
pub async fn job_cancel(st: St<'_>, id: String) -> AppResult<bool> {
    Ok(st.jobs.cancel(&id))
}

#[tauri::command]
pub async fn jobs_running(st: St<'_>) -> AppResult<Vec<jobs::JobInfo>> {
    Ok(st.jobs.running())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub root: String,
    pub started_at: i64,
    pub duration_ms: u64,
    pub status: String,
    pub total_size: u64,
    pub files: u64,
    pub dirs: u64,
    pub error_count: u64,
    pub errors: Vec<ScanErrorItem>,
    pub skipped_links: u64,
    pub cloud_bytes: u64,
    pub categories: Vec<ExtStat>,
    pub top_exts: Vec<ExtStat>,
    pub largest_dirs: Vec<NodeView>,
    pub hist: [u64; 8],
    pub hist_labels: [&'static str; 8],
    pub small_threshold: u64,
}

fn current(st: &AppState) -> AppResult<Arc<tree::DiskScan>> {
    st.disk_scan.read().clone().ok_or_else(|| AppError::NotFound("henüz bir disk taraması yapılmadı".into()))
}

#[tauri::command]
pub async fn scan_summary(st: St<'_>) -> AppResult<Option<ScanSummary>> {
    let Some(s) = st.disk_scan.read().clone() else { return Ok(None) };
    let root = &s.nodes[0];
    let mut top_exts = s.ext_stats();
    top_exts.truncate(40);
    Ok(Some(ScanSummary {
        root: s.root.to_string_lossy().into_owned(),
        started_at: s.started_at,
        duration_ms: s.duration_ms,
        status: s.status.as_str().into(),
        total_size: root.size,
        files: root.files,
        dirs: root.dirs,
        error_count: s.error_count,
        errors: s.errors.iter().take(200).cloned().collect(),
        skipped_links: s.nodes.iter().map(|n| n.skipped_links as u64).sum(),
        cloud_bytes: s.cloud_bytes,
        categories: s.category_stats(),
        top_exts,
        largest_dirs: s.largest_dirs(15, 3),
        hist: root.hist,
        hist_labels: tree::HIST_LABELS,
        small_threshold: s.small_threshold,
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDetail {
    pub node: NodeView,
    pub children: Vec<NodeView>,
    pub breadcrumb: Vec<(u32, String)>,
    pub hist: [u64; 8],
}

#[tauri::command]
pub async fn scan_node(st: St<'_>, id: u32) -> AppResult<NodeDetail> {
    let s = current(&st)?;
    let node = s.view(id).ok_or_else(|| AppError::NotFound("klasör".into()))?;
    let mut breadcrumb = Vec::new();
    let mut cur = id;
    while let Some(n) = s.nodes.get(cur as usize) {
        breadcrumb.push((cur, if n.parent == u32::MAX { s.root.to_string_lossy().into_owned() } else { n.name.clone() }));
        if n.parent == u32::MAX {
            break;
        }
        cur = n.parent;
    }
    breadcrumb.reverse();
    Ok(NodeDetail { children: s.children(id), hist: s.hist(id), node, breadcrumb })
}

#[tauri::command]
pub async fn scan_find(st: St<'_>, path: String) -> AppResult<Option<u32>> {
    Ok(current(&st)?.find(&path))
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LargeFilter {
    pub min_size: u64,
    pub extension: Option<String>,
    pub category: Option<String>,
    pub location: Option<String>,
    pub limit: Option<usize>,
}

#[tauri::command]
pub async fn scan_large_files(st: St<'_>, filter: LargeFilter) -> AppResult<Vec<LargeFile>> {
    let s = current(&st)?;
    let ext = filter.extension.map(|e| e.trim().trim_start_matches('.').to_lowercase()).filter(|e| !e.is_empty());
    let loc = filter.location.map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty());
    let excluded = st.settings.read().exclusions.clone();
    Ok(s.largest
        .iter()
        .filter(|f| f.size >= filter.min_size)
        .filter(|f| ext.as_ref().map(|e| &f.ext == e).unwrap_or(true))
        .filter(|f| filter.category.as_ref().map(|c| &f.category == c).unwrap_or(true))
        .filter(|f| loc.as_ref().map(|l| f.path.to_lowercase().contains(l)).unwrap_or(true))
        .filter(|f| !excluded.paths.iter().any(|x| crate::paths::is_within(&f.path, x)))
        .take(filter.limit.unwrap_or(500))
        .cloned()
        .collect())
}

#[tauri::command]
pub async fn scan_small_files(st: St<'_>) -> AppResult<Vec<SmallFileHotspot>> {
    let s = current(&st)?;
    let min = st.settings.read().small_file_min_count;
    Ok(tree::small_file_hotspots(&s, min, 100))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: i64,
    pub ext: String,
    pub category: String,
}

/// Bir klasörün doğrudan içerdiği dosyaları canlı olarak listeler.
#[tauri::command]
pub async fn list_dir_files(path: String, limit: Option<usize>) -> AppResult<Vec<DirFile>> {
    let p = validate_dir(&path)?;
    let rd = std::fs::read_dir(&p).map_err(|e| AppError::io(e, &path))?;
    let mut v = Vec::new();
    for e in rd.flatten() {
        let Ok(md) = e.metadata() else { continue };
        if !md.is_file() {
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        let ext = crate::scan::walker::extension_of(&name);
        v.push(DirFile {
            category: crate::scan::classify::category_of(&ext, md.len()).into(),
            ext,
            path: e.path().to_string_lossy().into_owned(),
            name,
            size: md.len(),
            modified: md
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
        });
    }
    v.sort_by(|a, b| b.size.cmp(&a.size));
    v.truncate(limit.unwrap_or(300));
    Ok(v)
}

// ---------------------------------------------------------------------------
// Eski dosyalar
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn cold_start(app: AppHandle, st: St<'_>, root: String, days: u32, min_size: u64) -> AppResult<String> {
    let root = validate_dir(&root)?;
    if days == 0 || days > 36_500 {
        return Err(AppError::InvalidInput("gün sayısı 1 ile 36500 arasında olmalı".into()));
    }
    let ctl = st.jobs.start("cold")?;
    let id = ctl.id.clone();
    spawn_job(app, ctl, None, move |st, ctl| {
        let opts = st.walk_options();
        let r = crate::scan::cold_files(&root, &opts, days, min_size, ctl);
        let status = if r.cancelled { "cancelled" } else { "complete" }.to_string();
        let summary = serde_json::json!({ "count": r.total_matched, "size": r.total_size });
        *st.cold.write() = Some(Arc::new(r));
        (status, None, summary)
    });
    Ok(id)
}

#[tauri::command]
pub async fn cold_result(st: St<'_>) -> AppResult<Option<Arc<ColdReport>>> {
    Ok(st.cold.read().clone())
}

// ---------------------------------------------------------------------------
// Yinelenen dosyalar
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn dupes_start(app: AppHandle, st: St<'_>, root: String, min_size: u64) -> AppResult<String> {
    let root = validate_dir(&root)?;
    let ctl = st.jobs.start("dupes")?;
    let id = ctl.id.clone();
    let min_size = min_size.max(1);
    spawn_job(app, ctl, None, move |st, ctl| {
        let opts = st.walk_options();
        let r = crate::scan::dupes::find(&root, &opts, min_size, ctl);
        let status = if r.cancelled { "cancelled" } else { "complete" }.to_string();
        let summary = serde_json::json!({ "groups": r.groups.len(), "wasted": r.total_wasted });
        *st.dupes.write() = Some(Arc::new(r));
        (status, None, summary)
    });
    Ok(id)
}

#[tauri::command]
pub async fn dupes_result(st: St<'_>) -> AppResult<Option<Arc<DupReport>>> {
    Ok(st.dupes.read().clone())
}

// ---------------------------------------------------------------------------
// Seçilen dosyalar üzerinde güvenli işlemler
// ---------------------------------------------------------------------------

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileOpResult {
    pub succeeded: Vec<String>,
    pub failed: Vec<(String, String)>,
    pub bytes: u64,
}

/// Seçilen dosyaları karantinaya (`mode = "quarantine"`) veya Geri Dönüşüm
/// Kutusu'na (`mode = "recycle"`) taşır. Kalıcı silme bu yoldan yapılmaz.
#[tauri::command]
pub async fn files_remove(st: St<'_>, paths: Vec<String>, mode: String, source: String) -> AppResult<FileOpResult> {
    if paths.is_empty() {
        return Ok(FileOpResult::default());
    }
    if paths.len() > 10_000 {
        return Err(AppError::InvalidInput("tek seferde en fazla 10.000 dosya".into()));
    }
    let source = match source.as_str() {
        "large" | "duplicates" | "cold" | "explorer" => source,
        _ => return Err(AppError::InvalidInput("geçersiz kaynak".into())),
    };
    let st2 = st.inner().clone();
    let mode2 = mode.clone();
    let res = tauri::async_runtime::spawn_blocking(move || -> AppResult<FileOpResult> {
        let mut out = FileOpResult::default();
        let mut ok_paths = Vec::new();
        for p in &paths {
            let path = Path::new(p);
            if !path.is_absolute() {
                out.failed.push((p.clone(), "Geçersiz yol".into()));
                continue;
            }
            if crate::cleanup::rules::is_protected_path(p) {
                out.failed.push((p.clone(), "Korunan sistem konumu".into()));
                continue;
            }
            match std::fs::symlink_metadata(path) {
                Ok(m) if m.is_file() => ok_paths.push((p.clone(), m.len())),
                Ok(_) => out.failed.push((p.clone(), "Dosya değil".into())),
                Err(e) => out.failed.push((p.clone(), AppError::io(e, "dosya").to_string())),
            }
        }
        match mode2.as_str() {
            "quarantine" => {
                let days = st2.settings.read().quarantine_retention_days;
                let q = st2.quarantine();
                for (p, len) in ok_paths {
                    match q.add(Path::new(&p), &source, days) {
                        Ok(_) => {
                            out.bytes += len;
                            out.succeeded.push(p);
                        }
                        Err(e) => out.failed.push((p, e.to_string())),
                    }
                }
            }
            "recycle" => {
                // Her dosyayı ayrı taşı ki hangi dosyanın başarısız olduğu bilinsin.
                for (p, len) in ok_paths {
                    match crate::platform::move_to_recycle_bin(std::slice::from_ref(&p)) {
                        Ok(()) if !Path::new(&p).exists() => {
                            out.bytes += len;
                            out.succeeded.push(p);
                        }
                        Ok(()) => out.failed.push((p, "Dosya taşınamadı".into())),
                        Err(e) => out.failed.push((p, e.to_string())),
                    }
                }
            }
            _ => return Err(AppError::InvalidInput("geçersiz işlem türü".into())),
        }
        Ok(out)
    })
    .await
    .map_err(|e| AppError::System(e.to_string()))??;

    let outcome = match (res.succeeded.is_empty(), res.failed.is_empty()) {
        (false, true) => Outcome::Success,
        (false, false) => Outcome::Partial,
        _ => Outcome::Failed,
    };
    let title = if mode == "quarantine" {
        format!("{} dosya karantinaya taşındı", res.succeeded.len())
    } else {
        format!("{} dosya Geri Dönüşüm Kutusu'na taşındı", res.succeeded.len())
    };
    oplog::record(
        &st.db,
        "files",
        &title,
        outcome,
        true,
        serde_json::json!({
            "mode": mode,
            "bytes": res.bytes,
            "items": res.succeeded.iter().take(200).collect::<Vec<_>>(),
            "failed": res.failed.iter().take(50).collect::<Vec<_>>(),
            "count": res.succeeded.len(),
        }),
    )?;
    Ok(res)
}

/// Taramalardan hariç tutulacak bir yol ekler.
#[tauri::command]
pub async fn exclusion_add_path(st: St<'_>, path: String) -> AppResult<()> {
    let path = path.trim().to_string();
    if !Path::new(&path).is_absolute() {
        return Err(AppError::InvalidInput("tam bir yol gerekli".into()));
    }
    let mut s = st.settings();
    if !s.exclusions.paths.iter().any(|p| crate::paths::normalize(p) == crate::paths::normalize(&path)) {
        s.exclusions.paths.push(path);
    }
    let s = s.validate()?;
    crate::settings::save(&st.db, &s)?;
    *st.settings.write() = s;
    Ok(())
}
