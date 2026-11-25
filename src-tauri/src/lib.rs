//! Ivily Optimizer — Windows için sistem izleme, disk zekâsı ve bakım aracı.
//!
//! Geliştirici: efecrtgl — https://github.com/efecrtgl/ivily-optimizer

pub mod cleanup;
pub mod commands;
pub mod db;
pub mod error;
pub mod jobs;
pub mod monitor;
pub mod oplog;
pub mod paths;
pub mod platform;
pub mod quarantine;
pub mod scan;
pub mod settings;
pub mod snapshots;
pub mod state;
pub mod sysmgmt;

use state::AppState;
use std::sync::Arc;
use tauri::Manager;

fn init_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::{fmt, EnvFilter};
    let dir = paths::logs_dir();
    std::fs::create_dir_all(&dir).ok()?;
    prune_old_logs(&dir, 14);
    let appender = tracing_appender::rolling::daily(&dir, "ivily.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_env("IVILY_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).with_writer(writer).with_ansi(false).with_target(false).init();
    Some(guard)
}

fn prune_old_logs(dir: &std::path::Path, keep_days: u64) {
    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(keep_days * 86_400);
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                if m < cutoff {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
    }
}

/// Başlangıçta yapılan hafif bakım: süresi dolan karantina öğeleri, eski
/// geçmiş kayıtları ve anlık görüntü saklama politikası.
fn startup_maintenance(st: &AppState) {
    let s = st.settings();
    match st.quarantine().purge_expired() {
        Ok((n, b)) if n > 0 => tracing::info!(count = n, bytes = b, "süresi dolan karantina öğeleri silindi"),
        Err(e) => tracing::warn!(error = %e, "karantina bakımı başarısız"),
        _ => {}
    }
    let _ = oplog::prune(&st.db, s.history_retention_days as i64);
    let _ = snapshots::prune(&st.db, s.snapshot_retention_days, s.snapshot_max_count);
}

pub fn run() {
    let _guard = init_logging();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Ivily Optimizer başlatılıyor");

    if let Err(e) = paths::ensure_dirs() {
        tracing::error!(error = %e, "veri klasörleri oluşturulamadı");
    }
    let db = match db::Db::open(&paths::db_path()) {
        Ok(db) => db,
        Err(e) => {
            // Bozuk veritabanı: yedekleyip yeniden oluştur, uygulama yine açılsın.
            tracing::error!(error = %e, "veritabanı açılamadı, yedeklenip yeniden oluşturuluyor");
            let bak = paths::db_path().with_extension(format!("bozuk-{}.db", chrono::Utc::now().timestamp()));
            let _ = std::fs::rename(paths::db_path(), bak);
            db::Db::open(&paths::db_path()).unwrap_or_else(|_| db::Db::open_in_memory().expect("bellek içi veritabanı"))
        }
    };
    let state = Arc::new(AppState::new(db));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state.clone())
        .setup(move |app| {
            let st = state.clone();
            std::thread::spawn(move || startup_maintenance(&st));
            if std::env::args().any(|a| a == "--minimized") {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.minimize();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_cached_overview,
            commands::system::get_system_info,
            commands::system::get_live_stats,
            commands::system::get_disks,
            commands::system::list_processes,
            commands::system::kill_process,
            commands::system::reveal_path,
            commands::system::open_url,
            commands::system::get_app_paths,
            commands::disk::scan_start,
            commands::disk::job_cancel,
            commands::disk::jobs_running,
            commands::disk::scan_summary,
            commands::disk::scan_node,
            commands::disk::scan_find,
            commands::disk::scan_large_files,
            commands::disk::scan_small_files,
            commands::disk::list_dir_files,
            commands::disk::cold_start,
            commands::disk::cold_result,
            commands::disk::dupes_start,
            commands::disk::dupes_result,
            commands::disk::files_remove,
            commands::disk::exclusion_add_path,
            commands::maintenance::cleanup_scan_start,
            commands::maintenance::cleanup_scan_result,
            commands::maintenance::cleanup_last_summary,
            commands::maintenance::cleanup_run,
            commands::maintenance::quarantine_list,
            commands::maintenance::quarantine_restore,
            commands::maintenance::quarantine_purge,
            commands::maintenance::oplog_list,
            commands::maintenance::snapshots_list,
            commands::maintenance::snapshot_delete,
            commands::maintenance::snapshot_compare,
            commands::maintenance::growth_report,
            commands::settings::settings_get,
            commands::settings::settings_set,
            commands::settings::settings_reset,
            commands::settings::data_export,
            commands::settings::data_clear_history,
            commands::settings::data_usage,
            commands::settings::logs_open,
            commands::settings::logs_export,
            commands::sysmgmt::startup_list,
            commands::sysmgmt::startup_set_enabled,
            commands::sysmgmt::services_list,
            commands::sysmgmt::service_set_start_type,
            commands::sysmgmt::service_control,
            commands::sysmgmt::tasks_list,
            commands::sysmgmt::task_set_enabled,
            commands::sysmgmt::power_list,
            commands::sysmgmt::power_set_active,
            commands::sysmgmt::network_adapters,
            commands::sysmgmt::network_connectivity_check,
            commands::sysmgmt::network_dns_lookup,
            commands::sysmgmt::dns_cache_list,
            commands::sysmgmt::dns_cache_flush,
            commands::sysmgmt::update_check,
            commands::sysmgmt::update_last_check,
            commands::update::update_install,
            commands::update::update_cancel,
            commands::maintenance::cleanup_categories,
        ])
        .run(tauri::generate_context!())
        .expect("Ivily Optimizer başlatılamadı");
}
