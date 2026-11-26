//! Temizlik, karantina, geçmiş ve anlık görüntü komutları.

use crate::cleanup::{self, CleanupReport, CleanupScanSummary, DeleteRemover, QuarantineRemover};
use crate::error::{AppError, AppResult};
use crate::jobs;
use crate::oplog::{self, Outcome};
use crate::quarantine::QuarantineItem;
use crate::snapshots::{self, Comparison, GrowthReport, SnapshotInfo};
use crate::state::AppState;
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

type St<'a> = State<'a, Arc<AppState>>;

// ---------------------------------------------------------------------------
// Temizlik
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn cleanup_scan_start(app: AppHandle, st: St<'_>) -> AppResult<String> {
    let ctl = st.jobs.start("cleanup-scan")?;
    let id = ctl.id.clone();
    std::thread::spawn(move || {
        let st = app.state::<Arc<AppState>>().inner().clone();
        let stop = jobs::spawn_reporter(app.clone(), ctl.clone(), None);
        let s = st.settings();
        let res = cleanup::scan(
            s.temp_min_age_hours,
            &s.user_rules,
            &s.exclusions.cleanup_categories,
            &s.exclusions,
            s.cleanup_use_quarantine,
            &ctl.cancel,
            &ctl.progress,
        );
        stop.store(true, Ordering::Relaxed);
        st.jobs.finish(&ctl.id);
        match res {
            Ok(scan) => {
                let sum = scan.summary();
                let summary =
                    serde_json::json!({ "total": sum.total_size, "safe": sum.safe_size, "items": sum.items.len() });
                let _ = st.db.kv_set("cleanup_last", &summary.to_string());
                *st.cleanup_scan.lock() = Some(Arc::new(scan));
                jobs::emit_done(&app, &ctl, "complete", None, summary);
            }
            Err(AppError::Cancelled) => jobs::emit_done(&app, &ctl, "cancelled", None, serde_json::Value::Null),
            Err(e) => jobs::emit_done(&app, &ctl, "failed", Some(e.to_string()), serde_json::Value::Null),
        }
    });
    Ok(id)
}

#[tauri::command]
pub async fn cleanup_scan_result(st: St<'_>) -> AppResult<Option<CleanupScanSummary>> {
    Ok(st.cleanup_scan.lock().as_ref().map(|s| s.summary()))
}

/// Bir temizlik kategorisinin aday dosyaları (en büyükler önce).
#[tauri::command]
pub async fn cleanup_item_files(st: St<'_>, id: String, limit: Option<usize>) -> AppResult<cleanup::CleanupItemFiles> {
    let scan =
        st.cleanup_scan.lock().clone().ok_or_else(|| AppError::NotFound("önce bir temizlik taraması yapın".into()))?;
    scan.item_files(&id, limit.unwrap_or(300).clamp(1, 20_000))
        .ok_or_else(|| AppError::NotFound("bu kategori son taramada bulunamadı".into()))
}

#[tauri::command]
pub async fn cleanup_last_summary(st: St<'_>) -> AppResult<Option<serde_json::Value>> {
    Ok(st.db.kv_get("cleanup_last")?.and_then(|s| serde_json::from_str(&s).ok()))
}

#[tauri::command]
pub async fn cleanup_run(app: AppHandle, st: St<'_>, ids: Vec<String>) -> AppResult<String> {
    let scan =
        st.cleanup_scan.lock().clone().ok_or_else(|| AppError::NotFound("önce bir temizlik taraması yapın".into()))?;
    if ids.is_empty() {
        return Err(AppError::InvalidInput("temizlenecek öğe seçilmedi".into()));
    }
    // Tarama sonuçları 30 dakikadan eskiyse yeniden taranmasını iste.
    if crate::db::now_ts() - scan.created_at > 30 * 60 {
        return Err(AppError::Conflict("tarama sonuçları eskidi; lütfen yeniden tarayın".into()));
    }
    let ctl = st.jobs.start("cleanup-run")?;
    let id = ctl.id.clone();
    std::thread::spawn(move || {
        let st = app.state::<Arc<AppState>>().inner().clone();
        let total: u64 = scan.items.iter().filter(|i| ids.contains(&i.id)).map(|i| i.files).sum();
        ctl.set_phase("Temizleniyor", total);
        let stop = jobs::spawn_reporter(app.clone(), ctl.clone(), None);
        let s = st.settings();
        let free_before = crate::platform::system_drive_free();
        let progress_ctl = ctl.clone();
        let on_progress = move |name: &str, n: u64| {
            progress_ctl.add_done(n);
            if let Some(mut c) = progress_ctl.progress.current.try_lock() {
                if *c != name {
                    *c = name.to_string();
                }
            }
        };
        // Kullanıcı kuralları her zaman karantinaya alınır; diğer öğeler
        // ayara göre karantinaya alınır veya kalıcı silinir.
        let is_user_rule =
            |id: &String| scan.items.iter().any(|i| &i.id == id && i.group == cleanup::rules::USER_RULE_GROUP);
        let (user_ids, other_ids): (Vec<String>, Vec<String>) = ids.iter().cloned().partition(is_user_rule);
        let mut quarantine_ids: Vec<String> = Vec::new();
        let mut report = CleanupReport::default();
        let mut used_quarantine = false;
        if !other_ids.is_empty() {
            if s.cleanup_use_quarantine {
                let rem = QuarantineRemover::new(st.quarantine(), s.quarantine_retention_days);
                report.merge(cleanup::execute(&scan, &other_ids, &rem, &ctl.cancel, &on_progress));
                quarantine_ids.extend(rem.ids.into_inner());
                used_quarantine = true;
            } else {
                report.merge(cleanup::execute(&scan, &other_ids, &DeleteRemover, &ctl.cancel, &on_progress));
            }
        }
        if !user_ids.is_empty() && !report.cancelled {
            let rem = QuarantineRemover::new(st.quarantine(), s.quarantine_retention_days);
            report.merge(cleanup::execute(&scan, &user_ids, &rem, &ctl.cancel, &on_progress));
            quarantine_ids.extend(rem.ids.into_inner());
            used_quarantine = true;
        }
        report.used_quarantine = used_quarantine;
        report.free_before = free_before;
        report.free_after = crate::platform::system_drive_free();
        stop.store(true, Ordering::Relaxed);
        st.jobs.finish(&ctl.id);
        // Temizlenen öğelerin listesi artık geçersiz.
        *st.cleanup_scan.lock() = None;

        let outcome = if report.cancelled {
            Outcome::Cancelled
        } else if report.total_files == 0 && report.total_problems > 0 {
            Outcome::Failed
        } else if report.total_problems > 0 {
            Outcome::Partial
        } else {
            Outcome::Success
        };
        let mut detail = serde_json::to_value(&report).unwrap_or_default();
        if report.used_quarantine {
            if let Some(obj) = detail.as_object_mut() {
                obj.insert("quarantineIdsTruncated".into(), (quarantine_ids.len() > 5000).into());
                quarantine_ids.truncate(5000);
                obj.insert("quarantineIds".into(), serde_json::json!(quarantine_ids));
            }
        }
        let _ = oplog::record(
            &st.db,
            "cleanup",
            &format!("Temizlik: {} öğe", report.items.len()),
            outcome,
            report.used_quarantine,
            detail,
        );
        let status = if report.cancelled { "cancelled" } else { "complete" };
        jobs::emit_done(&app, &ctl, status, None, serde_json::to_value(&report).unwrap_or_default());
    });
    Ok(id)
}

// ---------------------------------------------------------------------------
// Karantina
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuarantineOverview {
    pub items: Vec<QuarantineItem>,
    pub count: u64,
    pub size: u64,
    pub retention_days: u32,
    pub folder: String,
}

#[tauri::command]
pub async fn quarantine_list(st: St<'_>, history: bool) -> AppResult<QuarantineOverview> {
    let q = st.quarantine();
    let (count, size) = q.total_size()?;
    Ok(QuarantineOverview {
        items: q.list(history)?,
        count,
        size,
        retention_days: st.settings.read().quarantine_retention_days,
        folder: crate::paths::quarantine_dir().to_string_lossy().into_owned(),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuarantineRestoreResult {
    pub path: String,
    pub verified: bool,
    pub message: String,
}

#[tauri::command]
pub async fn quarantine_restore(
    st: St<'_>,
    id: String,
    rename_on_conflict: bool,
) -> AppResult<QuarantineRestoreResult> {
    let q = st.quarantine();
    let item = q.get(&id)?;
    let res = q.restore(&id, rename_on_conflict);
    if !matches!(res, Err(AppError::Conflict(_))) {
        oplog::record(
            &st.db,
            "quarantine",
            &format!("Geri yükleme: {}", file_name(&item.original_path)),
            if res.is_ok() { Outcome::Success } else { Outcome::Failed },
            false,
            serde_json::json!({
                "original": item.original_path,
                "restoredTo": res.as_ref().ok().map(|r| r.path.clone()),
                "verified": res.as_ref().ok().map(|r| r.verified),
                "error": res.as_ref().err().map(|e| e.to_string()),
            }),
        )?;
    }
    let out = res?;
    let message = if out.verified {
        "Dosya geri yüklendi ve hedefte doğrulandı.".to_string()
    } else {
        "Dosya geri yüklendi ancak hedefteki boyutu kayıtlı boyutla eşleşmiyor; dosyayı kontrol edin.".to_string()
    };
    Ok(QuarantineRestoreResult { path: out.path, verified: out.verified, message })
}

#[tauri::command]
pub async fn quarantine_purge(st: St<'_>, ids: Vec<String>) -> AppResult<u64> {
    let q = st.quarantine();
    let mut bytes = 0;
    let mut failed = Vec::new();
    for id in &ids {
        match q.purge(id) {
            Ok(b) => bytes += b,
            Err(e) => failed.push(e.to_string()),
        }
    }
    oplog::record(
        &st.db,
        "quarantine",
        &format!("Karantinadan kalıcı silme: {} öğe", ids.len() - failed.len()),
        if failed.is_empty() { Outcome::Success } else { Outcome::Partial },
        false,
        serde_json::json!({ "bytes": bytes, "failed": failed }),
    )?;
    Ok(bytes)
}

fn file_name(p: &str) -> String {
    std::path::Path::new(p).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.to_string())
}

// ---------------------------------------------------------------------------
// Geçmiş ve anlık görüntüler
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn snapshots_list(st: St<'_>, root: Option<String>) -> AppResult<Vec<SnapshotInfo>> {
    snapshots::list(&st.db, root.as_deref())
}

#[tauri::command]
pub async fn snapshot_delete(st: St<'_>, id: i64) -> AppResult<()> {
    snapshots::delete(&st.db, id)
}

#[tauri::command]
pub async fn snapshot_compare(st: St<'_>, a: i64, b: i64) -> AppResult<Comparison> {
    let mut c = snapshots::compare(&st.db, a, b)?;
    c.changes.truncate(500);
    Ok(c)
}

#[tauri::command]
pub async fn growth_report(st: St<'_>, root: String, days: u32) -> AppResult<GrowthReport> {
    snapshots::growth(&st.db, &root, days.clamp(1, 3650), 50)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryInfo {
    pub id: String,
    pub name: String,
    pub group: String,
    pub risk: cleanup::rules::Risk,
}

/// Bu bilgisayarda bulunan temizlik kategorileri (ayarlarda hariç tutmak için).
#[tauri::command]
pub async fn cleanup_categories(st: St<'_>) -> AppResult<Vec<CategoryInfo>> {
    let s = st.settings();
    Ok(cleanup::rules::categories(s.temp_min_age_hours, &s.user_rules)
        .into_iter()
        .map(|c| CategoryInfo { id: c.id, name: c.name, group: c.group.to_string(), risk: c.risk })
        .collect())
}
