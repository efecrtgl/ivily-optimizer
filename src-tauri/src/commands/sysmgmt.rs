//! Sistem yönetimi komutları: başlangıç, hizmetler, zamanlanmış görevler,
//! güç planları, ağ ve güncelleme denetimi.
//!
//! Yavaş olabilecek tüm işlemler (PowerShell, SCM, UAC beklemesi, ağ)
//! `spawn_blocking` ile arka planda çalıştırılır.

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::sysmgmt::{network, power, services, startup, tasks, update, ChangeResult};
use std::sync::Arc;
use tauri::State;

type St<'a> = State<'a, Arc<AppState>>;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> AppResult<T> + Send + 'static) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::System(format!("arka plan görevi tamamlanamadı: {e}")))?
}

// --- Başlangıç -------------------------------------------------------------

#[tauri::command]
pub async fn startup_list() -> AppResult<Vec<startup::StartupItem>> {
    blocking(|| Ok(startup::list())).await
}

#[tauri::command]
pub async fn startup_set_enabled(
    st: St<'_>,
    id: String,
    enabled: bool,
) -> AppResult<ChangeResult<startup::StartupItem>> {
    let db = st.db.clone();
    blocking(move || startup::set_enabled(&db, &id, enabled)).await
}

// --- Hizmetler -------------------------------------------------------------

#[tauri::command]
pub async fn services_list() -> AppResult<Vec<services::ServiceInfo>> {
    blocking(services::list).await
}

#[tauri::command]
pub async fn service_set_start_type(
    st: St<'_>,
    name: String,
    start_type: String,
) -> AppResult<ChangeResult<services::ServiceInfo>> {
    let db = st.db.clone();
    blocking(move || services::set_start_type(&db, &name, &start_type)).await
}

#[tauri::command]
pub async fn service_control(
    st: St<'_>,
    name: String,
    action: String,
) -> AppResult<ChangeResult<services::ServiceInfo>> {
    let db = st.db.clone();
    blocking(move || services::control(&db, &name, &action)).await
}

// --- Zamanlanmış görevler --------------------------------------------------

#[tauri::command]
pub async fn tasks_list() -> AppResult<Vec<tasks::TaskInfo>> {
    blocking(tasks::list).await
}

#[tauri::command]
pub async fn task_set_enabled(
    st: St<'_>,
    path: String,
    name: String,
    enabled: bool,
) -> AppResult<ChangeResult<tasks::TaskInfo>> {
    let db = st.db.clone();
    blocking(move || tasks::set_enabled(&db, &path, &name, enabled)).await
}

// --- Güç -------------------------------------------------------------------

#[tauri::command]
pub async fn power_list() -> AppResult<power::PowerOverview> {
    blocking(power::list).await
}

#[tauri::command]
pub async fn power_set_active(st: St<'_>, guid: String) -> AppResult<ChangeResult<power::PowerPlan>> {
    let db = st.db.clone();
    blocking(move || power::set_active(&db, &guid)).await
}

// --- Ağ --------------------------------------------------------------------

#[tauri::command]
pub async fn network_adapters(include_loopback: Option<bool>) -> AppResult<Vec<network::NetAdapter>> {
    blocking(move || network::adapters(include_loopback.unwrap_or(false))).await
}

#[tauri::command]
pub async fn network_connectivity_check() -> AppResult<network::ConnectivityReport> {
    blocking(|| Ok(network::connectivity_check())).await
}

#[tauri::command]
pub async fn network_dns_lookup(host: String) -> AppResult<network::DnsCheck> {
    blocking(move || network::dns_lookup(&host)).await
}

#[tauri::command]
pub async fn dns_cache_list() -> AppResult<network::DnsCacheList> {
    blocking(network::dns_cache_list).await
}

#[tauri::command]
pub async fn dns_cache_flush(st: St<'_>) -> AppResult<String> {
    let db = st.db.clone();
    blocking(move || network::dns_cache_flush(&db)).await
}

// --- Güncelleme ------------------------------------------------------------

#[tauri::command]
pub async fn update_check(st: St<'_>, include_prerelease: bool) -> AppResult<update::UpdateCheckResult> {
    let db = st.db.clone();
    blocking(move || update::check(&db, include_prerelease)).await
}

#[tauri::command]
pub async fn update_last_check(st: St<'_>) -> AppResult<Option<update::UpdateCheckResult>> {
    update::last_check(&st.db)
}
