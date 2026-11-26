//! Dosya gezgini, boş klasörler, özet hesaplama, klasör karşılaştırma,
//! envanter ve depolama içgörüleri komutları.

use crate::error::{AppError, AppResult};
use crate::files::{
    self, CompareReport, DirListing, EmptyFolders, EmptyRemoveResult, FsMetadata, FsRoot, HashResult, InventoryResult,
    StorageInsights,
};
use crate::oplog::{self, Outcome};
use crate::state::AppState;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};

type St<'a> = State<'a, Arc<AppState>>;

static LAST_HASH: Mutex<Option<HashResult>> = Mutex::new(None);
static LAST_COMPARE: Mutex<Option<Arc<CompareReport>>> = Mutex::new(None);

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> AppResult<T> + Send + 'static) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::System(format!("arka plan görevi tamamlanamadı: {e}")))?
}

fn current_scan(st: &AppState) -> Option<Arc<crate::scan::tree::DiskScan>> {
    st.disk_scan.read().clone()
}

fn require_scan(st: &AppState) -> AppResult<Arc<crate::scan::tree::DiskScan>> {
    current_scan(st).ok_or_else(|| AppError::NotFound("henüz bir disk taraması yapılmadı".into()))
}

// --- Gezgin ----------------------------------------------------------------

#[tauri::command]
pub async fn fs_list(st: St<'_>, path: String, show_hidden: bool) -> AppResult<DirListing> {
    let scan = current_scan(&st);
    blocking(move || files::list_dir(&path, show_hidden, scan.as_deref())).await
}

#[tauri::command]
pub async fn fs_roots(st: St<'_>) -> AppResult<Vec<FsRoot>> {
    let disks = st.monitor.lock().disks();
    Ok(files::roots(&disks))
}

#[tauri::command]
pub async fn fs_metadata(st: St<'_>, path: String) -> AppResult<FsMetadata> {
    let scan = current_scan(&st);
    blocking(move || files::metadata(&path, scan.as_deref())).await
}

// --- Boş klasörler ---------------------------------------------------------

#[tauri::command]
pub async fn scan_empty_folders(st: St<'_>) -> AppResult<EmptyFolders> {
    let scan = require_scan(&st)?;
    blocking(move || files::find_empty_folders(&scan)).await
}

#[tauri::command]
pub async fn empty_folders_remove(st: St<'_>, paths: Vec<String>) -> AppResult<EmptyRemoveResult> {
    if paths.is_empty() {
        return Ok(EmptyRemoveResult::default());
    }
    if paths.len() > 10_000 {
        return Err(AppError::InvalidInput("tek seferde en fazla 10.000 klasör".into()));
    }
    let res = blocking(move || Ok(files::remove_empty_folders(&paths))).await?;
    let outcome = match (res.removed.is_empty(), res.failed.is_empty()) {
        (false, true) => Outcome::Success,
        (false, false) => Outcome::Partial,
        _ => Outcome::Failed,
    };
    oplog::record(
        &st.db,
        "folders",
        &format!("{} boş klasör silindi", res.removed.len()),
        outcome,
        true,
        serde_json::json!({
            "paths": res.all_removed.iter().take(5000).collect::<Vec<_>>(),
            "pathsTruncated": res.all_removed.len() > 5000,
            "count": res.removed.len(),
            "failed": res.failed.iter().take(50).collect::<Vec<_>>(),
        }),
    )?;
    Ok(res)
}

// --- Özet hesaplama --------------------------------------------------------

#[tauri::command]
pub async fn file_hash_start(app: AppHandle, st: St<'_>, path: String) -> AppResult<String> {
    let p = PathBuf::from(path.trim());
    if !p.is_absolute() || !p.is_file() {
        return Err(AppError::InvalidInput("geçerli bir dosya seçin".into()));
    }
    let ctl = st.jobs.start("hash")?;
    let id = ctl.id.clone();
    crate::commands::disk::spawn_job(app, ctl, None, move |_st, ctl| match files::hash_file(&p, ctl) {
        Ok(r) => {
            let summary = serde_json::to_value(&r).unwrap_or_default();
            *LAST_HASH.lock() = Some(r);
            ("complete".into(), None, summary)
        }
        Err(AppError::Cancelled) => ("cancelled".into(), None, serde_json::Value::Null),
        Err(e) => ("failed".into(), Some(e.to_string()), serde_json::Value::Null),
    });
    Ok(id)
}

#[tauri::command]
pub async fn file_hash_result() -> AppResult<Option<HashResult>> {
    Ok(LAST_HASH.lock().clone())
}

// --- Klasör karşılaştırma --------------------------------------------------

fn validate_dir(p: &str) -> AppResult<PathBuf> {
    let path = PathBuf::from(p.trim());
    if !path.is_absolute() {
        return Err(AppError::InvalidInput("tam bir klasör yolu girin (ör. C:\\Users)".into()));
    }
    if !path.is_dir() {
        return Err(AppError::NotFound(format!("{} klasörü bulunamadı", path.display())));
    }
    Ok(path)
}

#[tauri::command]
pub async fn folder_compare_start(
    app: AppHandle,
    st: St<'_>,
    a: String,
    b: String,
    content: bool,
) -> AppResult<String> {
    let a = validate_dir(&a)?;
    let b = validate_dir(&b)?;
    if files::same_dir(&a, &b) {
        return Err(AppError::InvalidInput("karşılaştırmak için iki farklı klasör seçin".into()));
    }
    let ctl = st.jobs.start("compare")?;
    let id = ctl.id.clone();
    crate::commands::disk::spawn_job(app, ctl, None, move |st, ctl| {
        let opts = st.walk_options();
        let r = files::compare_folders(&a, &b, content, &opts, ctl);
        let status = if r.cancelled { "cancelled" } else { "complete" }.to_string();
        let summary = serde_json::json!({
            "onlyA": r.only_a.len(),
            "onlyB": r.only_b.len(),
            "different": r.different.len(),
            "identical": r.identical,
            "errors": r.errors,
        });
        let msg = r.cancelled.then(|| "Karşılaştırma iptal edildi. Sonuçlar kısmidir.".to_string());
        *LAST_COMPARE.lock() = Some(Arc::new(r));
        (status, msg, summary)
    });
    Ok(id)
}

#[tauri::command]
pub async fn folder_compare_result() -> AppResult<Option<Arc<CompareReport>>> {
    Ok(LAST_COMPARE.lock().clone())
}

// --- Envanter --------------------------------------------------------------

#[tauri::command]
pub async fn inventory_export(st: St<'_>, path: String, recursive: bool) -> AppResult<InventoryResult> {
    let root = validate_dir(&path)?;
    let out = files::export_dir()?.join(format!("ivily-envanter-{}.csv", files::stamp()));
    let opts = st.walk_options();
    blocking(move || files::write_inventory(&root, recursive, opts, &out, files::INVENTORY_LIMIT)).await
}

// --- Depolama içgörüleri ---------------------------------------------------

#[tauri::command]
pub async fn scan_insights(st: St<'_>) -> AppResult<StorageInsights> {
    let scan = require_scan(&st)?;
    let root = scan.root.to_string_lossy().into_owned();
    let volume = st
        .monitor
        .lock()
        .disks()
        .into_iter()
        .filter(|d| crate::paths::is_within(&root, &d.mount))
        .max_by_key(|d| d.mount.len())
        .map(|d| (d.mount, d.total, d.available));
    let db = st.db.clone();
    blocking(move || {
        let growth = crate::snapshots::growth(&db, &root, 30, 10).ok();
        Ok(files::insights(files::InsightInputs {
            scan: &scan,
            volume,
            growth,
            user_folders: files::user_folders(),
            app_roots: files::app_roots(),
        }))
    })
    .await
}
