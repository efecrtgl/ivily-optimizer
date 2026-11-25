//! Sistem geçmişi ve olay günlüğü zekâsı.
//!
//! Windows olay günlükleri (System, Application) sabit bir PowerShell
//! betiğiyle salt okunur olarak sorgulanır. Olaylar kategorilere ayrılır,
//! tekrarlayan örüntüler gruplanır ve bilinen olaylar için kısa Türkçe
//! açıklamalar ("Ivily yorumu") eklenir. Açıklamalar hiçbir zaman neden-sonuç
//! ilişkisi iddia etmez.

use crate::error::{AppError, AppResult};
use chrono::{Local, NaiveTime, TimeZone};
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;
use std::time::Instant;

/// Olay sorgu betiği. Değerler ortam değişkenlerinden okunur ve Rust
/// tarafında doğrulanmıştır:
/// - `IVILY_EV_LOGS`: izin listesindeki günlük adları (virgülle)
/// - `IVILY_EV_LEVELS`: tamsayı düzeyler (virgülle)
/// - `IVILY_EV_MAX`: tamsayı
/// - `IVILY_EV_DAYS` veya `IVILY_EV_FROM` + `IVILY_EV_TO` (unix saniye)
const QUERY_SCRIPT: &str = r#"[Console]::OutputEncoding=[System.Text.Encoding]::UTF8;
$ErrorActionPreference='SilentlyContinue';
$logs=@($env:IVILY_EV_LOGS -split ',');
$levels=@($env:IVILY_EV_LEVELS -split ',' | ForEach-Object { [int]$_ });
$max=[int]$env:IVILY_EV_MAX;
$f=@{ LogName=$logs; Level=$levels };
if ($env:IVILY_EV_FROM) {
  $f['StartTime']=[DateTimeOffset]::FromUnixTimeSeconds([long]$env:IVILY_EV_FROM).LocalDateTime;
  $f['EndTime']=[DateTimeOffset]::FromUnixTimeSeconds([long]$env:IVILY_EV_TO).LocalDateTime;
} else {
  $f['StartTime']=(Get-Date).AddDays(-[int]$env:IVILY_EV_DAYS);
}
$ev=@(Get-WinEvent -FilterHashtable $f -MaxEvents $max -ErrorAction SilentlyContinue);
$r=@($ev | Where-Object { $_.TimeCreated } | ForEach-Object {
  $m=$null; try { $m=$_.Message } catch { $m=$null }
  if ($m -and $m.Length -gt 4000) { $m=$m.Substring(0,4000) }
  [pscustomobject]@{ t=([DateTimeOffset]$_.TimeCreated).ToUnixTimeSeconds(); g=$_.LogName; v=[int]$_.Level; p=$_.ProviderName; i=$_.Id; r=$_.RecordId; m=$m }
});
ConvertTo-Json -InputObject $r -Depth 2 -Compress"#;

const ALLOWED_LOGS: &[&str] = &["System", "Application"];
const MSG_MAX: usize = 4000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventItem {
    pub id: String,
    pub ts: i64,
    pub log: String,
    pub level: u8,
    pub level_label: String,
    pub provider: String,
    pub event_id: u32,
    pub message: String,
    pub category: String,
    pub category_label: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelStats {
    pub critical: u32,
    pub error: u32,
    pub warning: u32,
    pub info: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryCount {
    pub code: String,
    pub label: String,
    pub count: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayStats {
    pub day: i64,
    pub critical: u32,
    pub error: u32,
    pub warning: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventStats {
    pub by_level: LevelStats,
    pub by_category: Vec<CategoryCount>,
    pub by_day: Vec<DayStats>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventPattern {
    pub provider: String,
    pub event_id: u32,
    pub category: String,
    pub category_label: String,
    pub level: u8,
    pub count: u32,
    pub first_ts: i64,
    pub last_ts: i64,
    pub distinct_days: u32,
    pub sample_message: String,
    pub interpretation: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notable {
    pub unexpected_shutdowns: u32,
    pub app_crashes: u32,
    pub app_hangs: u32,
    pub driver_issues: u32,
    pub disk_issues: u32,
    pub service_failures: u32,
    pub update_failures: u32,
    pub bsod: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsReport {
    pub events: Vec<EventItem>,
    pub truncated: bool,
    pub stats: EventStats,
    pub patterns: Vec<EventPattern>,
    pub notable: Notable,
    pub queried_at: i64,
    pub days: u32,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsContext {
    pub from: i64,
    pub to: i64,
    pub events: Vec<EventItem>,
    pub note: String,
}

// ---------------------------------------------------------------------------
// Eşleme tabloları
// ---------------------------------------------------------------------------

pub fn level_label(level: u8) -> &'static str {
    match level {
        1 => "Kritik",
        2 => "Hata",
        3 => "Uyarı",
        _ => "Bilgi",
    }
}

pub fn category_label(code: &str) -> &'static str {
    match code {
        "unexpected_shutdown" => "Beklenmedik kapanma",
        "bsod" => "Mavi ekran (sistem hatası)",
        "app_crash" => "Uygulama çökmesi",
        "app_hang" => "Uygulama yanıt vermedi",
        "disk" => "Disk ve dosya sistemi",
        "driver" => "Sürücü",
        "hardware" => "Donanım hata bildirimi",
        "service" => "Hizmet sorunu",
        "update" => "Windows Update",
        "network" => "Ağ",
        "security_center" => "Güvenlik Merkezi",
        "defender" => "Microsoft Defender",
        "boot" => "Açılış ve kapanış",
        "shutdown_planned" => "Planlı kapatma / yeniden başlatma",
        _ => "Diğer",
    }
}

/// Sağlayıcı ve olay kimliğine göre kategori. `message` yalnızca Windows
/// Error Reporting olaylarını ayırt etmek için kullanılır.
pub fn categorize(provider: &str, id: u32, message: &str) -> &'static str {
    let p = provider.to_ascii_lowercase();
    match (p.as_str(), id) {
        ("microsoft-windows-kernel-power", 41) | ("eventlog", 6008) => "unexpected_shutdown",
        ("microsoft-windows-wer-systemerrorreporting", 1001) | ("bugcheck", 1001) => "bsod",
        ("windows error reporting", 1001) => {
            if message.contains("BlueScreen") {
                "bsod"
            } else if message.contains("APPCRASH") {
                "app_crash"
            } else if message.contains("AppHang") {
                "app_hang"
            } else {
                "other"
            }
        }
        ("application error", 1000) | (".net runtime", 1026) => "app_crash",
        ("application hang", 1002) => "app_hang",
        ("disk", 7 | 51 | 153 | 154) => "disk",
        ("ntfs" | "microsoft-windows-ntfs", 55 | 98 | 137) => "disk",
        ("volmgr", 161) => "disk",
        ("storahci" | "stornvme", 129) => "disk",
        ("display", 4101) => "driver",
        ("microsoft-windows-kernel-pnp", 219) => "driver",
        ("microsoft-windows-whea-logger", _) => "hardware",
        ("service control manager", 7000 | 7001 | 7009 | 7011 | 7022 | 7023 | 7024 | 7031 | 7034 | 7043) => "service",
        ("microsoft-windows-windowsupdateclient", 20 | 25 | 31) => "update",
        ("microsoft-windows-servicing", _) => "update",
        ("microsoft-windows-dns-client", 1014) => "network",
        ("tcpip", 4199 | 4227) => "network",
        ("netlogon", _) => "network",
        ("microsoft-windows-windows defender", _) => "defender",
        ("securitycenter" | "microsoft-windows-security-center", _) => "security_center",
        ("microsoft-windows-kernel-boot", _) => "boot",
        ("microsoft-windows-kernel-general", 12 | 13) => "boot",
        ("user32", 1074) => "shutdown_planned",
        _ => {
            if p.starts_with("nvlddmkm") || p.starts_with("amdkmdag") || p.starts_with("igfx") || p.starts_with("amdwddmg") {
                "driver"
            } else {
                "other"
            }
        }
    }
}

const UNKNOWN_INTERPRETATION: &str = "Bu olay seçilen dönemde tekrarlandı. Ivily bu olay için özel bir yorum içermiyor.";

/// Kısa Türkçe açıklama ("Ivily yorumu").
pub fn interpretation(provider: &str, id: u32, category: &str) -> &'static str {
    let p = provider.to_ascii_lowercase();
    match (p.as_str(), id) {
        ("microsoft-windows-kernel-power", 41) => {
            return "Bilgisayar düzgün kapatılmadan yeniden başladı (ör. güç kesintisi, donma veya zorla kapatma). Nedeni bu olaydan tek başına anlaşılamaz."
        }
        ("eventlog", 6008) => {
            return "Windows, bir önceki kapanmanın beklenmedik şekilde gerçekleştiğini kaydetti. Nedeni bu olaydan tek başına anlaşılamaz."
        }
        ("microsoft-windows-dns-client", 1014) => {
            return "Bir alan adı için DNS yanıtı zamanında alınamadı. Kısa süreli bağlantı sorunlarında olağandır; sık tekrarlıyorsa ağ veya DNS ayarlarını gözden geçirmek yardımcı olabilir."
        }
        ("service control manager", 7031 | 7034) => {
            return "Bir hizmet beklenmedik şekilde sonlandı. Hizmetin adı olay metninde yer alır; Windows bazı hizmetleri otomatik olarak yeniden başlatır."
        }
        ("service control manager", 7000 | 7009 | 7011 | 7022 | 7023 | 7024) => {
            return "Bir hizmet başlatılamadı veya zamanında yanıt vermedi. Hizmetin adı olay metninde yer alır; ilgili yazılım kaldırılmış ya da güncellenmiş olabilir."
        }
        _ => {}
    }
    match category {
        "unexpected_shutdown" => "Bilgisayar beklenmedik şekilde kapandı veya yeniden başladı. Nedeni bu olaydan tek başına anlaşılamaz.",
        "bsod" => "Windows bir sistem hatasıyla (mavi ekran) yeniden başladı. Olay metnindeki hata kodu, sorunun kaynağını araştırmak için kullanılabilir.",
        "app_crash" => "Bir uygulama beklenmedik şekilde kapandı. Aynı uygulama sık sık çöküyorsa güncellemek veya yeniden yüklemek yardımcı olabilir.",
        "app_hang" => "Bir uygulama bir süre yanıt vermedi ve kapatıldı. Tek seferlik donmalar genellikle önemli değildir.",
        "disk" => "Disk veya dosya sistemiyle ilgili bir sorun kaydedildi. Tekrarlıyorsa disk sağlığını denetlemeniz ve önemli verilerinizi yedeklemeniz önerilir.",
        "driver" => "Bir aygıt sürücüsü (çoğunlukla ekran kartı) hata bildirdi. Tekrarlıyorsa sürücüyü üreticinin sitesinden güncellemek yardımcı olabilir.",
        "hardware" => "Donanım hata bildirimi (WHEA) kaydedildi. Düzeltilmiş hatalar çoğu zaman zararsızdır; sık tekrarlanması donanımla ilgili bir soruna işaret edebilir.",
        "service" => "Bir Windows hizmetiyle ilgili sorun kaydedildi. Hizmetin adı olay metninde yer alır.",
        "update" => "Bir Windows güncellemesi yüklenemedi veya güncelleme bileşeni sorun bildirdi. Windows Update genellikle daha sonra yeniden dener.",
        "network" => "Ağ bağlantısı veya ad çözümlemesiyle ilgili bir sorun kaydedildi. Kısa süreli kesintilerde olağandır.",
        "defender" => "Microsoft Defender bir uyarı veya hata kaydetti. Ayrıntılar için Windows Güvenlik uygulamasını kontrol edebilirsiniz.",
        "security_center" => "Windows Güvenlik Merkezi bir durum değişikliği kaydetti (ör. koruma bileşeni kapalı veya güncel değil).",
        "boot" => "Windows açılış veya kapanış sürecine ait bir kayıt. Genellikle olağandır.",
        "shutdown_planned" => "Bir kullanıcı veya uygulama bilgisayarı planlı olarak kapattı ya da yeniden başlattı. Bu olağan bir kayıttır.",
        _ => UNKNOWN_INTERPRETATION,
    }
}

// ---------------------------------------------------------------------------
// Ayrıştırma ve analiz
// ---------------------------------------------------------------------------

fn truncate_chars(s: &str, n: usize) -> String {
    match s.char_indices().nth(n) {
        Some((i, _)) => s[..i].to_string(),
        None => s.to_string(),
    }
}

/// Betik çıktısını ayrıştırır (tek nesne, dizi veya boş).
pub fn parse_events(json: &str) -> AppResult<Vec<EventItem>> {
    let items = super::json_array(json).map_err(|e| {
        tracing::warn!(error = %e, "olay JSON'u ayrıştırılamadı");
        AppError::System("olay günlüğü okunamadı (beklenmeyen çıktı)".into())
    })?;
    let mut out = Vec::with_capacity(items.len());
    for v in &items {
        let Some(ts) = v.get("t").and_then(Value::as_i64) else { continue };
        let log = v.get("g").and_then(Value::as_str).unwrap_or("").to_string();
        let raw_level = v.get("v").and_then(Value::as_u64).unwrap_or(4);
        // 0 (LogAlways) ve 5 (Ayrıntılı) bilgi olarak gösterilir.
        let level = if (1..=3).contains(&raw_level) { raw_level as u8 } else { 4 };
        let provider = v.get("p").and_then(Value::as_str).unwrap_or("").to_string();
        let event_id = v.get("i").and_then(Value::as_u64).unwrap_or(0) as u32;
        let record = v.get("r").and_then(Value::as_u64).unwrap_or(0);
        let message = truncate_chars(v.get("m").and_then(Value::as_str).unwrap_or("").trim(), MSG_MAX);
        let category = categorize(&provider, event_id, &message);
        out.push(EventItem {
            id: format!("{log}:{record}"),
            ts,
            level_label: level_label(level).to_string(),
            level,
            log,
            provider,
            event_id,
            message,
            category: category.to_string(),
            category_label: category_label(category).to_string(),
        });
    }
    out.sort_by(|a, b| b.ts.cmp(&a.ts));
    Ok(out)
}

/// Yerel gece yarısının unix zamanı.
pub fn local_midnight(ts: i64) -> i64 {
    let Some(dt) = Local.timestamp_opt(ts, 0).single() else { return ts - ts.rem_euclid(86_400) };
    let midnight = dt.date_naive().and_time(NaiveTime::MIN);
    Local.from_local_datetime(&midnight).earliest().map(|d| d.timestamp()).unwrap_or(ts - ts.rem_euclid(86_400))
}

pub fn compute_stats(events: &[EventItem]) -> EventStats {
    let mut by_level = LevelStats::default();
    let mut cats: HashMap<&str, u32> = HashMap::new();
    let mut days: BTreeMap<i64, DayStats> = BTreeMap::new();
    for e in events {
        match e.level {
            1 => by_level.critical += 1,
            2 => by_level.error += 1,
            3 => by_level.warning += 1,
            _ => by_level.info += 1,
        }
        *cats.entry(e.category.as_str()).or_insert(0) += 1;
        let day = local_midnight(e.ts);
        let d = days.entry(day).or_insert_with(|| DayStats { day, ..Default::default() });
        match e.level {
            1 => d.critical += 1,
            2 => d.error += 1,
            3 => d.warning += 1,
            _ => {}
        }
    }
    let mut by_category: Vec<CategoryCount> = cats
        .into_iter()
        .map(|(c, n)| CategoryCount { code: c.to_string(), label: category_label(c).to_string(), count: n })
        .collect();
    by_category.sort_by(|a, b| b.count.cmp(&a.count).then(a.code.cmp(&b.code)));
    EventStats { by_level, by_category, by_day: days.into_values().collect() }
}

pub fn compute_notable(events: &[EventItem]) -> Notable {
    let mut n = Notable::default();
    for e in events {
        match e.category.as_str() {
            "unexpected_shutdown" => n.unexpected_shutdowns += 1,
            "app_crash" => n.app_crashes += 1,
            "app_hang" => n.app_hangs += 1,
            "driver" => n.driver_issues += 1,
            "disk" => n.disk_issues += 1,
            "service" => n.service_failures += 1,
            "update" if e.level <= 3 => n.update_failures += 1,
            "bsod" => n.bsod += 1,
            _ => {}
        }
    }
    n
}

/// (sağlayıcı, olay kimliği) bazında tekrarlayan örüntüler: en az 3 kez
/// veya en az 2 farklı günde görülenler, sayıya göre sıralı.
pub fn compute_patterns(events: &[EventItem]) -> Vec<EventPattern> {
    struct Acc<'a> {
        first: &'a EventItem,
        latest: &'a EventItem,
        count: u32,
        level: u8,
        days: HashSet<i64>,
    }
    let mut groups: HashMap<(&str, u32), Acc> = HashMap::new();
    for e in events {
        let day = local_midnight(e.ts);
        let a = groups.entry((e.provider.as_str(), e.event_id)).or_insert_with(|| Acc {
            first: e,
            latest: e,
            count: 0,
            level: e.level,
            days: HashSet::new(),
        });
        a.count += 1;
        a.level = a.level.min(e.level);
        a.days.insert(day);
        if e.ts < a.first.ts {
            a.first = e;
        }
        if e.ts > a.latest.ts {
            a.latest = e;
        }
    }
    let mut out: Vec<EventPattern> = groups
        .into_iter()
        .filter(|(_, a)| a.count >= 3 || a.days.len() >= 2)
        .map(|((provider, event_id), a)| {
            let category = a.latest.category.clone();
            EventPattern {
                provider: provider.to_string(),
                event_id,
                category_label: category_label(&category).to_string(),
                interpretation: interpretation(provider, event_id, &category).to_string(),
                category,
                level: a.level,
                count: a.count,
                first_ts: a.first.ts,
                last_ts: a.latest.ts,
                distinct_days: a.days.len() as u32,
                sample_message: truncate_chars(&a.latest.message, 300),
            }
        })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then(a.level.cmp(&b.level)).then(b.last_ts.cmp(&a.last_ts)));
    out
}

// ---------------------------------------------------------------------------
// Doğrulama ve sorgu
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryParams {
    pub days: u32,
    pub levels: Vec<u8>,
    pub logs: Vec<String>,
    pub max: u32,
}

pub fn validate(days: u32, levels: Vec<u8>, logs: Vec<String>, max: u32) -> AppResult<QueryParams> {
    if !(1..=90).contains(&days) {
        return Err(AppError::InvalidInput("gün sayısı 1 ile 90 arasında olmalı".into()));
    }
    if !(100..=5000).contains(&max) {
        return Err(AppError::InvalidInput("en fazla olay sayısı 100 ile 5000 arasında olmalı".into()));
    }
    let mut lv: Vec<u8> = levels;
    lv.sort_unstable();
    lv.dedup();
    if lv.is_empty() || lv.iter().any(|l| !(1..=4).contains(l)) {
        return Err(AppError::InvalidInput("düzeyler 1 (Kritik), 2 (Hata), 3 (Uyarı) veya 4 (Bilgi) olmalı".into()));
    }
    let mut lg: Vec<String> = Vec::new();
    for l in logs {
        let Some(a) = ALLOWED_LOGS.iter().find(|a| a.eq_ignore_ascii_case(&l)) else {
            return Err(AppError::InvalidInput("günlük adı geçersiz (System veya Application olmalı)".into()));
        };
        if !lg.iter().any(|x| x == a) {
            lg.push(a.to_string());
        }
    }
    if lg.is_empty() {
        return Err(AppError::InvalidInput("en az bir günlük seçilmeli".into()));
    }
    lg.sort();
    Ok(QueryParams { days, levels: lv, logs: lg, max })
}

fn levels_env(levels: &[u8]) -> String {
    let mut v: Vec<String> = levels.iter().map(|l| l.to_string()).collect();
    // Bilgi seçildiyse "her zaman kaydet" (0) düzeyindeki olaylar da dahil.
    if levels.contains(&4) {
        v.push("0".into());
    }
    v.join(",")
}

fn run_query(env: &[(&str, &str)]) -> AppResult<Vec<EventItem>> {
    let out = super::run_powershell(QUERY_SCRIPT, env, "olay günlüğü")?;
    parse_events(&out)
}

fn cache() -> &'static Mutex<Option<(QueryParams, Instant, EventsReport)>> {
    static C: OnceLock<Mutex<Option<(QueryParams, Instant, EventsReport)>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(None))
}

pub fn build_report(params: &QueryParams, events: Vec<EventItem>) -> EventsReport {
    let truncated = events.len() as u32 >= params.max;
    let stats = compute_stats(&events);
    let patterns = compute_patterns(&events);
    let notable = compute_notable(&events);
    let mut note = format!(
        "Son {} günün olayları gösteriliyor. Kategoriler ve yorumlar bilgilendirme amaçlıdır; olaylar arasında neden-sonuç ilişkisi olduğu anlamına gelmez.",
        params.days
    );
    if truncated {
        note.push_str(&format!(" Sonuçlar en yeni {} olayla sınırlandı; daha eski olaylar gösterilmiyor.", params.max));
    }
    if events.is_empty() {
        note = format!("Son {} günde seçilen düzeylerde olay bulunamadı.", params.days);
    }
    EventsReport {
        events,
        truncated,
        stats,
        patterns,
        notable,
        queried_at: chrono::Utc::now().timestamp(),
        days: params.days,
        note,
    }
}

pub fn query(days: u32, levels: Vec<u8>, logs: Vec<String>, max: u32) -> AppResult<EventsReport> {
    let params = validate(days, levels, logs, max)?;
    if let Some((p, at, r)) = cache().lock().as_ref() {
        if *p == params && at.elapsed().as_secs() < 60 {
            return Ok(r.clone());
        }
    }
    let logs_s = params.logs.join(",");
    let levels_s = levels_env(&params.levels);
    let max_s = params.max.to_string();
    let days_s = params.days.to_string();
    let events = run_query(&[
        ("IVILY_EV_LOGS", &logs_s),
        ("IVILY_EV_LEVELS", &levels_s),
        ("IVILY_EV_MAX", &max_s),
        ("IVILY_EV_DAYS", &days_s),
    ])?;
    let report = build_report(&params, events);
    *cache().lock() = Some((params, Instant::now(), report.clone()));
    Ok(report)
}

pub fn context(ts: i64, window_minutes: u32) -> AppResult<EventsContext> {
    if !(1..=120).contains(&window_minutes) {
        return Err(AppError::InvalidInput("zaman aralığı 1 ile 120 dakika arasında olmalı".into()));
    }
    let now = chrono::Utc::now().timestamp();
    // 2000-01-01 ile şimdiden bir gün sonrası arasında olmalı.
    if !(946_684_800..=now + 86_400).contains(&ts) {
        return Err(AppError::InvalidInput("zaman damgası geçersiz".into()));
    }
    let w = window_minutes as i64 * 60;
    let (from, to) = (ts - w, ts + w);
    let from_s = from.to_string();
    let to_s = to.to_string();
    let events = run_query(&[
        ("IVILY_EV_LOGS", "System,Application"),
        ("IVILY_EV_LEVELS", "1,2,3,4,0"),
        ("IVILY_EV_MAX", "300"),
        ("IVILY_EV_FROM", &from_s),
        ("IVILY_EV_TO", &to_s),
    ])?;
    Ok(EventsContext {
        from,
        to,
        events,
        note: "Bu olaylar aynı zaman aralığında kaydedildi. Aralarında neden-sonuç ilişkisi olduğu kanıtlanmamıştır.".into(),
    })
}

// ---------------------------------------------------------------------------
// Dışa aktarma
// ---------------------------------------------------------------------------

fn fmt_local(ts: i64) -> String {
    Local.timestamp_opt(ts, 0).single().map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_else(|| ts.to_string())
}

fn csv_field(s: &str) -> String {
    let clean = s.replace("\r\n", " ").replace(['\r', '\n'], " ");
    // Formül enjeksiyonuna karşı (Excel): =, +, -, @ ile başlayan değerler.
    let clean = if clean.starts_with(['=', '+', '-', '@']) { format!("'{clean}") } else { clean };
    format!("\"{}\"", clean.replace('"', "\"\""))
}

pub fn to_csv(r: &EventsReport) -> String {
    let mut out = String::from("\u{feff}");
    out.push_str("Zaman;Günlük;Düzey;Kaynak;Olay Kimliği;Kategori;İleti\r\n");
    for e in &r.events {
        let row = [
            csv_field(&fmt_local(e.ts)),
            csv_field(&e.log),
            csv_field(&e.level_label),
            csv_field(&e.provider),
            e.event_id.to_string(),
            csv_field(&e.category_label),
            csv_field(&e.message),
        ];
        out.push_str(&row.join(";"));
        out.push_str("\r\n");
    }
    out
}

pub fn to_txt(r: &EventsReport) -> String {
    let mut o = String::new();
    o.push_str("IVILY OPTIMIZER — SİSTEM OLAYLARI RAPORU\r\n");
    o.push_str(&format!("Oluşturulma: {}\r\n", fmt_local(r.queried_at)));
    o.push_str(&format!("Dönem: son {} gün\r\n", r.days));
    o.push_str(&format!("Not: {}\r\n\r\n", r.note));
    o.push_str("ÖZET\r\n");
    let l = &r.stats.by_level;
    o.push_str(&format!(
        "  Toplam olay: {} (Kritik: {}, Hata: {}, Uyarı: {}, Bilgi: {})\r\n",
        r.events.len(),
        l.critical,
        l.error,
        l.warning,
        l.info
    ));
    let n = &r.notable;
    o.push_str(&format!("  Beklenmedik kapanma: {}\r\n", n.unexpected_shutdowns));
    o.push_str(&format!("  Mavi ekran: {}\r\n", n.bsod));
    o.push_str(&format!("  Uygulama çökmesi: {}\r\n", n.app_crashes));
    o.push_str(&format!("  Uygulama yanıt vermedi: {}\r\n", n.app_hangs));
    o.push_str(&format!("  Sürücü sorunu: {}\r\n", n.driver_issues));
    o.push_str(&format!("  Disk sorunu: {}\r\n", n.disk_issues));
    o.push_str(&format!("  Hizmet sorunu: {}\r\n", n.service_failures));
    o.push_str(&format!("  Güncelleme sorunu: {}\r\n\r\n", n.update_failures));
    o.push_str("KATEGORİLER\r\n");
    for c in &r.stats.by_category {
        o.push_str(&format!("  {}: {}\r\n", c.label, c.count));
    }
    o.push_str("\r\nTEKRARLAYAN OLAYLAR\r\n");
    if r.patterns.is_empty() {
        o.push_str("  Tekrarlayan olay bulunamadı.\r\n");
    }
    for p in &r.patterns {
        o.push_str(&format!(
            "  - {} / {} ({}, {}): {} kez, {} farklı gün, ilk {} — son {}\r\n",
            p.provider,
            p.event_id,
            p.category_label,
            level_label(p.level),
            p.count,
            p.distinct_days,
            fmt_local(p.first_ts),
            fmt_local(p.last_ts)
        ));
        o.push_str(&format!("    Ivily yorumu: {}\r\n", p.interpretation));
        if !p.sample_message.is_empty() {
            o.push_str(&format!("    Örnek ileti: {}\r\n", p.sample_message.replace(['\r', '\n'], " ")));
        }
    }
    o.push_str("\r\nOLAYLAR (en yeniden en eskiye)\r\n");
    for e in &r.events {
        o.push_str(&format!(
            "[{}] {} | {} | {} | Olay {} | {}\r\n",
            fmt_local(e.ts),
            e.level_label,
            e.log,
            e.provider,
            e.event_id,
            e.category_label
        ));
        if !e.message.is_empty() {
            o.push_str(&format!("    {}\r\n", e.message.replace("\r\n", "\r\n    ").replace('\n', "\r\n    ")));
        }
    }
    o.push_str("\r\nYorumlar Ivily tarafından bilgilendirme amacıyla eklenmiştir; kesin teşhis değildir.\r\n");
    o
}

fn export_dir() -> std::path::PathBuf {
    dirs::download_dir().or_else(dirs::desktop_dir).unwrap_or_else(crate::paths::data_dir)
}

pub fn export(days: u32, levels: Vec<u8>, logs: Vec<String>, max: u32, format: &str) -> AppResult<String> {
    let ext = match format {
        "csv" | "txt" | "json" => format,
        _ => return Err(AppError::InvalidInput("biçim geçersiz (csv, txt veya json olmalı)".into())),
    };
    let r = query(days, levels, logs, max)?;
    let body = match ext {
        "csv" => to_csv(&r),
        "txt" => to_txt(&r),
        _ => serde_json::to_string_pretty(&r)?,
    };
    let dir = export_dir();
    std::fs::create_dir_all(&dir).map_err(|e| AppError::io(e, "dışa aktarma klasörü oluşturulamadı"))?;
    let name = format!("ivily-olaylar-{}.{ext}", Local::now().format("%Y%m%d-%H%M%S"));
    let path = dir.join(name);
    std::fs::write(&path, body.as_bytes()).map_err(|e| AppError::io(e, "dışa aktarma dosyası yazılamadı"))?;
    Ok(path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(ts: i64, provider: &str, id: u32, level: u8, msg: &str) -> EventItem {
        let category = categorize(provider, id, msg);
        EventItem {
            id: format!("System:{ts}"),
            ts,
            log: "System".into(),
            level,
            level_label: level_label(level).into(),
            provider: provider.into(),
            event_id: id,
            message: msg.into(),
            category: category.into(),
            category_label: category_label(category).into(),
        }
    }

    #[test]
    fn category_mapping() {
        assert_eq!(categorize("Microsoft-Windows-Kernel-Power", 41, ""), "unexpected_shutdown");
        assert_eq!(categorize("EventLog", 6008, ""), "unexpected_shutdown");
        assert_eq!(categorize("EventLog", 6005, ""), "other");
        assert_eq!(categorize("Microsoft-Windows-WER-SystemErrorReporting", 1001, ""), "bsod");
        assert_eq!(categorize("Windows Error Reporting", 1001, "Olay Adı: BlueScreen"), "bsod");
        assert_eq!(categorize("Windows Error Reporting", 1001, "Event Name: APPCRASH"), "app_crash");
        assert_eq!(categorize("Application Error", 1000, ""), "app_crash");
        assert_eq!(categorize(".NET Runtime", 1026, ""), "app_crash");
        assert_eq!(categorize("Application Hang", 1002, ""), "app_hang");
        assert_eq!(categorize("disk", 153, ""), "disk");
        assert_eq!(categorize("Ntfs", 55, ""), "disk");
        assert_eq!(categorize("stornvme", 129, ""), "disk");
        assert_eq!(categorize("Display", 4101, ""), "driver");
        assert_eq!(categorize("nvlddmkm", 13, ""), "driver");
        assert_eq!(categorize("Microsoft-Windows-WHEA-Logger", 17, ""), "hardware");
        assert_eq!(categorize("Service Control Manager", 7031, ""), "service");
        assert_eq!(categorize("Service Control Manager", 7036, ""), "other");
        assert_eq!(categorize("Microsoft-Windows-WindowsUpdateClient", 20, ""), "update");
        assert_eq!(categorize("Microsoft-Windows-DNS-Client", 1014, ""), "network");
        assert_eq!(categorize("Microsoft-Windows-Windows Defender", 1116, ""), "defender");
        assert_eq!(categorize("Microsoft-Windows-Kernel-General", 12, ""), "boot");
        assert_eq!(categorize("User32", 1074, ""), "shutdown_planned");
        assert_eq!(category_label("bsod"), "Mavi ekran (sistem hatası)");
        assert!(interpretation("Microsoft-Windows-Kernel-Power", 41, "unexpected_shutdown").starts_with("Bilgisayar düzgün kapatılmadan"));
        assert_eq!(interpretation("Foo", 1, "other"), UNKNOWN_INTERPRETATION);
    }

    #[test]
    fn pattern_grouping() {
        let day = 86_400;
        let base = 1_700_000_000;
        let events = vec![
            ev(base, "Microsoft-Windows-Kernel-Power", 41, 1, "a"),
            ev(base + 2 * day, "Microsoft-Windows-Kernel-Power", 41, 1, "son"),
            ev(base + 10, "Foo", 5, 3, "x"),
            ev(base + 20, "Foo", 5, 2, "x"),
            ev(base + 30, "Foo", 5, 3, "y"),
            ev(base + 40, "Bar", 9, 2, "tek"),
        ];
        let p = compute_patterns(&events);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].provider, "Foo");
        assert_eq!(p[0].count, 3);
        assert_eq!(p[0].level, 2);
        assert_eq!(p[0].interpretation, UNKNOWN_INTERPRETATION);
        let kp = &p[1];
        assert_eq!(kp.count, 2);
        assert_eq!(kp.distinct_days, 2);
        assert_eq!(kp.first_ts, base);
        assert_eq!(kp.last_ts, base + 2 * day);
        assert_eq!(kp.sample_message, "son");
        assert_eq!(kp.category, "unexpected_shutdown");

        let s = compute_stats(&events);
        assert_eq!(s.by_level.critical, 2);
        assert_eq!(s.by_level.error, 2);
        assert_eq!(s.by_level.warning, 2);
        assert!(s.by_day.len() >= 2);
        assert_eq!(s.by_day.iter().map(|d| d.critical + d.error + d.warning).sum::<u32>(), 6);
        let n = compute_notable(&events);
        assert_eq!(n.unexpected_shutdowns, 2);
        let r = build_report(&QueryParams { days: 7, levels: vec![1, 2, 3], logs: vec!["System".into()], max: 100 }, events);
        assert!(!r.truncated);
        let txt = to_txt(&r);
        assert!(txt.contains("Ivily yorumu:"));
        let csv = to_csv(&r);
        assert!(csv.starts_with('\u{feff}'));
        assert_eq!(csv.lines().count(), 7);
    }

    #[test]
    fn json_parsing() {
        assert!(parse_events("").unwrap().is_empty());
        assert!(parse_events("null").unwrap().is_empty());
        assert!(parse_events("[]").unwrap().is_empty());
        let one = r#"{"t":1700000000,"g":"System","v":1,"p":"Microsoft-Windows-Kernel-Power","i":41,"r":12345,"m":null}"#;
        let v = parse_events(one).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].id, "System:12345");
        assert_eq!(v[0].message, "");
        assert_eq!(v[0].category, "unexpected_shutdown");
        assert_eq!(v[0].level_label, "Kritik");
        let long = "ğ".repeat(5000);
        let arr = format!(
            r#"[{{"t":1,"g":"Application","v":0,"p":"User32","i":1074,"r":1,"m":"{long}"}},{{"t":5,"g":"System","v":2,"p":"x","i":7,"r":2,"m":"b"}},{{"g":"System"}}]"#
        );
        let v = parse_events(&arr).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].ts, 5);
        assert_eq!(v[1].level, 4);
        assert_eq!(v[1].message.chars().count(), 4000);
        assert!(parse_events("{bozuk").is_err());
    }

    #[test]
    fn validation() {
        assert!(validate(0, vec![1], vec!["System".into()], 100).is_err());
        assert!(validate(91, vec![1], vec!["System".into()], 100).is_err());
        assert!(validate(7, vec![], vec!["System".into()], 100).is_err());
        assert!(validate(7, vec![5], vec!["System".into()], 100).is_err());
        assert!(validate(7, vec![1], vec!["Security".into()], 100).is_err());
        assert!(validate(7, vec![1], vec!["System,Security".into()], 100).is_err());
        assert!(validate(7, vec![1], vec![], 100).is_err());
        assert!(validate(7, vec![1], vec!["System".into()], 99).is_err());
        assert!(validate(7, vec![1], vec!["System".into()], 5001).is_err());
        let p = validate(7, vec![3, 1, 1], vec!["application".into(), "System".into()], 300).unwrap();
        assert_eq!(p.levels, vec![1, 3]);
        assert_eq!(p.logs, vec!["Application", "System"]);
        assert_eq!(levels_env(&[1, 4]), "1,4,0");
        assert!(context(1_700_000_000, 0).is_err());
        assert!(context(5, 10).is_err());
        assert!(export(7, vec![1], vec!["System".into()], 100, "exe").is_err());
        assert_eq!(csv_field("=cmd"), "\"'=cmd\"");
        assert_eq!(csv_field("a\"b\nc"), "\"a\"\"b c\"");
    }

    #[cfg(windows)]
    #[test]
    fn query_readonly_integration() {
        let r = query(2, vec![1, 2, 3], vec!["System".into(), "Application".into()], 300).unwrap();
        println!(
            "olay sayısı: {} (kısaltıldı: {}), örüntü: {}, kategoriler: {:?}",
            r.events.len(),
            r.truncated,
            r.patterns.len(),
            r.stats.by_category.iter().map(|c| (&c.code, c.count)).collect::<Vec<_>>()
        );
        // Önbellekten
        let r2 = query(2, vec![1, 2, 3], vec!["System".into(), "Application".into()], 300).unwrap();
        assert_eq!(r.queried_at, r2.queried_at);
        let c = context(chrono::Utc::now().timestamp(), 60).unwrap();
        println!("bağlam olayları: {}", c.events.len());
    }
}
