//! Sistem bilgisi, canlı izleme ve süreç komutları.

use crate::error::{AppError, AppResult};
use crate::monitor::{DiskInfo, LiveStats, ProcessInfo, SystemInfo};
use crate::oplog::{self, Outcome};
use crate::state::AppState;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

type St<'a> = State<'a, Arc<AppState>>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedOverview {
    pub info: Option<SystemInfo>,
    pub saved_at: Option<i64>,
}

const CACHE_KEY: &str = "overview_cache";

/// Uygulama açılırken anında gösterilecek son bilinen sistem bilgisi.
#[tauri::command]
pub async fn get_cached_overview(st: St<'_>) -> AppResult<CachedOverview> {
    let raw = st.db.kv_get(CACHE_KEY)?;
    let parsed: Option<(i64, serde_json::Value)> = raw.and_then(|r| serde_json::from_str(&r).ok());
    Ok(match parsed {
        Some((ts, v)) => CachedOverview { info: serde_json::from_value(v).ok(), saved_at: Some(ts) },
        None => CachedOverview { info: None, saved_at: None },
    })
}

#[tauri::command]
pub async fn get_system_info(st: St<'_>) -> AppResult<SystemInfo> {
    let st2 = st.inner().clone();
    let info = tauri::async_runtime::spawn_blocking(move || st2.monitor.lock().system_info())
        .await
        .map_err(|e| AppError::System(e.to_string()))?;
    if let Ok(v) = serde_json::to_value(&info) {
        let _ = st.db.kv_set(CACHE_KEY, &serde_json::to_string(&(crate::db::now_ts(), v)).unwrap_or_default());
    }
    Ok(info)
}

#[tauri::command]
pub async fn get_live_stats(st: St<'_>) -> AppResult<LiveStats> {
    Ok(st.monitor.lock().live())
}

#[tauri::command]
pub async fn get_disks(st: St<'_>) -> AppResult<Vec<DiskInfo>> {
    Ok(st.monitor.lock().disks())
}

#[tauri::command]
pub async fn list_processes(st: St<'_>) -> AppResult<Vec<ProcessInfo>> {
    let st2 = st.inner().clone();
    tauri::async_runtime::spawn_blocking(move || st2.monitor.lock().processes())
        .await
        .map_err(|e| AppError::System(e.to_string()))
}

#[tauri::command]
pub async fn kill_process(st: St<'_>, pid: u32, name: String, confirm_sensitive: bool) -> AppResult<()> {
    let res = st.monitor.lock().kill(pid, &name, confirm_sensitive);
    let outcome = if res.is_ok() { Outcome::Success } else { Outcome::Failed };
    if !matches!(res, Err(AppError::Protected(_)) | Err(AppError::Conflict(_))) {
        oplog::record(
            &st.db,
            "process",
            &format!("Süreç sonlandırıldı: {name}"),
            outcome,
            false,
            serde_json::json!({ "pid": pid, "name": name, "error": res.as_ref().err().map(|e| e.to_string()) }),
        )?;
    }
    res
}

#[tauri::command]
pub async fn reveal_path(path: String) -> AppResult<()> {
    crate::platform::reveal_in_explorer(&path)
}

#[tauri::command]
pub async fn open_url(url: String) -> AppResult<()> {
    crate::platform::open_url(&url)
}

#[tauri::command]
pub async fn get_app_paths() -> AppResult<serde_json::Value> {
    Ok(serde_json::json!({
        "data": crate::paths::data_dir(),
        "logs": crate::paths::logs_dir(),
        "quarantine": crate::paths::quarantine_dir(),
        "db": crate::paths::db_path(),
    }))
}
