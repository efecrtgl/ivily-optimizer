//! GitHub Releases üzerinden güncelleme denetimi.
//!
//! Bu modül yalnızca yeni sürüm olup olmadığını denetler. İndirme, imza
//! doğrulama (minisign) ve kurulum ön yüzde `tauri-plugin-updater` ile yapılır.

use crate::db::{now_ts, Db};
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const REPO: &str = "efecrtgl/ivily-optimizer";
const API_URL: &str = "https://api.github.com/repos/efecrtgl/ivily-optimizer/releases?per_page=10";
const ASSET_PREFIX: &str = "https://github.com/efecrtgl/ivily-optimizer/releases/download/";
const HTML_PREFIX: &str = "https://github.com/efecrtgl/ivily-optimizer/";
const KV_KEY: &str = "update_last_check";
const MIN_INTERVAL_SECS: i64 = 60;
const NOTES_MAX_CHARS: usize = 20_000;

#[derive(Debug, Clone, Deserialize)]
pub struct GhAsset {
    pub name: String,
    #[serde(default)]
    pub size: u64,
    pub browser_download_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub draft: bool,
    pub published_at: Option<String>,
    pub html_url: Option<String>,
    #[serde(default)]
    pub assets: Vec<GhAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstallerAsset {
    pub name: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub release_name: Option<String>,
    pub notes: Option<String>,
    pub published_at: Option<String>,
    pub html_url: Option<String>,
    pub installer_asset: Option<InstallerAsset>,
    pub has_signature: bool,
    pub updater_manifest_url: Option<String>,
    pub prerelease: bool,
    pub include_prerelease: bool,
    /// Unix zaman damgası (saniye).
    pub checked_at: i64,
    /// Sonuç önbellekten mi geldi (60 saniyelik asgari aralık)?
    #[serde(default)]
    pub from_cache: bool,
    /// Kullanıcıya gösterilecek Türkçe özet.
    #[serde(default)]
    pub message: String,
}

pub fn parse_version(tag: &str) -> Option<semver::Version> {
    let t = tag.trim();
    let t = t.strip_prefix('v').or_else(|| t.strip_prefix('V')).unwrap_or(t);
    semver::Version::parse(t).ok()
}

pub fn valid_asset_url(url: &str) -> bool {
    url.starts_with(ASSET_PREFIX) && !url.contains("..") && !url.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// Taslakları atlar, gerekirse ön sürümleri atlar, ayrıştırılamayan etiketleri
/// yok sayar ve en yüksek semver sürümünü seçer.
pub fn select_latest(releases: &[GhRelease], include_prerelease: bool) -> Option<(&GhRelease, semver::Version)> {
    releases
        .iter()
        .filter(|r| !r.draft)
        .filter_map(|r| parse_version(&r.tag_name).map(|v| (r, v)))
        .filter(|(r, v)| include_prerelease || (!r.prerelease && v.pre.is_empty()))
        .max_by(|a, b| a.1.cmp(&b.1))
}

fn truncate_chars(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s.to_string(),
    }
}

/// Seçilen sürümden sonucu üretir (saf işlev; testlerde kullanılır).
pub fn build_result(releases: &[GhRelease], current: &str, include_prerelease: bool, checked_at: i64) -> UpdateCheckResult {
    let cur = parse_version(current);
    let mut res = UpdateCheckResult {
        current_version: current.to_string(),
        latest_version: None,
        update_available: false,
        release_name: None,
        notes: None,
        published_at: None,
        html_url: None,
        installer_asset: None,
        has_signature: false,
        updater_manifest_url: None,
        prerelease: false,
        include_prerelease,
        checked_at,
        from_cache: false,
        message: "Henüz yayımlanmış bir sürüm yok.".into(),
    };
    let Some((rel, ver)) = select_latest(releases, include_prerelease) else { return res };
    let installer = rel
        .assets
        .iter()
        .find(|a| a.name.to_ascii_lowercase().ends_with("-setup.exe") && valid_asset_url(&a.browser_download_url));
    res.has_signature = installer
        .map(|i| {
            let sig = format!("{}.sig", i.name);
            rel.assets.iter().any(|a| a.name.eq_ignore_ascii_case(&sig) && valid_asset_url(&a.browser_download_url))
        })
        .unwrap_or(false);
    res.installer_asset = installer.map(|a| InstallerAsset { name: a.name.clone(), size: a.size, url: a.browser_download_url.clone() });
    res.updater_manifest_url = rel
        .assets
        .iter()
        .find(|a| a.name == "latest.json" && valid_asset_url(&a.browser_download_url))
        .map(|a| a.browser_download_url.clone());
    res.update_available = cur.as_ref().map(|c| ver > *c).unwrap_or(false);
    res.latest_version = Some(ver.to_string());
    res.release_name = rel.name.clone().filter(|n| !n.trim().is_empty());
    res.notes = rel.body.as_deref().map(|b| truncate_chars(b, NOTES_MAX_CHARS));
    res.published_at = rel.published_at.clone();
    res.html_url = rel.html_url.clone().filter(|u| u.starts_with(HTML_PREFIX));
    res.prerelease = rel.prerelease || !ver.pre.is_empty();
    res.message = if res.update_available {
        format!("Yeni bir sürüm mevcut: {ver} (kullandığınız sürüm: {current}).")
    } else {
        format!("Ivily Optimizer güncel (sürüm {current}).")
    };
    res
}

pub fn parse_releases(body: &str) -> AppResult<Vec<GhRelease>> {
    serde_json::from_str(body).map_err(|e| {
        tracing::warn!(error = %e, "GitHub yanıtı ayrıştırılamadı");
        AppError::Network("GitHub'dan beklenmeyen bir yanıt alındı; sürüm bilgisi okunamadı".into())
    })
}

/// Hız sınırı mesajı (`x-ratelimit-reset` başlığı varsa sıfırlanma saatiyle).
pub fn rate_limit_message(reset: Option<i64>) -> String {
    match reset.and_then(|r| chrono::DateTime::from_timestamp(r, 0)) {
        Some(t) => format!(
            "GitHub istek sınırına ulaşıldı. Lütfen {} sonrasında tekrar deneyin.",
            t.with_timezone(&chrono::Local).format("%H:%M")
        ),
        None => "GitHub istek sınırına ulaşıldı. Lütfen bir süre sonra tekrar deneyin.".into(),
    }
}

/// Sürüm listesini indirir. 404 durumunda `None` döner (yayımlanmış sürüm yok).
fn fetch() -> AppResult<Option<String>> {
    let config = ureq::Agent::config_builder()
        .https_only(true)
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(10)))
        .max_redirects(3)
        .user_agent("IvilyOptimizer-Updater")
        .build();
    let agent: ureq::Agent = config.into();
    let resp = agent
        .get(API_URL)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .call();
    let mut resp = match resp {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "güncelleme denetimi bağlantı hatası");
            return Err(match e {
                ureq::Error::Timeout(_) => AppError::Network("GitHub zamanında yanıt vermedi; lütfen daha sonra tekrar deneyin".into()),
                ureq::Error::Tls(_) | ureq::Error::Rustls(_) => AppError::Network("GitHub ile güvenli bağlantı kurulamadı".into()),
                _ => AppError::Network("İnternet bağlantısı yok veya GitHub'a ulaşılamıyor".into()),
            });
        }
    };
    let status = resp.status().as_u16();
    let header = |k: &str| resp.headers().get(k).and_then(|v| v.to_str().ok()).map(|s| s.trim().to_string());
    match status {
        200 => {}
        404 => return Ok(None),
        403 | 429 => {
            let remaining = header("x-ratelimit-remaining");
            let reset = header("x-ratelimit-reset").and_then(|s| s.parse::<i64>().ok());
            if status == 429 || remaining.as_deref() == Some("0") || reset.is_some() {
                return Err(AppError::Network(rate_limit_message(reset)));
            }
            return Err(AppError::Network("GitHub isteği reddetti (HTTP 403)".into()));
        }
        s if s >= 500 => return Err(AppError::Network(format!("GitHub şu anda yanıt veremiyor (HTTP {s}); lütfen daha sonra tekrar deneyin"))),
        s => return Err(AppError::Network(format!("GitHub beklenmeyen bir yanıt döndürdü (HTTP {s})"))),
    }
    resp.body_mut()
        .with_config()
        .limit(5 * 1024 * 1024)
        .read_to_string()
        .map(Some)
        .map_err(|_| AppError::Network("GitHub yanıtı okunamadı; bağlantı kesilmiş olabilir".into()))
}

/// Kaydedilmiş son denetim sonucu.
pub fn last_check(db: &Db) -> AppResult<Option<UpdateCheckResult>> {
    Ok(db.kv_get(KV_KEY)?.and_then(|s| serde_json::from_str(&s).ok()))
}

/// Güncelleme denetimi. Son başarılı denetimden bu yana 60 saniye geçmediyse
/// (ve aynı ayarlarla yapıldıysa) API'ye gidilmeden önbellek döndürülür.
pub fn check(db: &Db, include_prerelease: bool) -> AppResult<UpdateCheckResult> {
    let current = env!("CARGO_PKG_VERSION");
    let now = now_ts();
    if let Some(mut prev) = last_check(db)? {
        if prev.include_prerelease == include_prerelease
            && prev.current_version == current
            && now >= prev.checked_at
            && now - prev.checked_at < MIN_INTERVAL_SECS
        {
            prev.from_cache = true;
            return Ok(prev);
        }
    }
    let releases = match fetch()? {
        Some(body) => parse_releases(&body)?,
        None => Vec::new(),
    };
    let res = build_result(&releases, current, include_prerelease, now);
    if let Ok(s) = serde_json::to_string(&res) {
        let _ = db.kv_set(KV_KEY, &s);
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(tag: &str, pre: bool, draft: bool) -> String {
        format!(
            r#"{{"tag_name":"{tag}","name":"Sürüm {tag}","body":"Notlar","prerelease":{pre},"draft":{draft},"published_at":"2026-11-01T10:00:00Z","html_url":"https://github.com/efecrtgl/ivily-optimizer/releases/tag/{tag}","assets":[
              {{"name":"Ivily.Optimizer_{tag}_x64-setup.exe","size":1234,"browser_download_url":"https://github.com/efecrtgl/ivily-optimizer/releases/download/{tag}/Ivily.Optimizer_{tag}_x64-setup.exe"}},
              {{"name":"Ivily.Optimizer_{tag}_x64-setup.exe.sig","size":10,"browser_download_url":"https://github.com/efecrtgl/ivily-optimizer/releases/download/{tag}/Ivily.Optimizer_{tag}_x64-setup.exe.sig"}},
              {{"name":"latest.json","size":10,"browser_download_url":"https://github.com/efecrtgl/ivily-optimizer/releases/download/{tag}/latest.json"}}]}}"#
        )
    }

    fn list(items: &[String]) -> Vec<GhRelease> {
        parse_releases(&format!("[{}]", items.join(","))).unwrap()
    }

    #[test]
    fn semver_not_lexicographic() {
        let r = list(&[rel("v0.9.0", false, false), rel("v0.10.0", false, false), rel("v0.2.0", false, false)]);
        let (best, v) = select_latest(&r, false).unwrap();
        assert_eq!(v.to_string(), "0.10.0");
        assert_eq!(best.tag_name, "v0.10.0");
    }

    #[test]
    fn prerelease_and_draft_handling() {
        let r = list(&[
            rel("v0.3.0", false, true),        // taslak: atlanır
            rel("v0.2.0-beta.1", true, false), // ön sürüm
            rel("v0.1.5", false, false),
            rel("sürüm-x", false, false), // ayrıştırılamaz: yok sayılır
            rel("0.2.0-rc.1", false, false), // prerelease bayrağı yok ama semver ön sürüm
        ]);
        assert_eq!(select_latest(&r, false).unwrap().1.to_string(), "0.1.5");
        assert_eq!(select_latest(&r, true).unwrap().1.to_string(), "0.2.0-rc.1");
        let res = build_result(&r, "0.1.0", false, 1);
        assert!(res.update_available);
        assert_eq!(res.latest_version.as_deref(), Some("0.1.5"));
        assert!(!res.prerelease);
        let res = build_result(&r, "0.1.0", true, 1);
        assert!(res.prerelease);
    }

    #[test]
    fn up_to_date_and_empty() {
        let r = list(&[rel("v0.1.0", false, false)]);
        let res = build_result(&r, "0.1.0", false, 5);
        assert!(!res.update_available);
        assert_eq!(res.latest_version.as_deref(), Some("0.1.0"));
        let res = build_result(&[], "0.1.0", false, 5);
        assert!(!res.update_available);
        assert!(res.latest_version.is_none());
        assert_eq!(res.message, "Henüz yayımlanmış bir sürüm yok.");
        // Mevcut sürüm daha yeniyse güncelleme yok.
        let res = build_result(&r, "0.2.0", false, 5);
        assert!(!res.update_available);
    }

    #[test]
    fn assets_and_signature() {
        let r = list(&[rel("v1.2.3", false, false)]);
        let res = build_result(&r, "1.0.0", false, 1);
        let inst = res.installer_asset.unwrap();
        assert!(inst.name.ends_with("-setup.exe"));
        assert_eq!(inst.size, 1234);
        assert!(res.has_signature);
        assert!(res.updater_manifest_url.unwrap().ends_with("/latest.json"));
        assert!(res.html_url.is_some());
    }

    #[test]
    fn foreign_asset_urls_rejected() {
        let json = r#"[{"tag_name":"v2.0.0","prerelease":false,"draft":false,"html_url":"https://evil.example/x","assets":[
            {"name":"x-setup.exe","size":1,"browser_download_url":"https://evil.example/releases/download/v2.0.0/x-setup.exe"},
            {"name":"latest.json","size":1,"browser_download_url":"https://github.com/other/repo/releases/download/v2.0.0/latest.json"}]}]"#;
        let r = parse_releases(json).unwrap();
        let res = build_result(&r, "1.0.0", false, 1);
        assert!(res.update_available);
        assert!(res.installer_asset.is_none());
        assert!(!res.has_signature);
        assert!(res.updater_manifest_url.is_none());
        assert!(res.html_url.is_none());
        assert!(valid_asset_url("https://github.com/efecrtgl/ivily-optimizer/releases/download/v1/a.exe"));
        assert!(!valid_asset_url("http://github.com/efecrtgl/ivily-optimizer/releases/download/v1/a.exe"));
        assert!(!valid_asset_url("https://github.com/efecrtgl/ivily-optimizer/releases/download/../../x"));
    }

    #[test]
    fn version_parsing_and_notes() {
        assert_eq!(parse_version("v1.2.3").unwrap().to_string(), "1.2.3");
        assert_eq!(parse_version("1.2.3-beta.2").unwrap().pre.as_str(), "beta.2");
        assert!(parse_version("v1.2").is_none());
        assert!(parse_version("latest").is_none());
        let long = "ş".repeat(NOTES_MAX_CHARS + 10);
        let t = truncate_chars(&long, NOTES_MAX_CHARS);
        assert_eq!(t.chars().count(), NOTES_MAX_CHARS + 1);
    }

    #[test]
    fn malformed_json_is_turkish_error() {
        let e = parse_releases("{\"message\":\"Not Found\"}").unwrap_err();
        assert!(e.to_string().contains("beklenmeyen"));
        assert!(parse_releases("<html>").is_err());
    }

    #[test]
    fn rate_limit_text() {
        assert!(rate_limit_message(Some(1_900_000_000)).contains("sonrasında"));
        assert!(rate_limit_message(None).contains("bir süre sonra"));
    }

    #[test]
    fn cache_within_min_interval() {
        let db = Db::open_in_memory().unwrap();
        assert!(last_check(&db).unwrap().is_none());
        let mut res = build_result(&[], env!("CARGO_PKG_VERSION"), false, now_ts());
        res.latest_version = Some("9.9.9".into());
        db.kv_set(KV_KEY, &serde_json::to_string(&res).unwrap()).unwrap();
        // Ağ erişimi olmadan önbellekten dönmeli.
        let c = check(&db, false).unwrap();
        assert!(c.from_cache);
        assert_eq!(c.latest_version.as_deref(), Some("9.9.9"));
        assert_eq!(last_check(&db).unwrap().unwrap().latest_version.as_deref(), Some("9.9.9"));
    }
}
