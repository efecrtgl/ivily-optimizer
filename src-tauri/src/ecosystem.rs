//! Ivily Ekosistem Protokolü (IEP) — 0.1 hazırlık katmanı.
//!
//! Ivily Optimizer bağımsız bir uygulamadır; Ivily Center olmadan tam
//! çalışır. Bu modül, ileride Center'ın Optimizer'ı **yerel olarak** keşfedip
//! durumunu okuyabilmesi için en küçük ve güvenli yüzeyi sağlar:
//!
//! - **Taşıma:** komut satırı. `ivily-optimizer.exe --iep describe` tek bir
//!   JSON tanımlayıcısı yazar ve çıkar; `--iep request` standart girişten tek
//!   bir istek okur, doğrular ve tek bir yanıt yazar. Pencere açılmaz, arka
//!   plan servisi veya dinleyen bir port/boru **yoktur**.
//! - **Yalnızca okuma:** kimlik, protokol sürümü, yetenekler, sağlık durumu,
//!   güncelleme meta verisi ve etkinlik **sayıları** (başlık veya yol yok).
//!   Sistem değiştiren hiçbir işlem bu yüzeyden yapılamaz; kabuk, dosya
//!   sistemi veya keyfi komut erişimi yoktur.
//! - **Doğrulama:** istekler boyut, protokol adı, sürüm uyumluluğu, kimlik
//!   biçimi ve izin verilen yöntem listesine göre reddedilir.
//!
//! Ayrıntılı belirtim: `docs/IEP.md`.

use crate::db::SCHEMA_VERSION;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const PROTOCOL: &str = "iep";
/// Bu uygulamanın konuştuğu protokol sürümü (semver).
pub const PROTOCOL_VERSION: &str = "0.1.0";
pub const APP_ID: &str = "com.efecrtgl.ivilyoptimizer";
pub const PRODUCT: &str = "ivily-optimizer";
/// İstek gövdesi için üst sınır.
pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
/// Etkinlik özetinin kapsadığı gün sayısı.
pub const ACTIVITY_WINDOW_DAYS: i64 = 7;

// ---------------------------------------------------------------------------
// Tanımlayıcı
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolInfo {
    pub name: &'static str,
    pub version: &'static str,
    /// Bu sürümle uyumlu en eski istemci protokol sürümü.
    pub min_compatible: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub id: &'static str,
    pub product: &'static str,
    pub name: &'static str,
    pub version: &'static str,
    pub platform: &'static str,
    pub repository: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportInfo {
    pub kind: &'static str,
    pub describe: [&'static str; 2],
    pub request: [&'static str; 2],
    pub background_service: bool,
}

/// Yeteneğin erişim düzeyi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Access {
    /// Yalnızca okuma; sistem durumunu değiştirmez.
    Read,
    /// Optimizer'ın kendi arayüzünde kullanıcı onayı gerektirir.
    UserApproved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Availability {
    Available,
    Planned,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    pub id: &'static str,
    pub version: &'static str,
    pub access: Access,
    pub status: Availability,
    pub description: &'static str,
}

/// Optimizer'ın ilan ettiği yetenekler. `planned` olanlar henüz
/// uygulanmamıştır; istemciler bunlara güvenmemelidir.
pub fn capabilities() -> Vec<Capability> {
    use Access::*;
    use Availability::*;
    vec![
        Capability {
            id: "app.identity",
            version: "1",
            access: Read,
            status: Available,
            description: "Uygulama kimliği, sürümü ve protokol sürümü.",
        },
        Capability {
            id: "app.capabilities",
            version: "1",
            access: Read,
            status: Available,
            description: "Desteklenen yeteneklerin listesi.",
        },
        Capability {
            id: "app.health",
            version: "1",
            access: Read,
            status: Available,
            description: "Veri klasörü ve veritabanı durumu.",
        },
        Capability {
            id: "update.metadata",
            version: "1",
            access: Read,
            status: Available,
            description: "Son güncelleme denetiminin sonucu. Ağ isteği yapılmaz.",
        },
        Capability {
            id: "activity.summary",
            version: "1",
            access: Read,
            status: Available,
            description: "Son 7 gündeki işlem geçmişi kayıtlarının türe ve sonuca göre sayıları. Başlık veya dosya yolu içermez.",
        },
        Capability {
            id: "system.insights",
            version: "1",
            access: Read,
            status: Planned,
            description: "Disk doluluğu ve Ivily Zekâsı önerilerinin özeti.",
        },
        Capability {
            id: "operation.request",
            version: "1",
            access: UserApproved,
            status: Planned,
            description: "Center bir bakım işlemi önerir; Optimizer bunu kendi arayüzünde gösterir ve yalnızca kullanıcı onaylarsa yürütür.",
        },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HealthStatus {
    Ok,
    Degraded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DatabaseState {
    /// Henüz oluşturulmamış (uygulama hiç açılmamış).
    Missing,
    Ok,
    /// Şema bu sürümden eski; uygulama açıldığında yükseltilir.
    NeedsMigration,
    /// Daha yeni bir sürüme ait.
    NewerSchema,
    Unreadable,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub status: HealthStatus,
    pub data_dir_exists: bool,
    pub database: DatabaseState,
    pub schema_version: Option<i64>,
    pub expected_schema_version: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMetadata {
    pub current_version: &'static str,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub checked_at: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivitySummary {
    pub window_days: i64,
    pub total: u64,
    pub by_kind: BTreeMap<String, u64>,
    pub by_outcome: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    pub protocol: ProtocolInfo,
    pub app: AppInfo,
    pub transport: TransportInfo,
    pub capabilities: Vec<Capability>,
    pub health: Health,
    pub update: UpdateMetadata,
    pub activity: Option<ActivitySummary>,
    pub generated_at: i64,
}

/// Veritabanını **oluşturmadan ve değiştirmeden** açar. Uygulama açıkken de
/// okunabilir (WAL). Dosya yoksa `None` döner.
fn open_existing_read_only(path: &Path) -> Result<rusqlite::Connection, DatabaseState> {
    use rusqlite::OpenFlags;
    if !path.is_file() {
        return Err(DatabaseState::Missing);
    }
    let conn = rusqlite::Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| DatabaseState::Unreadable)?;
    conn.pragma_update(None, "query_only", true).map_err(|_| DatabaseState::Unreadable)?;
    // `query_only` SQL yazımlarını engeller; ancak SQLite son bağlantı
    // kapanırken bekleyen WAL içeriğini ana dosyaya aktarır (checkpoint).
    // Tanımlayıcı veri dosyasına hiç dokunmamalı.
    conn.set_db_config(rusqlite::config::DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)
        .map_err(|_| DatabaseState::Unreadable)?;
    conn.busy_timeout(std::time::Duration::from_secs(2)).map_err(|_| DatabaseState::Unreadable)?;
    Ok(conn)
}

fn read_activity(conn: &rusqlite::Connection, now: i64) -> Option<ActivitySummary> {
    let since = now - ACTIVITY_WINDOW_DAYS * 86_400;
    let mut st =
        conn.prepare("SELECT kind, outcome, COUNT(*) FROM oplog WHERE ts >= ?1 GROUP BY kind, outcome").ok()?;
    let rows =
        st.query_map([since], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))).ok()?;
    let mut s = ActivitySummary { window_days: ACTIVITY_WINDOW_DAYS, ..Default::default() };
    for (kind, outcome, n) in rows.flatten() {
        let n = n.max(0) as u64;
        s.total += n;
        *s.by_kind.entry(kind).or_default() += n;
        *s.by_outcome.entry(outcome).or_default() += n;
    }
    Some(s)
}

fn read_update(conn: &rusqlite::Connection) -> UpdateMetadata {
    let cached: Option<serde_json::Value> = conn
        .query_row("SELECT value FROM kv WHERE key = 'update_last_check'", [], |r| r.get::<_, String>(0))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());
    let latest = cached.as_ref().and_then(|v| v.get("latestVersion")).and_then(|v| v.as_str()).map(String::from);
    let checked_at = cached.as_ref().and_then(|v| v.get("checkedAt")).and_then(|v| v.as_i64());
    // Önbellekteki "updateAvailable" eski bir sürümde hesaplanmış olabilir;
    // çalışan sürümle yeniden karşılaştır.
    let update_available = latest
        .as_deref()
        .and_then(|l| semver::Version::parse(l.trim_start_matches('v')).ok())
        .zip(semver::Version::parse(env!("CARGO_PKG_VERSION")).ok())
        .is_some_and(|(l, c)| l > c);
    UpdateMetadata { current_version: env!("CARGO_PKG_VERSION"), latest_version: latest, update_available, checked_at }
}

/// Belirtilen veri klasörü ve veritabanı için tanımlayıcıyı üretir.
pub fn describe_at(data_dir: &Path, db_path: &Path) -> Descriptor {
    let now = chrono::Utc::now().timestamp();
    let mut schema_version = None;
    let mut activity = None;
    let mut update = UpdateMetadata {
        current_version: env!("CARGO_PKG_VERSION"),
        latest_version: None,
        update_available: false,
        checked_at: None,
    };
    let database = match open_existing_read_only(db_path) {
        Err(state) => state,
        Ok(conn) => match conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0)) {
            Err(_) => DatabaseState::Unreadable,
            Ok(v) => {
                schema_version = Some(v);
                if v > SCHEMA_VERSION {
                    DatabaseState::NewerSchema
                } else {
                    activity = read_activity(&conn, now);
                    update = read_update(&conn);
                    if v < SCHEMA_VERSION {
                        DatabaseState::NeedsMigration
                    } else {
                        DatabaseState::Ok
                    }
                }
            }
        },
    };
    let healthy = matches!(database, DatabaseState::Ok | DatabaseState::Missing | DatabaseState::NeedsMigration);
    Descriptor {
        protocol: ProtocolInfo { name: PROTOCOL, version: PROTOCOL_VERSION, min_compatible: PROTOCOL_VERSION },
        app: AppInfo {
            id: APP_ID,
            product: PRODUCT,
            name: "Ivily Optimizer",
            version: env!("CARGO_PKG_VERSION"),
            platform: "windows",
            repository: "https://github.com/efecrtgl/ivily-optimizer",
        },
        transport: TransportInfo {
            kind: "cli",
            describe: ["--iep", "describe"],
            request: ["--iep", "request"],
            background_service: false,
        },
        capabilities: capabilities(),
        health: Health {
            status: if healthy { HealthStatus::Ok } else { HealthStatus::Degraded },
            data_dir_exists: data_dir.is_dir(),
            database,
            schema_version,
            expected_schema_version: SCHEMA_VERSION,
        },
        update,
        activity,
        generated_at: now,
    }
}

/// Uygulamanın gerçek veri konumları için tanımlayıcı.
pub fn describe() -> Descriptor {
    describe_at(&crate::paths::data_dir(), &crate::paths::db_path())
}

// ---------------------------------------------------------------------------
// İstek doğrulama
// ---------------------------------------------------------------------------

/// İstek zarfı. Bilinmeyen alanlar reddedilir.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Request {
    pub protocol: String,
    pub version: String,
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

/// İzin verilen yöntemler. Hepsi salt okunurdur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Describe,
    Health,
    Capabilities,
}

impl Method {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "describe" => Some(Self::Describe),
            "health" => Some(Self::Health),
            "capabilities" => Some(Self::Capabilities),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProtocolError {
    pub code: &'static str,
    pub message: String,
}

fn perr(code: &'static str, message: impl Into<String>) -> ProtocolError {
    ProtocolError { code, message: message.into() }
}

/// İstemcinin protokol sürümü bu uygulamayla uyumlu mu? Semver kuralı: ana
/// sürüm aynı olmalı; 0.x sürümlerinde ikincil sürüm de aynı olmalıdır.
pub fn is_compatible(client: &str) -> bool {
    let (Ok(c), Ok(ours)) = (semver::Version::parse(client), semver::Version::parse(PROTOCOL_VERSION)) else {
        return false;
    };
    if c.major != ours.major {
        return false;
    }
    if ours.major == 0 {
        return c.minor == ours.minor;
    }
    true
}

/// Ham isteği ayrıştırır ve doğrular.
pub fn parse_request(text: &str) -> Result<(Request, Method), ProtocolError> {
    if text.len() > MAX_REQUEST_BYTES {
        return Err(perr("too_large", format!("istek en fazla {MAX_REQUEST_BYTES} bayt olabilir")));
    }
    let req: Request = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|e| perr("invalid_request", e.to_string()))?;
    if req.protocol != PROTOCOL {
        return Err(perr("unsupported_protocol", format!("beklenen protokol '{PROTOCOL}'")));
    }
    if !is_compatible(&req.version) {
        return Err(perr(
            "incompatible_version",
            format!("istemci sürümü {} bu uygulamanın IEP {PROTOCOL_VERSION} sürümüyle uyumlu değil", req.version),
        ));
    }
    if req.id.is_empty() || req.id.len() > 64 || !req.id.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    {
        return Err(perr(
            "invalid_id",
            "kimlik 1–64 karakter olmalı ve yalnızca harf, rakam, '-', '_' veya '.' içermeli",
        ));
    }
    let method = Method::parse(&req.method).ok_or_else(|| perr("unknown_method", "bilinmeyen yöntem"))?;
    match &req.params {
        None | Some(serde_json::Value::Null) => {}
        Some(serde_json::Value::Object(m)) if m.is_empty() => {}
        Some(_) => return Err(perr("invalid_params", "bu yöntem parametre almaz")),
    }
    Ok((req, method))
}

/// Bir isteği işler ve yanıt JSON'unu döndürür. Yanıt her zaman geçerli bir
/// JSON nesnesidir; hata durumunda `ok: false` ve `error` alanı içerir.
pub fn handle_request(text: &str, descriptor: impl FnOnce() -> Descriptor) -> serde_json::Value {
    let envelope =
        |id: Option<&str>| serde_json::json!({ "protocol": PROTOCOL, "version": PROTOCOL_VERSION, "id": id });
    match parse_request(text) {
        Err(e) => {
            let mut v = envelope(None);
            v["ok"] = false.into();
            v["error"] = serde_json::to_value(e).unwrap_or_default();
            v
        }
        Ok((req, method)) => {
            let d = descriptor();
            let result = match method {
                Method::Describe => serde_json::to_value(&d),
                Method::Health => serde_json::to_value(&d.health),
                Method::Capabilities => serde_json::to_value(&d.capabilities),
            }
            .unwrap_or_default();
            let mut v = envelope(Some(&req.id));
            v["ok"] = true.into();
            v["result"] = result;
            v
        }
    }
}

// ---------------------------------------------------------------------------
// Komut satırı
// ---------------------------------------------------------------------------

/// `--iep` bağımsız değişkenlerini işler. IEP çağrısı değilse `None` döner ve
/// uygulama normal şekilde açılır; aksi halde çıkış kodu döner. Arayüz ve tek
/// örnek kilidi hiç başlatılmaz.
pub fn run_cli<I: IntoIterator<Item = String>>(
    args: I,
    stdin: &mut dyn std::io::Read,
    stdout: &mut dyn std::io::Write,
) -> Option<i32> {
    let args: Vec<String> = args.into_iter().collect();
    let pos = args.iter().position(|a| a == "--iep")?;
    let out = match args.get(pos + 1).map(String::as_str) {
        Some("describe") => serde_json::to_value(describe()).unwrap_or_default(),
        Some("request") => {
            let mut buf = Vec::new();
            let _ = std::io::Read::read_to_end(
                &mut std::io::Read::take(&mut *stdin, MAX_REQUEST_BYTES as u64 + 1),
                &mut buf,
            );
            let text = String::from_utf8_lossy(&buf);
            let v = handle_request(&text, describe);
            let ok = v["ok"].as_bool().unwrap_or(false);
            let _ = writeln!(stdout, "{v}");
            return Some(if ok { 0 } else { 1 });
        }
        _ => {
            let _ = writeln!(
                stdout,
                "{}",
                serde_json::json!({
                    "protocol": PROTOCOL, "version": PROTOCOL_VERSION, "ok": false,
                    "error": { "code": "usage", "message": "kullanım: --iep describe | --iep request (istek standart girişten okunur)" }
                })
            );
            return Some(2);
        }
    };
    let _ = writeln!(stdout, "{out}");
    Some(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn req(over: serde_json::Value) -> String {
        let mut v = serde_json::json!({ "protocol": "iep", "version": "0.1.0", "id": "c-1", "method": "describe" });
        for (k, val) in over.as_object().unwrap() {
            v[k] = val.clone();
        }
        v.to_string()
    }

    #[test]
    fn descriptor_for_missing_database_does_not_create_it() {
        let t = tempfile::tempdir().unwrap();
        let db = t.path().join("ivily.db");
        let d = describe_at(t.path(), &db);
        assert!(!db.exists(), "tanımlayıcı veritabanı oluşturmamalı");
        assert_eq!(d.health.database, DatabaseState::Missing);
        assert_eq!(d.health.status, HealthStatus::Ok);
        assert!(d.activity.is_none());
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(v["protocol"]["name"], "iep");
        assert_eq!(v["app"]["id"], APP_ID);
        assert_eq!(v["transport"]["backgroundService"], false);
        assert_eq!(v["capabilities"][0]["access"], "read");
    }

    #[test]
    fn descriptor_reads_activity_counts_and_update_metadata_without_writing() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("ivily.db");
        {
            let db = Db::open(&path).unwrap();
            crate::oplog::record(
                &db,
                "cleanup",
                "Gizli/başlık C:\\Users\\x",
                crate::oplog::Outcome::Success,
                true,
                serde_json::json!({}),
            )
            .unwrap();
            crate::oplog::record(&db, "startup", "x", crate::oplog::Outcome::Failed, false, serde_json::json!({}))
                .unwrap();
            db.kv_set("update_last_check", r#"{"latestVersion":"99.0.0","checkedAt":1700000000}"#).unwrap();
        }
        let before = std::fs::read(&path).unwrap();
        let d = describe_at(t.path(), &path);
        assert_eq!(d.health.database, DatabaseState::Ok);
        let a = d.activity.unwrap();
        assert_eq!(a.total, 2);
        assert_eq!(a.by_kind["cleanup"], 1);
        assert_eq!(a.by_outcome["failed"], 1);
        assert_eq!(d.update.latest_version.as_deref(), Some("99.0.0"));
        assert!(d.update.update_available);
        assert_eq!(d.update.checked_at, Some(1_700_000_000));
        // Başlıklar ve yollar dışarı verilmez.
        let s = serde_json::to_string(&serde_json::to_value(describe_at(t.path(), &path)).unwrap()).unwrap();
        assert!(!s.contains("Gizli"));
        assert_eq!(std::fs::read(&path).unwrap(), before, "veritabanı değişmemeli");
    }

    #[test]
    fn pending_wal_is_not_checkpointed_into_the_database() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("ivily.db");
        drop(Db::open(&path).unwrap());
        // Uygulama zorla kapatılmış gibi: yazılmış ama ana dosyaya aktarılmamış
        // bir WAL bırak ve açık bağlantı kalmasın.
        {
            let c = rusqlite::Connection::open(&path).unwrap();
            c.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
            c.set_db_config(rusqlite::config::DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true).unwrap();
            c.execute("INSERT INTO kv(key, value) VALUES('x', 'y')", []).unwrap();
        }
        assert!(t.path().join("ivily.db-wal").metadata().map(|m| m.len() > 0).unwrap_or(false), "WAL bekleniyordu");
        let before = std::fs::read(&path).unwrap();
        let d = describe_at(t.path(), &path);
        assert_eq!(d.health.database, DatabaseState::Ok);
        assert_eq!(std::fs::read(&path).unwrap(), before, "ana veritabanı dosyası değişmemeli");
    }

    #[test]
    fn newer_schema_is_reported_as_degraded() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("ivily.db");
        let c = rusqlite::Connection::open(&path).unwrap();
        c.pragma_update(None, "user_version", SCHEMA_VERSION + 1).unwrap();
        drop(c);
        let d = describe_at(t.path(), &path);
        assert_eq!(d.health.database, DatabaseState::NewerSchema);
        assert_eq!(d.health.status, HealthStatus::Degraded);
    }

    #[test]
    fn compatibility_follows_semver() {
        assert!(is_compatible("0.1.0"));
        assert!(is_compatible("0.1.7"));
        assert!(!is_compatible("0.2.0"));
        assert!(!is_compatible("1.0.0"));
        assert!(!is_compatible("bozuk"));
        assert!(!is_compatible(""));
    }

    #[test]
    fn valid_requests_are_accepted() {
        let (r, m) = parse_request(&req(serde_json::json!({}))).unwrap();
        assert_eq!((r.id.as_str(), m), ("c-1", Method::Describe));
        assert_eq!(
            parse_request(&req(serde_json::json!({ "method": "health", "params": {} }))).unwrap().1,
            Method::Health
        );
        assert_eq!(
            parse_request(&req(serde_json::json!({ "method": "capabilities", "params": null }))).unwrap().1,
            Method::Capabilities
        );
    }

    #[test]
    fn invalid_requests_are_rejected_with_codes() {
        let code = |s: &str| parse_request(s).unwrap_err().code;
        assert_eq!(code("{bozuk"), "invalid_request");
        assert_eq!(code("[]"), "invalid_request");
        assert_eq!(code(&req(serde_json::json!({ "extra": 1 }))), "invalid_request");
        assert_eq!(code(&req(serde_json::json!({ "protocol": "http" }))), "unsupported_protocol");
        assert_eq!(code(&req(serde_json::json!({ "version": "1.0.0" }))), "incompatible_version");
        assert_eq!(code(&req(serde_json::json!({ "id": "" }))), "invalid_id");
        assert_eq!(code(&req(serde_json::json!({ "id": "a b" }))), "invalid_id");
        assert_eq!(code(&req(serde_json::json!({ "id": "x".repeat(65) }))), "invalid_id");
        // Sistem değiştiren veya bilinmeyen yöntemler yoktur.
        for m in ["run", "exec", "shell", "delete", "cleanup.run", "service.stop", ""] {
            assert_eq!(code(&req(serde_json::json!({ "method": m }))), "unknown_method", "{m}");
        }
        assert_eq!(code(&req(serde_json::json!({ "params": { "path": "C:\\" } }))), "invalid_params");
        assert_eq!(code(&req(serde_json::json!({ "params": [1] }))), "invalid_params");
        assert_eq!(code(&" ".repeat(MAX_REQUEST_BYTES + 1)), "too_large");
    }

    #[test]
    fn responses_echo_id_and_report_errors() {
        let t = tempfile::tempdir().unwrap();
        let d = || describe_at(t.path(), &t.path().join("yok.db"));
        let ok = handle_request(&req(serde_json::json!({ "method": "health" })), d);
        assert_eq!(ok["ok"], true);
        assert_eq!(ok["id"], "c-1");
        assert_eq!(ok["result"]["database"], "missing");
        let bad = handle_request("{}", d);
        assert_eq!(bad["ok"], false);
        assert_eq!(bad["id"], serde_json::Value::Null);
        assert_eq!(bad["error"]["code"], "invalid_request");
    }

    #[test]
    fn cli_only_handles_iep_arguments() {
        let mut out = Vec::new();
        let mut empty: &[u8] = b"";
        assert_eq!(run_cli(["--minimized".to_string()], &mut empty, &mut out), None);
        assert!(out.is_empty());
        assert_eq!(run_cli(["--iep".to_string()], &mut empty, &mut out), Some(2));
        let mut out = Vec::new();
        let mut input: &[u8] = br#"{"protocol":"iep","version":"0.1.0","id":"x","method":"nope"}"#;
        assert_eq!(run_cli(["--iep".to_string(), "request".to_string()], &mut input, &mut out), Some(1));
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["error"]["code"], "unknown_method");
    }
}
