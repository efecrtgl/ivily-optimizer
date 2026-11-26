//! Güncelleme indirme, doğrulama ve kurulum.
//!
//! - Yalnızca bu deponun GitHub Releases adreslerinden bildirim dosyası
//!   (`latest.json`) kabul edilir.
//! - Yükleyici, uygulamaya **gömülü** minisign genel anahtarıyla doğrulanır
//!   (`tauri-plugin-updater`, `Update::download` içinde). Anahtar indirilen
//!   yerden alınmaz; imzası tutmayan paket asla kurulmaz.
//! - Windows'ta çalışan uygulama kendi dosyasını değiştiremez; eklenti,
//!   doğrulanmış NSIS yükleyicisini başlatır ve uygulamayı kapatır. Yükleyici
//!   başarısız olursa mevcut kurulum yerinde kalır.

use crate::error::{AppError, AppResult};
use crate::oplog::{self, Outcome};
use crate::state::AppState;
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::UpdaterExt;

const ALLOWED_PREFIX: &str = "https://github.com/efecrtgl/ivily-optimizer/releases/";

static TASK: Mutex<Option<tauri::async_runtime::JoinHandle<()>>> = Mutex::new(None);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    stage: &'static str,
    downloaded: u64,
    total: u64,
}

pub fn validate_manifest_url(url: &str) -> AppResult<tauri::Url> {
    if !url.starts_with(ALLOWED_PREFIX) || !url.ends_with("/latest.json") || url.contains("..") {
        return Err(AppError::InvalidInput("güncelleme bildirimi bu projenin GitHub sürümlerinden gelmiyor".into()));
    }
    tauri::Url::parse(url).map_err(|_| AppError::InvalidInput("geçersiz güncelleme adresi".into()))
}

fn map_err(e: tauri_plugin_updater::Error) -> AppError {
    use tauri_plugin_updater::Error as E;
    tracing::error!(error = %e, "güncelleme hatası");
    match e {
        E::Minisign(_) | E::SignatureUtf8(_) | E::Base64(_) => {
            AppError::Protected("güncelleme dosyasının imzası doğrulanamadı; dosya kurulmadı".into())
        }
        E::Reqwest(_) | E::Network(_) => {
            AppError::Network("güncelleme indirilemedi; internet bağlantınızı denetleyin".into())
        }
        other => AppError::System(format!("güncelleme kurulamadı: {other}")),
    }
}

#[tauri::command]
pub async fn update_install(
    app: AppHandle,
    st: State<'_, Arc<AppState>>,
    manifest_url: String,
    expected_version: Option<String>,
) -> AppResult<()> {
    let url = validate_manifest_url(&manifest_url)?;
    let _ = app.emit("update-progress", Progress { stage: "preparing", downloaded: 0, total: 0 });
    let updater = app
        .updater_builder()
        .endpoints(vec![url])
        .map_err(map_err)?
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(map_err)?;
    let update = updater
        .check()
        .await
        .map_err(map_err)?
        .ok_or_else(|| AppError::NotFound("kurulacak daha yeni bir sürüm bulunamadı".into()))?;
    if let Some(v) = &expected_version {
        if v.trim_start_matches('v') != update.version.trim_start_matches('v') {
            return Err(AppError::Conflict("sürüm bilgisi tutarsız; güncelleme iptal edildi".into()));
        }
    }
    let version = update.version.clone();
    let db = st.db.clone();
    let app2 = app.clone();
    let (tx, rx) = std::sync::mpsc::channel::<AppResult<()>>();
    let handle = tauri::async_runtime::spawn(async move {
        let mut downloaded = 0u64;
        let a = app2.clone();
        let res = update
            .download(
                move |n, total| {
                    downloaded += n as u64;
                    let _ = a.emit(
                        "update-progress",
                        Progress { stage: "downloading", downloaded, total: total.unwrap_or(0) },
                    );
                },
                || {},
            )
            .await;
        let res = match res {
            Ok(bytes) => {
                // download() imzayı zaten doğruladı; kullanıcıya aşamayı göster.
                let _ = app2.emit(
                    "update-progress",
                    Progress { stage: "verifying", downloaded: bytes.len() as u64, total: bytes.len() as u64 },
                );
                let _ = oplog::record(
                    &db,
                    "update",
                    &format!("Güncelleme kuruluyor: {version}"),
                    Outcome::Success,
                    false,
                    serde_json::json!({ "version": version }),
                );
                let _ = app2.emit("update-progress", Progress { stage: "installing", downloaded: 0, total: 0 });
                // Windows'ta install, yükleyiciyi başlattıktan sonra uygulamayı kapatır.
                update.install(&bytes).map_err(map_err)
            }
            Err(e) => Err(map_err(e)),
        };
        let _ = tx.send(res);
    });
    *TASK.lock() = Some(handle);
    let res = tauri::async_runtime::spawn_blocking(move || rx.recv().unwrap_or(Err(AppError::Cancelled)))
        .await
        .map_err(|e| AppError::System(e.to_string()))?;
    *TASK.lock() = None;
    if let Err(e) = &res {
        let _ = oplog::record(
            &st.db,
            "update",
            "Güncelleme başarısız",
            Outcome::Failed,
            false,
            serde_json::json!({ "error": e.to_string() }),
        );
    }
    res
}

#[tauri::command]
pub async fn update_cancel() -> AppResult<bool> {
    match TASK.lock().take() {
        Some(h) => {
            h.abort();
            Ok(true)
        }
        None => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manifest_url_must_be_from_repo() {
        assert!(validate_manifest_url(
            "https://github.com/efecrtgl/ivily-optimizer/releases/download/v1.0.0/latest.json"
        )
        .is_ok());
        assert!(validate_manifest_url(
            "https://github.com/efecrtgl/ivily-optimizer/releases/latest/download/latest.json"
        )
        .is_ok());
        assert!(validate_manifest_url("https://evil.example/latest.json").is_err());
        assert!(validate_manifest_url("https://github.com/someone/else/releases/download/v1/latest.json").is_err());
        assert!(validate_manifest_url(
            "https://github.com/efecrtgl/ivily-optimizer/releases/download/../x/latest.json"
        )
        .is_err());
        assert!(validate_manifest_url("http://github.com/efecrtgl/ivily-optimizer/releases/download/v1/latest.json")
            .is_err());
    }
}
