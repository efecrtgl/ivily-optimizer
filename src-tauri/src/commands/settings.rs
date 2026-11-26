//! Ayarlar, gizlilik (dışa aktarma / silme) ve günlük komutları.

use crate::error::{AppError, AppResult};
use crate::settings::{self, Settings};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

type St<'a> = State<'a, Arc<AppState>>;

#[tauri::command]
pub async fn settings_get(st: St<'_>) -> AppResult<Settings> {
    Ok(st.settings())
}

#[tauri::command]
pub async fn settings_set(st: St<'_>, settings: Settings) -> AppResult<Settings> {
    let new = settings.validate()?;
    let old = st.settings();
    if new.launch_at_startup != old.launch_at_startup {
        crate::platform::set_launch_at_startup(new.launch_at_startup)?;
    }
    settings::save(&st.db, &new)?;
    *st.settings.write() = new.clone();
    tracing::info!("ayarlar kaydedildi");
    Ok(new)
}

#[tauri::command]
pub async fn settings_reset(st: St<'_>) -> AppResult<Settings> {
    let mut def = Settings::default();
    // İlk kullanım rehberini yeniden göstermeyelim.
    def.onboarding_done = true;
    if st.settings.read().launch_at_startup {
        crate::platform::set_launch_at_startup(false)?;
    }
    settings::save(&st.db, &def)?;
    *st.settings.write() = def.clone();
    Ok(def)
}

/// Yerel verileri (ayarlar, işlem geçmişi, anlık görüntü özetleri, karantina
/// kayıtları) okunabilir bir JSON dosyası olarak İndirilenler klasörüne
/// aktarır. Dosya içerikleri dahil edilmez.
#[tauri::command]
pub async fn data_export(st: St<'_>) -> AppResult<String> {
    let dir = dirs::download_dir()
        .or_else(dirs::desktop_dir)
        .ok_or_else(|| AppError::NotFound("İndirilenler klasörü".into()))?;
    let name = format!("ivily-veri-{}.json", chrono::Local::now().format("%Y%m%d-%H%M%S"));
    let path = dir.join(name);
    let data = serde_json::json!({
        "uygulama": "Ivily Optimizer",
        "surum": env!("CARGO_PKG_VERSION"),
        "olusturulma": chrono::Local::now().to_rfc3339(),
        "ayarlar": st.settings(),
        "islemGecmisi": crate::oplog::list(&st.db, None, 100_000)?,
        "anlikGoruntuler": crate::snapshots::list(&st.db, None)?,
        "karantina": st.quarantine().list(true)?,
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&data)?).map_err(|e| AppError::io(e, &path.to_string_lossy()))?;
    Ok(path.to_string_lossy().into_owned())
}

/// İşlem geçmişini ve anlık görüntüleri siler. Karantinadaki dosyalar
/// kullanıcı verisi olduğu için burada silinmez.
#[tauri::command]
pub async fn data_clear_history(st: St<'_>) -> AppResult<()> {
    st.db.with(|c| {
        c.execute_batch("DELETE FROM oplog; DELETE FROM snapshot_dirs; DELETE FROM snapshots; DELETE FROM kv WHERE key IN ('overview_cache','cleanup_last'); VACUUM;")
    })?;
    *st.disk_scan.write() = None;
    tracing::info!("geçmiş ve anlık görüntüler silindi");
    Ok(())
}

#[tauri::command]
pub async fn data_usage(st: St<'_>) -> AppResult<serde_json::Value> {
    let db_size = std::fs::metadata(crate::paths::db_path()).map(|m| m.len()).unwrap_or(0);
    let logs: u64 = std::fs::read_dir(crate::paths::logs_dir())
        .map(|rd| rd.flatten().filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum())
        .unwrap_or(0);
    let (qcount, qsize) = st.quarantine().total_size()?;
    let snaps = crate::snapshots::list(&st.db, None)?.len();
    Ok(serde_json::json!({
        "dbSize": db_size, "logsSize": logs, "quarantineCount": qcount, "quarantineSize": qsize, "snapshotCount": snaps,
        "dataDir": crate::paths::data_dir(),
    }))
}

#[tauri::command]
pub async fn logs_open() -> AppResult<()> {
    crate::platform::reveal_in_explorer(&crate::paths::logs_dir().to_string_lossy())
}

/// Günlükleri tek bir metin dosyası olarak İndirilenler klasörüne aktarır.
#[tauri::command]
pub async fn logs_export() -> AppResult<String> {
    let dir = dirs::download_dir()
        .or_else(dirs::desktop_dir)
        .ok_or_else(|| AppError::NotFound("İndirilenler klasörü".into()))?;
    let path = dir.join(format!("ivily-gunluk-{}.txt", chrono::Local::now().format("%Y%m%d-%H%M%S")));
    let mut files: Vec<_> = std::fs::read_dir(crate::paths::logs_dir())
        .map_err(|e| AppError::io(e, "günlük klasörü"))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    let mut out = String::from("Ivily Optimizer günlükleri\nBu dosya uygulama olaylarını, hataları ve sistem yönetimi işlemlerini içerir. Dosya içerikleri yer almaz.\n\n");
    for f in files.iter().rev().take(7).rev() {
        out.push_str(&format!("===== {} =====\n", f.file_name().unwrap_or_default().to_string_lossy()));
        out.push_str(&std::fs::read_to_string(f).unwrap_or_default());
        out.push('\n');
    }
    std::fs::write(&path, out).map_err(|e| AppError::io(e, &path.to_string_lossy()))?;
    Ok(path.to_string_lossy().into_owned())
}
