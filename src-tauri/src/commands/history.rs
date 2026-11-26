//! İşlem geçmişi komutları: filtreli liste, geri alma ve dışa aktarma.

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::oplog::{self, OpEntry, OpFilter, Outcome};
use crate::quarantine::Quarantine;
use crate::state::AppState;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;

type St<'a> = State<'a, Arc<AppState>>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpPage {
    pub items: Vec<OpEntry>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub ok: bool,
    pub message: String,
    pub restore_op_id: Option<i64>,
    pub details: Vec<String>,
}

#[tauri::command]
pub async fn oplog_list(
    st: St<'_>,
    kind: Option<String>,
    query: Option<String>,
    outcome: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> AppResult<OpPage> {
    let f = OpFilter {
        kind,
        query,
        outcome,
        limit: limit.unwrap_or(100).clamp(1, 5000),
        offset: offset.unwrap_or(0).max(0),
    };
    let (items, total) = oplog::query(&st.db, &f)?;
    Ok(OpPage { items, total })
}

#[tauri::command]
pub async fn oplog_restore(st: St<'_>, id: i64) -> AppResult<RestoreResult> {
    let db = st.db.clone();
    let qroot = crate::paths::quarantine_dir();
    tauri::async_runtime::spawn_blocking(move || restore_entry(&db, &qroot, id))
        .await
        .map_err(|e| AppError::System(format!("arka plan görevi tamamlanamadı: {e}")))?
}

#[tauri::command]
pub async fn oplog_export(st: St<'_>, format: String) -> AppResult<String> {
    let dir = crate::files::export_dir()?;
    let db = st.db.clone();
    tauri::async_runtime::spawn_blocking(move || export_to(&db, &format, &dir))
        .await
        .map_err(|e| AppError::System(format!("arka plan görevi tamamlanamadı: {e}")))?
}

/// Tüm geçmişi `dir` içine CSV veya JSON olarak yazar; dosya yolunu döndürür.
pub fn export_to(db: &Db, format: &str, dir: &Path) -> AppResult<String> {
    let ext = match format {
        "csv" => "csv",
        "json" => "json",
        _ => return Err(AppError::InvalidInput("dışa aktarma biçimi csv veya json olmalı".into())),
    };
    let (items, _) = oplog::query(db, &OpFilter { limit: 1_000_000, ..Default::default() })?;
    let body = if ext == "csv" { oplog::to_csv(&items) } else { serde_json::to_string_pretty(&items)? };
    let path = dir.join(format!("ivily-gecmis-{}.{ext}", crate::files::stamp()));
    std::fs::write(&path, body).map_err(|e| AppError::io(e, &path.to_string_lossy()))?;
    tracing::info!(path = %path.display(), rows = items.len(), "işlem geçmişi dışa aktarıldı");
    Ok(path.to_string_lossy().into_owned())
}

// ---------------------------------------------------------------------------
// Geri alma
// ---------------------------------------------------------------------------

fn str_of<'a>(d: &'a Value, k: &str) -> &'a str {
    d.get(k).and_then(Value::as_str).unwrap_or("")
}

fn strings_of(d: &Value, k: &str) -> Vec<String> {
    d.get(k)
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn restored_title(e: &OpEntry) -> String {
    format!("Geri alındı: {}", e.title)
}

/// Bir geçmiş kaydını geri alır. Geri alma işleminin kendisi yeni bir kayıt
/// olarak yazılır; özgün kayıt "restored" veya "failed" olarak işaretlenir.
pub fn restore_entry(db: &Db, qroot: &Path, id: i64) -> AppResult<RestoreResult> {
    let e = oplog::get(db, id)?.ok_or_else(|| AppError::NotFound("geçmiş kaydı".into()))?;
    if e.restore_status != "available" {
        return Err(AppError::Conflict(match e.restore_status.as_str() {
            "restored" => "bu işlem zaten geri alındı".into(),
            "failed" => {
                "bu işlemin geri alınması daha önce başarısız oldu; değişikliği ilgili bölümden elle yapabilirsiniz"
                    .into()
            }
            _ => "bu işlem geri alınamaz".into(),
        }));
    }
    match e.kind.as_str() {
        "startup" | "task" | "power" | "service" => restore_system(db, &e),
        "files" | "cleanup" => restore_quarantined(db, qroot, &e),
        "folders" => restore_folders(db, &e),
        _ => Err(AppError::Unsupported("bu işlem türü geri alınamaz".into())),
    }
}

fn restore_system(db: &Db, e: &OpEntry) -> AppResult<RestoreResult> {
    use crate::sysmgmt::{power, services, startup, tasks};
    let d = &e.detail;
    let prev_bool = d.get("previousEnabled").and_then(Value::as_bool).unwrap_or(false);
    // Her çekirdek işlev değişikliği kendi kaydıyla geçmişe yazar.
    let res: AppResult<(String, Option<i64>)> = match e.kind.as_str() {
        "startup" => startup::set_enabled(db, str_of(d, "id"), prev_bool).map(|r| (r.message, r.oplog_id)),
        "task" => {
            tasks::set_enabled(db, str_of(d, "path"), str_of(d, "name"), prev_bool).map(|r| (r.message, r.oplog_id))
        }
        "power" => power::set_active(db, str_of(d, "previousGuid")).map(|r| (r.message, r.oplog_id)),
        "service" => services::set_start_type(db, str_of(d, "name"), str_of(d, "previousStartType"))
            .map(|r| (r.message, r.oplog_id)),
        _ => unreachable!(),
    };
    match res {
        Ok((message, op)) => {
            let op_id = match op {
                Some(op) => {
                    oplog::annotate(db, op, &restored_title(e), &[("restoredFrom", json!(e.id))])?;
                    op
                }
                None => oplog::record(
                    db,
                    &e.kind,
                    &restored_title(e),
                    Outcome::Success,
                    false,
                    json!({ "restoredFrom": e.id, "note": "Öğe zaten önceki durumundaydı; değişiklik gerekmedi." }),
                )?,
            };
            oplog::mark_restore(db, e.id, "restored", Some(op_id), None)?;
            Ok(RestoreResult { ok: true, message, restore_op_id: Some(op_id), details: vec![] })
        }
        Err(AppError::Cancelled) => Ok(RestoreResult {
            ok: false,
            message: "Yönetici onayı verilmediği için geri alma yapılmadı. İsterseniz yeniden deneyebilirsiniz.".into(),
            restore_op_id: None,
            details: vec![],
        }),
        Err(err) => {
            let msg = err.to_string();
            let op = oplog::record(
                db,
                &e.kind,
                &format!("Geri alma başarısız: {}", e.title),
                Outcome::Failed,
                false,
                json!({ "restoredFrom": e.id, "error": msg }),
            )?;
            oplog::mark_restore(db, e.id, "failed", Some(op), Some(&msg))?;
            Ok(RestoreResult {
                ok: false,
                message: format!("Geri alınamadı: {msg}"),
                restore_op_id: Some(op),
                details: vec![],
            })
        }
    }
}

fn restore_quarantined(db: &Db, qroot: &Path, e: &OpEntry) -> AppResult<RestoreResult> {
    let q = Quarantine::new(db, qroot.to_path_buf());
    let ids = strings_of(&e.detail, "quarantineIds");
    let mut details = Vec::new();
    let mut restored = Vec::new();
    let mut problems = 0usize;
    let mut skipped = 0usize;
    for qid in &ids {
        let item = match q.get(qid) {
            Ok(i) => i,
            Err(_) => {
                skipped += 1;
                details.push(format!("Karantina kaydı bulunamadı ({qid})."));
                continue;
            }
        };
        if item.state != "quarantined" {
            skipped += 1;
            let why = match item.state.as_str() {
                "restored" => "daha önce geri yüklenmiş",
                "deleted" => "karantinadan kalıcı olarak silinmiş",
                _ => "artık karantinada değil",
            };
            details.push(format!("{}: {why}.", item.original_path));
            continue;
        }
        match q.restore(qid, false) {
            Ok(out) => {
                if !out.verified {
                    details.push(format!("{}: geri yüklendi ancak boyutu doğrulanamadı.", out.path));
                }
                restored.push(out.path);
            }
            Err(AppError::Conflict(_)) => {
                problems += 1;
                details.push(format!(
                    "{}: bu konumda artık başka bir dosya var; üzerine yazılmadı. Dosya karantinada kaldı.",
                    item.original_path
                ));
            }
            Err(err) => {
                problems += 1;
                details.push(format!("{}: {err}", item.original_path));
            }
        }
    }
    finish_bulk(db, e, restored, problems, skipped, details, "dosya geri yüklendi")
}

fn restore_folders(db: &Db, e: &OpEntry) -> AppResult<RestoreResult> {
    let mut paths: Vec<String> = strings_of(&e.detail, "paths");
    paths.sort_by_key(|p| p.len());
    let mut details = Vec::new();
    let mut restored = Vec::new();
    let mut problems = 0usize;
    let mut skipped = 0usize;
    for p in paths {
        let path = PathBuf::from(&p);
        if !path.is_absolute() || crate::cleanup::rules::is_protected_resolved(&p) {
            problems += 1;
            details.push(format!("{p}: korunan veya geçersiz konum; oluşturulmadı."));
            continue;
        }
        if path.exists() {
            skipped += 1;
            continue;
        }
        match std::fs::create_dir_all(&path) {
            Ok(()) => restored.push(p),
            Err(err) => {
                problems += 1;
                details.push(AppError::io(err, &p).to_string());
            }
        }
    }
    if skipped > 0 {
        details.push(format!("{skipped} klasör zaten mevcuttu."));
    }
    finish_bulk(db, e, restored, problems, 0, details, "klasör yeniden oluşturuldu")
}

fn finish_bulk(
    db: &Db,
    e: &OpEntry,
    restored: Vec<String>,
    problems: usize,
    skipped: usize,
    details: Vec<String>,
    what: &str,
) -> AppResult<RestoreResult> {
    let n = restored.len();
    let outcome = match (n > 0, problems > 0) {
        (true, false) => Outcome::Success,
        (true, true) => Outcome::Partial,
        (false, true) => Outcome::Failed,
        // Geri alınacak bir şey kalmamıştı (hepsi zaten yerinde).
        (false, false) => Outcome::Success,
    };
    let title =
        if outcome == Outcome::Failed { format!("Geri alma başarısız: {}", e.title) } else { restored_title(e) };
    let op = oplog::record(
        db,
        &e.kind,
        &title,
        outcome,
        false,
        json!({
            "restoredFrom": e.id,
            "restored": restored.iter().take(500).collect::<Vec<_>>(),
            "count": n,
            "problems": problems,
            "skipped": skipped,
            "notes": details.iter().take(100).collect::<Vec<_>>(),
        }),
    )?;
    let mut message = if n > 0 { format!("{n} {what}.") } else { "Geri yüklenecek öğe kalmamıştı.".to_string() };
    if problems > 0 {
        message.push_str(&format!(" {problems} öğe geri alınamadı; ayrıntılara bakın."));
    }
    if outcome == Outcome::Failed {
        let err = format!("{problems} öğe geri alınamadı");
        oplog::mark_restore(db, e.id, "failed", Some(op), Some(&err))?;
        Ok(RestoreResult { ok: false, message, restore_op_id: Some(op), details })
    } else {
        let err = (problems > 0).then(|| format!("{problems} öğe geri alınamadı"));
        oplog::mark_restore(db, e.id, "restored", Some(op), err.as_deref())?;
        Ok(RestoreResult { ok: true, message, restore_op_id: Some(op), details })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn restores_quarantine_backed_files_entry() {
        let db = Db::open_in_memory().unwrap();
        let data = tempfile::tempdir().unwrap();
        let qdir = tempfile::tempdir().unwrap();
        let q = Quarantine::new(&db, qdir.path().to_path_buf());
        let a = data.path().join("a.txt");
        let b = data.path().join("b.txt");
        fs::write(&a, b"aaa").unwrap();
        fs::write(&b, b"bb").unwrap();
        let ia = q.add(&a, "large", 30).unwrap();
        let ib = q.add(&b, "large", 30).unwrap();
        // b'nin yerine yeni bir dosya gelmiş: üzerine yazılmamalı.
        fs::write(&b, b"yeni").unwrap();
        let id = oplog::record(
            &db,
            "files",
            "2 dosya karantinaya taşındı",
            Outcome::Success,
            true,
            json!({"mode":"quarantine","items":[a, b],"quarantineIds":[ia.id, ib.id],"count":2}),
        )
        .unwrap();
        assert_eq!(oplog::get(&db, id).unwrap().unwrap().restore_status, "available");
        let r = restore_entry(&db, qdir.path(), id).unwrap();
        assert!(r.ok);
        assert_eq!(fs::read(&a).unwrap(), b"aaa");
        assert_eq!(fs::read(&b).unwrap(), b"yeni");
        assert_eq!(r.details.len(), 1, "çakışma ayrıntılarda raporlanmalı");
        assert_eq!(q.get(&ib.id).unwrap().state, "quarantined");
        let orig = oplog::get(&db, id).unwrap().unwrap();
        assert_eq!(orig.restore_status, "restored");
        assert_eq!(orig.restore_op, r.restore_op_id);
        assert!(orig.restored_at.is_some());
        let op = oplog::get(&db, r.restore_op_id.unwrap()).unwrap().unwrap();
        assert_eq!(op.kind, "files");
        assert!(op.title.starts_with("Geri alındı: "));
        assert_eq!(op.outcome, "partial");
        assert_eq!(op.restore_status, "none");
        // İkinci kez geri alınamaz.
        assert!(matches!(restore_entry(&db, qdir.path(), id), Err(AppError::Conflict(_))));
    }

    #[test]
    fn restores_empty_folder_entry() {
        let db = Db::open_in_memory().unwrap();
        let t = tempfile::tempdir().unwrap();
        fs::create_dir_all(t.path().join("x/y/z")).unwrap();
        fs::create_dir_all(t.path().join("w")).unwrap();
        let paths =
            vec![t.path().join("x").to_string_lossy().into_owned(), t.path().join("w").to_string_lossy().into_owned()];
        let r = crate::files::remove_empty_folders(&paths);
        assert_eq!(r.removed.len(), 2);
        assert_eq!(r.all_removed.len(), 4);
        assert!(!t.path().join("x").exists());
        let id = oplog::record(
            &db,
            "folders",
            "2 boş klasör silindi",
            Outcome::Success,
            true,
            json!({ "paths": r.all_removed }),
        )
        .unwrap();
        let res = restore_entry(&db, Path::new("unused"), id).unwrap();
        assert!(res.ok, "{res:?}");
        assert!(t.path().join("x/y/z").is_dir());
        assert!(t.path().join("w").is_dir());
        assert_eq!(oplog::get(&db, id).unwrap().unwrap().restore_status, "restored");
    }

    #[test]
    fn refuses_non_restorable_entries() {
        let db = Db::open_in_memory().unwrap();
        let id = oplog::record(&db, "process", "Süreç sonlandırıldı", Outcome::Success, false, json!({})).unwrap();
        assert!(matches!(restore_entry(&db, Path::new("x"), id), Err(AppError::Conflict(_))));
        assert!(matches!(restore_entry(&db, Path::new("x"), 999), Err(AppError::NotFound(_))));
    }

    #[test]
    fn exports_csv_and_json() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        oplog::record(&db, "cleanup", "Temizlik: 2 öğe", Outcome::Success, false, json!({})).unwrap();
        let p = export_to(&db, "csv", dir.path()).unwrap();
        assert!(p.contains("ivily-gecmis-") && p.ends_with(".csv"));
        let text = fs::read_to_string(&p).unwrap();
        assert!(text.starts_with('\u{feff}'));
        assert!(text.contains(";Temizlik;Temizlik: 2 öğe;"));
        let p = export_to(&db, "json", dir.path()).unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(v[0]["restoreStatus"], "none");
        assert!(export_to(&db, "xml", dir.path()).is_err());
    }
}
