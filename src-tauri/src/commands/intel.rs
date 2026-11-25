//! Sistem zekâsı komutları: dijital imza, başlangıç ayrıntıları, süreç
//! ayrıntıları ve yeniden başlatma, görev geçmişi ve olay günlüğü.
//!
//! Tüm komutlar `spawn_blocking` ile arka planda çalışır.

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::sysmgmt::{events, procinfo, signature, startup, tasks};
use std::sync::Arc;
use tauri::State;

type St<'a> = State<'a, Arc<AppState>>;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> AppResult<T> + Send + 'static) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::System(format!("arka plan görevi tamamlanamadı: {e}")))?
}

// --- Dijital imza ----------------------------------------------------------

#[tauri::command]
pub async fn signature_check(paths: Vec<String>) -> AppResult<Vec<signature::SignatureResult>> {
    blocking(move || signature::check_many(paths)).await
}

// --- Başlangıç -------------------------------------------------------------

#[tauri::command]
pub async fn startup_extras() -> AppResult<startup::StartupExtras> {
    blocking(startup::extras).await
}

#[tauri::command]
pub async fn startup_boot_history() -> AppResult<startup::BootHistory> {
    blocking(startup::boot_history).await
}

// --- Süreçler --------------------------------------------------------------

#[tauri::command]
pub async fn process_details(pid: u32) -> AppResult<procinfo::ProcessDetails> {
    blocking(move || procinfo::details(pid)).await
}

#[tauri::command]
pub async fn process_restart(st: St<'_>, pid: u32, name: String) -> AppResult<procinfo::ProcessRestartResult> {
    let db = st.db.clone();
    blocking(move || procinfo::restart(&db, pid, &name)).await
}

// --- Görev geçmişi ---------------------------------------------------------

#[tauri::command]
pub async fn task_history(path: String, name: String) -> AppResult<tasks::TaskHistory> {
    blocking(move || tasks::history(&path, &name)).await
}

// --- Olay günlüğü ----------------------------------------------------------

#[tauri::command]
pub async fn events_query(days: u32, levels: Vec<u8>, logs: Vec<String>, max: u32) -> AppResult<events::EventsReport> {
    blocking(move || events::query(days, levels, logs, max)).await
}

#[tauri::command]
pub async fn events_context(ts: i64, window_minutes: u32) -> AppResult<events::EventsContext> {
    blocking(move || events::context(ts, window_minutes)).await
}

#[tauri::command]
pub async fn events_export(
    days: u32,
    levels: Vec<u8>,
    logs: Vec<String>,
    max: u32,
    format: String,
) -> AppResult<String> {
    blocking(move || events::export(days, levels, logs, max, &format)).await
}
