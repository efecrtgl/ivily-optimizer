//! Zamanlanmış görevler.
//!
//! Listeleme sabit bir PowerShell betiğiyle (`Get-ScheduledTask`) yapılır.
//! `\Microsoft\Windows\` altındaki Windows bakım görevleri salt okunurdur.

use super::{validate_task_name, validate_task_path, ChangeResult};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::oplog::{self, Outcome};
use serde::Serialize;
use serde_json::Value;

/// Görev listesi betiği. Kullanıcı girdisi içermez; tek bir görevi sorgulamak
/// için yol ve ad `IVILY_TASK_PATH` / `IVILY_TASK_NAME` ortam
/// değişkenlerinden okunur ve tam eşitlikle (joker karakter olmadan)
/// karşılaştırılır.
const LIST_SCRIPT: &str = r#"[Console]::OutputEncoding=[System.Text.Encoding]::UTF8;
$ErrorActionPreference='SilentlyContinue';
$p=$env:IVILY_TASK_PATH; $n=$env:IVILY_TASK_NAME;
$all=@(Get-ScheduledTask);
if ($p) { $all=@($all | Where-Object { $_.TaskPath -eq $p -and $_.TaskName -eq $n }) }
function D($d){ if($d -and $d.Year -gt 2000){ $d.ToString('o') } else { $null } }
$r=@($all | ForEach-Object {
  $t=$_; $i=$null;
  try { $i = $t | Get-ScheduledTaskInfo -ErrorAction Stop } catch { $i=$null }
  [pscustomobject]@{
    TaskPath=$t.TaskPath; TaskName=$t.TaskName; State=[string]$t.State;
    Description=$t.Description; Author=$t.Author;
    Actions=@($t.Actions | ForEach-Object { [pscustomobject]@{ Execute=$_.Execute; Arguments=$_.Arguments } });
    Triggers=@($t.Triggers | ForEach-Object { [pscustomobject]@{ Kind=$_.CimClass.CimClassName; StartBoundary=$_.StartBoundary; Enabled=$_.Enabled } });
    HasInfo=($null -ne $i);
    LastRunTime=$(if($i){D $i.LastRunTime}else{$null});
    NextRunTime=$(if($i){D $i.NextRunTime}else{$null});
    LastTaskResult=$(if($i){$i.LastTaskResult}else{$null})
  }
});
ConvertTo-Json -InputObject $r -Depth 4 -Compress"#;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskAction {
    pub execute: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskTrigger {
    /// CIM sınıf adı (ör. MSFT_TaskLogonTrigger).
    pub kind: String,
    /// Türkçe açıklama (ör. "Oturum açıldığında").
    pub label: String,
    pub start_boundary: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInfo {
    /// `yol + ad` (ör. `\Vendor\Updater`).
    pub id: String,
    pub path: String,
    pub name: String,
    /// ready, running, disabled, queued, unknown
    pub state: String,
    pub state_label: String,
    pub enabled: bool,
    pub description: Option<String>,
    pub author: Option<String>,
    pub actions: Vec<TaskAction>,
    pub triggers: Vec<TaskTrigger>,
    /// Okunabilir tetikleyici özeti (ör. "Oturum açıldığında; Günlük").
    pub trigger_summary: String,
    pub last_run_time: Option<String>,
    pub next_run_time: Option<String>,
    pub last_task_result: Option<i64>,
    /// Son çalıştırma bilgisi alınabildi mi?
    pub has_run_info: bool,
    pub protected: bool,
    pub protected_reason: Option<String>,
}

pub fn trigger_label(kind: &str) -> &'static str {
    match kind {
        "MSFT_TaskLogonTrigger" => "Oturum açıldığında",
        "MSFT_TaskBootTrigger" => "Sistem başlatıldığında",
        "MSFT_TaskDailyTrigger" => "Günlük",
        "MSFT_TaskWeeklyTrigger" => "Haftalık",
        "MSFT_TaskTimeTrigger" => "Belirli bir zamanda",
        "MSFT_TaskIdleTrigger" => "Bilgisayar boştayken",
        "MSFT_TaskEventTrigger" => "Bir olay gerçekleştiğinde",
        "MSFT_TaskSessionStateChangeTrigger" => "Oturum durumu değiştiğinde",
        _ => "Diğer",
    }
}

pub fn state_code(s: &str) -> &'static str {
    match s {
        "Ready" => "ready",
        "Running" => "running",
        "Disabled" => "disabled",
        "Queued" => "queued",
        _ => "unknown",
    }
}

pub fn state_label(code: &str) -> &'static str {
    match code {
        "ready" => "Hazır",
        "running" => "Çalışıyor",
        "disabled" => "Devre dışı",
        "queued" => "Kuyrukta",
        _ => "Bilinmiyor",
    }
}

const PROTECTED_PREFIX: &str = "\\Microsoft\\Windows\\";
const PROTECTED_REASON: &str =
    "Bu bir Windows bakım görevidir; sistemin düzgün çalışması için Windows tarafından yönetilir ve yalnızca görüntülenebilir.";

pub fn is_protected_path(path: &str) -> bool {
    path.len() >= PROTECTED_PREFIX.len() && path[..PROTECTED_PREFIX.len()].eq_ignore_ascii_case(PROTECTED_PREFIX)
}

fn s(v: &Value, k: &str) -> Option<String> {
    match v.get(k) {
        Some(Value::String(s)) if !s.trim().is_empty() => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

fn arr<'a>(v: &'a Value, k: &str) -> Vec<&'a Value> {
    match v.get(k) {
        Some(Value::Array(a)) => a.iter().collect(),
        Some(Value::Null) | None => Vec::new(),
        Some(o) => vec![o],
    }
}

/// Betik çıktısını ayrıştırır (tek nesne, dizi veya boş).
pub fn parse_tasks(json: &str) -> AppResult<Vec<TaskInfo>> {
    let items = super::json_array(json).map_err(|e| {
        tracing::warn!(error = %e, "görev listesi JSON'u ayrıştırılamadı");
        AppError::System("zamanlanmış görev listesi okunamadı (beklenmeyen çıktı)".into())
    })?;
    let mut out = Vec::with_capacity(items.len());
    for v in &items {
        let (Some(path), Some(name)) = (s(v, "TaskPath"), s(v, "TaskName")) else { continue };
        let state = state_code(s(v, "State").as_deref().unwrap_or(""));
        let triggers: Vec<TaskTrigger> = arr(v, "Triggers")
            .into_iter()
            .map(|t| {
                let kind = s(t, "Kind").unwrap_or_default();
                TaskTrigger {
                    label: trigger_label(&kind).to_string(),
                    kind,
                    start_boundary: s(t, "StartBoundary"),
                    enabled: t.get("Enabled").and_then(Value::as_bool).unwrap_or(true),
                }
            })
            .collect();
        let mut labels: Vec<&str> = Vec::new();
        for t in &triggers {
            if !labels.contains(&t.label.as_str()) {
                labels.push(&t.label);
            }
        }
        let trigger_summary =
            if labels.is_empty() { "Tetikleyici yok (yalnızca el ile)".to_string() } else { labels.join("; ") };
        let actions = arr(v, "Actions")
            .into_iter()
            .map(|a| TaskAction { execute: s(a, "Execute"), arguments: s(a, "Arguments") })
            .collect();
        let protected = is_protected_path(&path);
        out.push(TaskInfo {
            id: format!("{path}{name}"),
            enabled: state != "disabled",
            state: state.to_string(),
            state_label: state_label(state).to_string(),
            description: s(v, "Description").filter(|d| !d.starts_with("$(@")),
            author: s(v, "Author").filter(|d| !d.starts_with("$(@")),
            actions,
            trigger_summary,
            triggers,
            last_run_time: s(v, "LastRunTime"),
            next_run_time: s(v, "NextRunTime"),
            last_task_result: v.get("LastTaskResult").and_then(Value::as_i64),
            has_run_info: v.get("HasInfo").and_then(Value::as_bool).unwrap_or(false),
            protected,
            protected_reason: protected.then(|| PROTECTED_REASON.to_string()),
            path,
            name,
        });
    }
    out.sort_by_key(|a| a.id.to_lowercase());
    Ok(out)
}

pub fn list() -> AppResult<Vec<TaskInfo>> {
    let out = super::run_powershell(LIST_SCRIPT, &[], "zamanlanmış görev listesi")?;
    parse_tasks(&out)
}

fn query(path: &str, name: &str) -> AppResult<TaskInfo> {
    let out = super::run_powershell(
        LIST_SCRIPT,
        &[("IVILY_TASK_PATH", path), ("IVILY_TASK_NAME", name)],
        "zamanlanmış görev bilgisi",
    )?;
    parse_tasks(&out)?
        .into_iter()
        .find(|t| t.path.eq_ignore_ascii_case(path) && t.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| AppError::NotFound("zamanlanmış görev bulunamadı".into()))
}

/// Bir zamanlanmış görevi etkinleştirir veya devre dışı bırakır.
pub fn set_enabled(db: &Db, path: &str, name: &str, enabled: bool) -> AppResult<ChangeResult<TaskInfo>> {
    validate_task_path(path)?;
    validate_task_name(name)?;
    if is_protected_path(path) {
        return Err(AppError::Protected(PROTECTED_REASON.into()));
    }
    let before = query(path, name)?;
    if before.enabled == enabled {
        return Ok(ChangeResult {
            message: if enabled { "Görev zaten etkin." } else { "Görev zaten devre dışı." }.into(),
            item: before,
            elevated: false,
            oplog_id: None,
        });
    }
    let full = format!("{}{}", before.path, before.name);
    let flag = if enabled { "/ENABLE" } else { "/DISABLE" };
    let detail = serde_json::json!({
        "path": before.path,
        "name": before.name,
        "previousState": before.state,
        "previousEnabled": before.enabled,
        "requestedEnabled": enabled,
    });

    // Önce normal yetkiyle dene; olmazsa yalnızca bu komut için UAC iste.
    let mut elevated = false;
    let direct = crate::platform::hidden_command(&crate::platform::system_tool("schtasks.exe"))
        .args(["/Change", "/TN", &full, flag])
        .output();
    let ok = match direct {
        Ok(o) if o.status.success() => true,
        Ok(o) => {
            let msg = super::decode_console(&o.stderr);
            tracing::info!(stderr = %msg.trim(), "schtasks normal yetkiyle başarısız; yönetici onayı denenecek");
            false
        }
        Err(e) => return Err(AppError::io(e, "schtasks.exe çalıştırılamadı")),
    };
    if !ok {
        elevated = true;
        let args: Vec<String> = vec!["/Change".into(), "/TN".into(), full.clone(), flag.into()];
        let res = match crate::platform::run_elevated("schtasks.exe", &args) {
            Ok(0) => Ok(()),
            Ok(code) => Err(AppError::System(format!("görev değiştirilemedi (schtasks.exe çıkış kodu {code})"))),
            Err(e) => Err(e),
        };
        if let Err(e) = res {
            if !matches!(e, AppError::Cancelled) {
                let mut d = detail.clone();
                d["error"] = Value::String(e.to_string());
                let _ = oplog::record(
                    db,
                    "task",
                    &format!("Zamanlanmış görev değiştirilemedi: {full}"),
                    Outcome::Failed,
                    false,
                    d,
                );
            }
            return Err(e);
        }
    }

    let after = query(path, name)?;
    if after.enabled != enabled {
        return Err(AppError::System(format!(
            "komut çalıştı ancak görevin durumu değişmedi (şu an: {})",
            after.state_label
        )));
    }
    let oplog_id = oplog::record(
        db,
        "task",
        &format!("Zamanlanmış görev {}: {full}", if enabled { "etkinleştirildi" } else { "devre dışı bırakıldı" }),
        Outcome::Success,
        true,
        serde_json::json!({
            "path": after.path,
            "name": after.name,
            "previousState": before.state,
            "previousEnabled": before.enabled,
            "newState": after.state,
            "newEnabled": after.enabled,
            "elevated": elevated,
        }),
    )
    .ok();
    let mut message = if enabled {
        "Görev etkinleştirildi.".to_string()
    } else {
        "Görev devre dışı bırakıldı; silinmedi ve yeniden etkinleştirilebilir.".to_string()
    };
    if elevated {
        message.push_str(" Değişiklik yönetici onayıyla (UAC) uygulandı.");
    }
    Ok(ChangeResult { item: after, message, elevated, oplog_id })
}

// ---------------------------------------------------------------------------
// Görev geçmişi
// ---------------------------------------------------------------------------

/// Görev geçmişi betiği. Tam görev yolu `IVILY_TASK_FULL` ortam
/// değişkeninden okunur ve tam eşitlikle (joker karakter olmadan)
/// karşılaştırılır. Günlük kapalıysa hiçbir şey değiştirilmez.
const HISTORY_SCRIPT: &str = r#"[Console]::OutputEncoding=[System.Text.Encoding]::UTF8;
$ErrorActionPreference='SilentlyContinue';
$full=$env:IVILY_TASK_FULL;
$ln='Microsoft-Windows-TaskScheduler/Operational';
$enabled=$null;
try { $l=Get-WinEvent -ListLog $ln -ErrorAction Stop; $enabled=[bool]$l.IsEnabled } catch { $enabled=$null }
$out=New-Object System.Collections.ArrayList;
$err=$null;
if ($enabled -ne $false) {
  try {
    $evs=@(Get-WinEvent -LogName $ln -MaxEvents 4000 -ErrorAction Stop)
    foreach ($e in $evs) {
      $x=[xml]$e.ToXml(); $tn=$null; $rc=$null;
      foreach ($d in @($x.Event.EventData.Data)) {
        if ($d.Name -eq 'TaskName') { $tn=$d.'#text' }
        elseif ($d.Name -eq 'ResultCode') { $rc=$d.'#text' }
      }
      if ($tn -ne $null -and $tn -eq $full) {
        [void]$out.Add([pscustomobject]@{ t=([DateTimeOffset]$e.TimeCreated).ToUnixTimeSeconds(); id=$e.Id; rc=$rc })
        if ($out.Count -ge 30) { break }
      }
    }
  } catch {
    if ($_.FullyQualifiedErrorId -notmatch 'NoMatchingEventsFound') { $err=[string]$_.FullyQualifiedErrorId }
  }
}
ConvertTo-Json -InputObject ([pscustomobject]@{ enabled=$enabled; error=$err; events=@($out) }) -Depth 4 -Compress"#;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskHistoryEvent {
    pub ts: i64,
    pub event_id: u32,
    pub label: String,
    pub result_code: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskHistory {
    pub log_enabled: Option<bool>,
    pub events: Vec<TaskHistoryEvent>,
    pub note: String,
}

pub fn history_label(id: u32) -> &'static str {
    match id {
        100 => "Görev başlatıldı",
        101 => "Başlatma hatası",
        102 => "Görev tamamlandı",
        103 => "Başlatılamadı",
        110 => "Kullanıcı tarafından tetiklendi",
        111 => "Sonlandırıldı",
        200 => "Eylem başlatıldı",
        201 => "Eylem tamamlandı",
        _ => "Diğer",
    }
}

fn parse_result_code(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => {
            let t = s.trim();
            if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                i64::from_str_radix(h, 16).ok()
            } else {
                t.parse::<i64>().ok()
            }
        }
        _ => None,
    }
}

const NOTE_DISABLED: &str = "Windows, zamanlanmış görev geçmişini varsayılan olarak kapalı tutar; bu nedenle geçmiş kayıt bulunmuyor. Ivily bu ayarı değiştirmez. İsterseniz Görev Zamanlayıcı'daki \"Tüm Görevlerin Geçmişini Etkinleştir\" seçeneğiyle açabilirsiniz.";

/// Geçmiş betiği çıktısını ayrıştırır.
pub fn parse_history(json: &str) -> AppResult<TaskHistory> {
    let t = json.trim().trim_start_matches('\u{feff}');
    let v: Value = serde_json::from_str(if t.is_empty() { "{}" } else { t }).map_err(|e| {
        tracing::warn!(error = %e, "görev geçmişi JSON'u ayrıştırılamadı");
        AppError::System("görev geçmişi okunamadı (beklenmeyen çıktı)".into())
    })?;
    let log_enabled = v.get("enabled").and_then(Value::as_bool);
    let error = s(&v, "error");
    let mut events: Vec<TaskHistoryEvent> = arr(&v, "events")
        .into_iter()
        .filter_map(|e| {
            let ts = e.get("t").and_then(Value::as_i64)?;
            let id = e.get("id").and_then(Value::as_u64)? as u32;
            Some(TaskHistoryEvent {
                ts,
                event_id: id,
                label: history_label(id).to_string(),
                result_code: e.get("rc").and_then(parse_result_code),
            })
        })
        .collect();
    events.sort_by_key(|e| std::cmp::Reverse(e.ts));
    events.truncate(30);
    let note = if log_enabled == Some(false) {
        NOTE_DISABLED.to_string()
    } else if error.is_some() {
        "Görev geçmişi okunamadı; bu günlüğe erişim izni olmayabilir.".to_string()
    } else if events.is_empty() {
        "Bu görev için kayıtlı geçmiş bulunamadı. Görev son zamanlarda çalışmamış veya eski kayıtlar silinmiş olabilir."
            .to_string()
    } else {
        format!("Bu görev için son {} olay gösteriliyor.", events.len())
    };
    Ok(TaskHistory { log_enabled, events, note })
}

/// Bir görevin Görev Zamanlayıcı geçmişini okur (salt okunur).
pub fn history(path: &str, name: &str) -> AppResult<TaskHistory> {
    validate_task_path(path)?;
    validate_task_name(name)?;
    let full = format!("{path}{name}");
    let out = super::run_powershell(HISTORY_SCRIPT, &[("IVILY_TASK_FULL", &full)], "görev geçmişi")?;
    parse_history(&out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_parsing() {
        let h = parse_history(r#"{"enabled":false,"error":null,"events":[]}"#).unwrap();
        assert_eq!(h.log_enabled, Some(false));
        assert!(h.events.is_empty());
        assert!(h.note.contains("varsayılan olarak kapalı"));
        let h =
            parse_history(r#"{"enabled":true,"error":null,"events":{"t":1700000000,"id":102,"rc":"0x0"}}"#).unwrap();
        assert_eq!(h.events.len(), 1);
        assert_eq!(h.events[0].label, "Görev tamamlandı");
        assert_eq!(h.events[0].result_code, Some(0));
        let h = parse_history(r#"{"enabled":true,"events":[{"t":1,"id":100,"rc":null},{"t":5,"id":201,"rc":"2147942402"},{"t":3,"id":329}]}"#).unwrap();
        assert_eq!(h.events.iter().map(|e| e.ts).collect::<Vec<_>>(), vec![5, 3, 1]);
        assert_eq!(h.events[0].result_code, Some(2147942402));
        assert_eq!(h.events[1].label, "Diğer");
        assert_eq!(parse_history("").unwrap().log_enabled, None);
        assert!(parse_history("<html>").is_err());
        assert!(history("Vendor\\", "x").is_err());
        assert!(history("\\Vendor\\", "a\"b").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn history_readonly() {
        let h = history("\\Microsoft\\Windows\\Defrag\\", "ScheduledDefrag").unwrap();
        println!("görev geçmişi: enabled={:?} olay={} not={}", h.log_enabled, h.events.len(), h.note);
    }

    const SINGLE: &str = r#"{"TaskPath":"\\Vendor\\","TaskName":"Updater","State":"Ready","Description":"Güncellemeleri denetler","Author":"Vendor Inc.","Actions":[{"Execute":"C:\\Program Files\\Vendor\\upd.exe","Arguments":"/check"}],"Triggers":[{"Kind":"MSFT_TaskLogonTrigger","StartBoundary":null,"Enabled":true},{"Kind":"MSFT_TaskDailyTrigger","StartBoundary":"2024-01-01T10:00:00","Enabled":true}],"HasInfo":true,"LastRunTime":"2026-11-24T10:00:00.0000000+03:00","NextRunTime":null,"LastTaskResult":0}"#;

    #[test]
    fn parse_single_object() {
        let v = parse_tasks(SINGLE).unwrap();
        assert_eq!(v.len(), 1);
        let t = &v[0];
        assert_eq!(t.id, "\\Vendor\\Updater");
        assert_eq!(t.state, "ready");
        assert_eq!(t.state_label, "Hazır");
        assert!(t.enabled);
        assert!(!t.protected);
        assert_eq!(t.actions.len(), 1);
        assert_eq!(t.actions[0].arguments.as_deref(), Some("/check"));
        assert_eq!(t.triggers.len(), 2);
        assert_eq!(t.trigger_summary, "Oturum açıldığında; Günlük");
        assert_eq!(t.last_task_result, Some(0));
        assert!(t.has_run_info);
    }

    #[test]
    fn parse_array_and_edge_cases() {
        let json = r#"[{"TaskPath":"\\Microsoft\\Windows\\Defrag\\","TaskName":"ScheduledDefrag","State":"Disabled","Description":"$(@%systemroot%\\system32\\defragsvc.dll,-801)","Actions":{"Execute":"%windir%\\system32\\defrag.exe","Arguments":"-c"},"Triggers":null,"HasInfo":false},
                       {"TaskPath":"\\","TaskName":"Görev","State":"Running","Actions":[],"Triggers":[{"Kind":"MSFT_TaskFooTrigger","Enabled":false}]},
                       {"TaskName":"eksik yol"}]"#;
        let v = parse_tasks(json).unwrap();
        assert_eq!(v.len(), 2);
        let d = v.iter().find(|t| t.name == "ScheduledDefrag").unwrap();
        assert!(d.protected);
        assert!(!d.enabled);
        assert_eq!(d.state_label, "Devre dışı");
        assert!(d.description.is_none());
        assert_eq!(d.actions.len(), 1);
        assert_eq!(d.trigger_summary, "Tetikleyici yok (yalnızca el ile)");
        let g = v.iter().find(|t| t.name == "Görev").unwrap();
        assert_eq!(g.state, "running");
        assert_eq!(g.triggers[0].label, "Diğer");
        assert!(!g.triggers[0].enabled);
        assert_eq!(parse_tasks("").unwrap().len(), 0);
        assert_eq!(parse_tasks("[]").unwrap().len(), 0);
        assert!(parse_tasks("<html>").is_err());
    }

    #[test]
    fn labels_and_protection() {
        assert_eq!(trigger_label("MSFT_TaskBootTrigger"), "Sistem başlatıldığında");
        assert_eq!(trigger_label("MSFT_TaskSessionStateChangeTrigger"), "Oturum durumu değiştiğinde");
        assert_eq!(state_label(state_code("Queued")), "Kuyrukta");
        assert!(is_protected_path("\\Microsoft\\Windows\\UpdateOrchestrator\\"));
        assert!(is_protected_path("\\microsoft\\windows\\"));
        assert!(!is_protected_path("\\Microsoft\\Office\\"));
        assert!(!is_protected_path("\\"));
        let db = Db::open_in_memory().unwrap();
        assert!(matches!(
            set_enabled(&db, "\\Microsoft\\Windows\\Defrag\\", "ScheduledDefrag", false),
            Err(AppError::Protected(_))
        ));
        assert!(matches!(set_enabled(&db, "Vendor\\", "x", false), Err(AppError::InvalidInput(_))));
        assert!(matches!(set_enabled(&db, "\\Vendor\\", "a\"b", false), Err(AppError::InvalidInput(_))));
    }

    #[cfg(windows)]
    #[test]
    fn list_readonly() {
        let v = list().unwrap();
        assert!(!v.is_empty());
        assert!(v.iter().any(|t| t.protected));
    }
}
