//! Windows hizmetleri (Service Control Manager).
//!
//! Listeleme doğrudan Win32 SCM API'si ile yapılır. Değişiklikler yalnızca o
//! komut için UAC onayı isteyen `sc.exe` ile uygulanır; ardından hizmet
//! yeniden sorgulanır ve gerçek durum bildirilir.

use super::{validate_service_name, ChangeResult};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::oplog::{self, Outcome};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    /// running, stopped, start_pending, stop_pending, continue_pending,
    /// pause_pending, paused, unknown
    pub status: String,
    pub status_label: String,
    /// auto, auto_delayed, manual, disabled, boot, system, unknown
    pub start_type: String,
    pub start_type_label: String,
    pub binary_path: Option<String>,
    pub account: Option<String>,
    pub pid: Option<u32>,
    pub can_stop: bool,
    pub protected: bool,
    pub protected_reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Korunan hizmetler
// ---------------------------------------------------------------------------

const R_CORE: &str = "Windows'un temel bileşenlerinden biridir; değiştirilmesi sistemin açılmamasına veya kararsız çalışmasına yol açabilir.";
const R_SECURITY: &str = "Bilgisayarınızın güvenliği için gereklidir (güvenlik duvarı, virüs koruması, kimlik bilgileri veya şifreleme).";
const R_NETWORK: &str = "Ağ ve internet bağlantısı için gereklidir; değiştirilmesi bağlantının kesilmesine yol açabilir.";
const R_AUDIO: &str = "Ses aygıtlarının çalışması için gereklidir.";
const R_UPDATE: &str = "Windows güncellemeleri, lisans etkinleştirme veya yazılım kurulumu için gereklidir; değiştirilmesi güvenlik güncellemelerinin alınmasını engelleyebilir.";
const R_UAC: &str = "Yönetici izni (UAC) isteklerini işler; değiştirilmesi yönetici gerektiren işlemleri imkânsız hâle getirebilir.";

/// Korunan hizmetler ve gerekçeleri. Bu hizmetlerin başlangıç türü
/// değiştirilemez ve durdurulamaz (başlatılmalarına izin verilir).
const PROTECTED: &[(&str, &str)] = &[
    // Çekirdek altyapı
    ("RpcSs", R_CORE),
    ("RpcEptMapper", R_CORE),
    ("DcomLaunch", R_CORE),
    ("LSM", R_CORE),
    ("EventLog", R_CORE),
    ("PlugPlay", R_CORE),
    ("Power", R_CORE),
    ("ProfSvc", R_CORE),
    ("Schedule", R_CORE),
    ("Winmgmt", R_CORE),
    ("BrokerInfrastructure", R_CORE),
    ("CoreMessagingRegistrar", R_CORE),
    ("SystemEventsBroker", R_CORE),
    ("StateRepository", R_CORE),
    ("UserManager", R_CORE),
    ("gpsvc", R_CORE),
    ("TimeBrokerSvc", R_CORE),
    ("SENS", R_CORE),
    ("AppXSvc", R_CORE),
    ("camsvc", R_CORE),
    ("Appinfo", R_UAC),
    // Güvenlik
    ("SamSs", R_SECURITY),
    ("CryptSvc", R_SECURITY),
    ("BFE", R_SECURITY),
    ("MpsSvc", R_SECURITY),
    ("WinDefend", R_SECURITY),
    ("WdNisSvc", R_SECURITY),
    ("SecurityHealthService", R_SECURITY),
    ("wscsvc", R_SECURITY),
    ("SgrmBroker", R_SECURITY),
    ("KeyIso", R_SECURITY),
    ("VaultSvc", R_SECURITY),
    ("EFS", R_SECURITY),
    ("mpssvc", R_SECURITY),
    ("Sense", R_SECURITY),
    ("webthreatdefsvc", R_SECURITY),
    // Ağ
    ("Dhcp", R_NETWORK),
    ("Dnscache", R_NETWORK),
    ("nsi", R_NETWORK),
    ("NlaSvc", R_NETWORK),
    ("netprofm", R_NETWORK),
    ("WinHttpAutoProxySvc", R_NETWORK),
    // Ses
    ("AudioEndpointBuilder", R_AUDIO),
    ("Audiosrv", R_AUDIO),
    // Güncelleme, lisans ve kurulum
    ("wuauserv", R_UPDATE),
    ("UsoSvc", R_UPDATE),
    ("WaaSMedicSvc", R_UPDATE),
    ("BITS", R_UPDATE),
    ("TrustedInstaller", R_UPDATE),
    ("msiserver", R_UPDATE),
    ("sppsvc", R_UPDATE),
    ("ClipSVC", R_UPDATE),
    ("LicenseManager", R_UPDATE),
];

/// Bir hizmetin korunup korunmadığını ve gerekçesini döndürür (büyük/küçük
/// harf duyarsız).
pub fn protection(name: &str) -> Option<&'static str> {
    PROTECTED.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, r)| *r)
}

pub fn status_code(state: u32) -> &'static str {
    match state {
        1 => "stopped",
        2 => "start_pending",
        3 => "stop_pending",
        4 => "running",
        5 => "continue_pending",
        6 => "pause_pending",
        7 => "paused",
        _ => "unknown",
    }
}

pub fn status_label(code: &str) -> &'static str {
    match code {
        "stopped" => "Durduruldu",
        "start_pending" => "Başlatılıyor",
        "stop_pending" => "Durduruluyor",
        "running" => "Çalışıyor",
        "continue_pending" => "Sürdürülüyor",
        "pause_pending" => "Duraklatılıyor",
        "paused" => "Duraklatıldı",
        _ => "Bilinmiyor",
    }
}

pub fn start_type_code(raw: u32, delayed: bool) -> &'static str {
    match raw {
        0 => "boot",
        1 => "system",
        2 if delayed => "auto_delayed",
        2 => "auto",
        3 => "manual",
        4 => "disabled",
        _ => "unknown",
    }
}

pub fn start_type_label(code: &str) -> &'static str {
    match code {
        "auto" => "Otomatik",
        "auto_delayed" => "Otomatik (Gecikmeli başlatma)",
        "manual" => "El ile",
        "disabled" => "Devre dışı",
        "boot" => "Önyükleme",
        "system" => "Sistem",
        _ => "Bilinmiyor",
    }
}

/// Arayüzdeki başlangıç türünü `sc.exe config start=` değerine çevirir.
pub fn sc_start_value(start_type: &str) -> AppResult<&'static str> {
    match start_type {
        "auto" => Ok("auto"),
        "auto_delayed" => Ok("delayed-auto"),
        "manual" => Ok("demand"),
        "disabled" => Ok("disabled"),
        _ => Err(AppError::InvalidInput("başlangıç türü geçersiz (auto, auto_delayed, manual veya disabled olmalı)".into())),
    }
}

/// `sc.exe` çıkış kodlarını (Win32 hata kodları) Türkçe açıklamaya çevirir.
pub fn sc_error_message(code: u32) -> String {
    let m = match code {
        5 => "erişim reddedildi",
        1051 => "bu hizmete bağımlı çalışan başka hizmetler olduğu için durdurulamadı",
        1052 => "bu işlem hizmet için geçerli değil",
        1053 => "hizmet başlatma veya durdurma isteğine zamanında yanıt vermedi",
        1056 => "hizmet zaten çalışıyor",
        1058 => "hizmet devre dışı olduğu için başlatılamıyor; önce başlangıç türünü değiştirin",
        1060 => "böyle bir hizmet yok",
        1061 => "hizmet şu anda denetim isteklerini kabul etmiyor",
        1062 => "hizmet zaten çalışmıyor",
        1068 => "hizmetin bağımlı olduğu bir hizmet başlatılamadı",
        1072 => "hizmet silinmek üzere işaretlenmiş",
        _ => "",
    };
    if m.is_empty() {
        format!("işlem başarısız oldu (sc.exe çıkış kodu {code})")
    } else {
        format!("{m} (kod {code})")
    }
}

// ---------------------------------------------------------------------------
// Win32 SCM
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::sysmgmt::{pwstr_to_string, wide};
    use windows::core::PCWSTR;
    use windows::Win32::System::Services::*;

    struct Handle(SC_HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseServiceHandle(self.0);
            }
        }
    }

    fn scm() -> AppResult<Handle> {
        unsafe {
            OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT | SC_MANAGER_ENUMERATE_SERVICE)
                .map(Handle)
                .map_err(|e| AppError::System(format!("hizmet yöneticisine bağlanılamadı ({e})")))
        }
    }

    /// Hizalı bayt arabelleği (u64 dizisi üzerinde).
    fn aligned(bytes: usize) -> Vec<u64> {
        vec![0u64; bytes.div_ceil(8).max(1)]
    }
    fn as_bytes(v: &mut [u64]) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(v.as_mut_ptr() as *mut u8, v.len() * 8) }
    }

    struct Config {
        start_raw: u32,
        binary: Option<String>,
        account: Option<String>,
        display: Option<String>,
        description: Option<String>,
        delayed: bool,
    }

    fn query_config(scm: &Handle, name: &str) -> Option<Config> {
        unsafe {
            let w = wide(name);
            let svc = Handle(OpenServiceW(scm.0, PCWSTR(w.as_ptr()), SERVICE_QUERY_CONFIG).ok()?);
            let mut needed = 0u32;
            let _ = QueryServiceConfigW(svc.0, None, 0, &mut needed);
            if needed == 0 {
                return None;
            }
            let mut buf = aligned(needed as usize);
            let cfg_ptr = buf.as_mut_ptr() as *mut QUERY_SERVICE_CONFIGW;
            QueryServiceConfigW(svc.0, Some(cfg_ptr), needed, &mut needed).ok()?;
            let cfg = &*cfg_ptr;
            let nonempty = |s: String| (!s.is_empty()).then_some(s);
            let binary = nonempty(pwstr_to_string(cfg.lpBinaryPathName));
            let account = nonempty(pwstr_to_string(cfg.lpServiceStartName));
            let display = nonempty(pwstr_to_string(cfg.lpDisplayName));
            let start_raw = cfg.dwStartType.0;

            // Açıklama
            let mut description = None;
            let mut n2 = 0u32;
            let _ = QueryServiceConfig2W(svc.0, SERVICE_CONFIG_DESCRIPTION, None, &mut n2);
            if n2 > 0 {
                let mut b2 = aligned(n2 as usize);
                let bytes = as_bytes(&mut b2);
                if QueryServiceConfig2W(svc.0, SERVICE_CONFIG_DESCRIPTION, Some(&mut bytes[..n2 as usize]), &mut n2).is_ok() {
                    let d = &*(b2.as_ptr() as *const SERVICE_DESCRIPTIONW);
                    let s = pwstr_to_string(d.lpDescription);
                    // Çözümlenemeyen kaynak dizeleri ("@%SystemRoot%\...,-100") gösterilmez.
                    if !s.is_empty() && !s.starts_with('@') {
                        description = Some(s);
                    }
                }
            }
            // Gecikmeli otomatik başlatma
            let mut delayed = false;
            if start_raw == 2 {
                let mut b3 = aligned(16);
                let mut n3 = 0u32;
                let bytes = as_bytes(&mut b3);
                if QueryServiceConfig2W(svc.0, SERVICE_CONFIG_DELAYED_AUTO_START_INFO, Some(&mut bytes[..16]), &mut n3).is_ok() {
                    let d = &*(b3.as_ptr() as *const SERVICE_DELAYED_AUTO_START_INFO);
                    delayed = d.fDelayedAutostart.as_bool();
                }
            }
            Some(Config { start_raw, binary, account, display, description, delayed })
        }
    }

    fn make(scm: &Handle, name: String, display: String, sp: &SERVICE_STATUS_PROCESS) -> ServiceInfo {
        let cfg = query_config(scm, &name);
        let status = status_code(sp.dwCurrentState.0);
        let start = cfg.as_ref().map(|c| start_type_code(c.start_raw, c.delayed)).unwrap_or("unknown");
        let prot = protection(&name);
        ServiceInfo {
            display_name: if display.is_empty() { cfg.as_ref().and_then(|c| c.display.clone()).unwrap_or_else(|| name.clone()) } else { display },
            description: cfg.as_ref().and_then(|c| c.description.clone()),
            status: status.to_string(),
            status_label: status_label(status).to_string(),
            start_type: start.to_string(),
            start_type_label: start_type_label(start).to_string(),
            binary_path: cfg.as_ref().and_then(|c| c.binary.clone()),
            account: cfg.as_ref().and_then(|c| c.account.clone()),
            pid: (sp.dwProcessId != 0).then_some(sp.dwProcessId),
            can_stop: sp.dwControlsAccepted & SERVICE_ACCEPT_STOP != 0,
            protected: prot.is_some(),
            protected_reason: prot.map(|s| s.to_string()),
            name,
        }
    }

    pub fn list() -> AppResult<Vec<ServiceInfo>> {
        let scm = scm()?;
        let mut out = Vec::new();
        let mut resume = 0u32;
        let mut buf = aligned(64 * 1024);
        loop {
            let mut needed = 0u32;
            let mut count = 0u32;
            let r = unsafe {
                EnumServicesStatusExW(
                    scm.0,
                    SC_ENUM_PROCESS_INFO,
                    SERVICE_WIN32,
                    SERVICE_STATE_ALL,
                    Some(as_bytes(&mut buf)),
                    &mut needed,
                    &mut count,
                    Some(&mut resume),
                    PCWSTR::null(),
                )
            };
            let more = match &r {
                Ok(()) => false,
                Err(e) if e.code() == windows::Win32::Foundation::ERROR_MORE_DATA.to_hresult() => true,
                Err(e) => return Err(AppError::System(format!("hizmetler listelenemedi ({e})"))),
            };
            let entries = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW, count as usize) };
            for e in entries {
                let name = unsafe { pwstr_to_string(e.lpServiceName) };
                let display = unsafe { pwstr_to_string(e.lpDisplayName) };
                if name.is_empty() {
                    continue;
                }
                let sp = e.ServiceStatusProcess;
                out.push((name, display, sp));
            }
            if !more {
                break;
            }
            if count == 0 && (needed as usize) > buf.len() * 8 {
                buf = aligned(needed as usize);
            }
        }
        let mut v: Vec<ServiceInfo> = out.into_iter().map(|(n, d, sp)| make(&scm, n, d, &sp)).collect();
        v.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
        Ok(v)
    }

    pub fn query(name: &str) -> AppResult<ServiceInfo> {
        let scm = scm()?;
        unsafe {
            let w = wide(name);
            let svc = Handle(
                OpenServiceW(scm.0, PCWSTR(w.as_ptr()), SERVICE_QUERY_STATUS)
                    .map_err(|_| AppError::NotFound(format!("\"{name}\" adlı hizmet bulunamadı")))?,
            );
            let mut sp = SERVICE_STATUS_PROCESS::default();
            let mut needed = 0u32;
            let bytes = std::slice::from_raw_parts_mut(&mut sp as *mut _ as *mut u8, std::mem::size_of::<SERVICE_STATUS_PROCESS>());
            QueryServiceStatusEx(svc.0, SC_STATUS_PROCESS_INFO, Some(bytes), &mut needed)
                .map_err(|e| AppError::System(format!("hizmet durumu okunamadı ({e})")))?;
            drop(svc);
            Ok(make(&scm, name.to_string(), String::new(), &sp))
        }
    }
}

#[cfg(windows)]
pub fn list() -> AppResult<Vec<ServiceInfo>> {
    imp::list()
}
#[cfg(not(windows))]
pub fn list() -> AppResult<Vec<ServiceInfo>> {
    Err(AppError::Unsupported("yalnızca Windows".into()))
}

#[cfg(windows)]
pub fn query(name: &str) -> AppResult<ServiceInfo> {
    validate_service_name(name)?;
    imp::query(name)
}
#[cfg(not(windows))]
pub fn query(_name: &str) -> AppResult<ServiceInfo> {
    Err(AppError::Unsupported("yalnızca Windows".into()))
}

fn record_failure(db: &Db, title: String, detail: serde_json::Value, e: &AppError) {
    if !matches!(e, AppError::Cancelled) {
        let mut d = detail;
        d["error"] = serde_json::Value::String(e.to_string());
        let _ = oplog::record(db, "service", &title, Outcome::Failed, false, d);
    }
}

/// Hizmetin başlangıç türünü değiştirir (UAC onayı gerekir).
pub fn set_start_type(db: &Db, name: &str, start_type: &str) -> AppResult<ChangeResult<ServiceInfo>> {
    validate_service_name(name)?;
    let value = sc_start_value(start_type)?;
    if let Some(reason) = protection(name) {
        return Err(AppError::Protected(format!("\"{name}\" hizmetinin başlangıç türü değiştirilemez. {reason}")));
    }
    let before = query(name)?;
    if before.start_type == start_type {
        return Ok(ChangeResult {
            message: format!("Başlangıç türü zaten \"{}\".", before.start_type_label),
            item: before,
            elevated: false,
            oplog_id: None,
        });
    }
    if matches!(before.start_type.as_str(), "boot" | "system" | "unknown") {
        return Err(AppError::Protected("bu hizmet türünün başlangıç ayarı değiştirilemez".into()));
    }
    let args: Vec<String> = vec!["config".into(), before.name.clone(), "start=".into(), value.into()];
    let detail = serde_json::json!({
        "name": before.name,
        "displayName": before.display_name,
        "previousStartType": before.start_type,
        "previousStatus": before.status,
        "requestedStartType": start_type,
    });
    let title_fail = format!("Hizmet başlangıç türü değiştirilemedi: {}", before.display_name);
    match crate::platform::run_elevated("sc.exe", &args) {
        Ok(0) => {}
        Ok(code) => {
            let e = AppError::System(format!("hizmet başlangıç türü değiştirilemedi: {}", sc_error_message(code)));
            record_failure(db, title_fail, detail, &e);
            return Err(e);
        }
        Err(e) => {
            record_failure(db, title_fail, detail, &e);
            return Err(e);
        }
    }
    let after = query(name)?;
    if after.start_type != start_type {
        let e = AppError::System(format!(
            "komut çalıştı ancak başlangıç türü değişmedi (şu an: {})",
            after.start_type_label
        ));
        record_failure(db, title_fail, detail, &e);
        return Err(e);
    }
    let oplog_id = oplog::record(
        db,
        "service",
        &format!("Hizmet başlangıç türü değiştirildi: {} ({} → {})", after.display_name, before.start_type_label, after.start_type_label),
        Outcome::Success,
        true,
        serde_json::json!({
            "action": "start_type",
            "name": after.name,
            "displayName": after.display_name,
            "previousStartType": before.start_type,
            "previousStatus": before.status,
            "newStartType": after.start_type,
        }),
    )
    .ok();
    Ok(ChangeResult {
        message: format!(
            "Başlangıç türü \"{}\" olarak ayarlandı. Değişiklik yönetici onayıyla (UAC) uygulandı.",
            after.start_type_label
        ),
        item: after,
        elevated: true,
        oplog_id,
    })
}

/// Hizmeti başlatır veya durdurur (UAC onayı gerekir).
pub fn control(db: &Db, name: &str, action: &str) -> AppResult<ChangeResult<ServiceInfo>> {
    validate_service_name(name)?;
    let (verb, target, label) = match action {
        "start" => ("start", "running", "başlatıldı"),
        "stop" => ("stop", "stopped", "durduruldu"),
        _ => return Err(AppError::InvalidInput("işlem geçersiz (start veya stop olmalı)".into())),
    };
    if action == "stop" {
        if let Some(reason) = protection(name) {
            return Err(AppError::Protected(format!("\"{name}\" hizmeti durdurulamaz. {reason}")));
        }
    }
    let before = query(name)?;
    if before.status == target {
        return Ok(ChangeResult {
            message: format!("Hizmet zaten {}.", if action == "start" { "çalışıyor" } else { "durdurulmuş" }),
            item: before,
            elevated: false,
            oplog_id: None,
        });
    }
    if action == "start" && before.start_type == "disabled" {
        return Err(AppError::Conflict("hizmet devre dışı olduğu için başlatılamıyor; önce başlangıç türünü değiştirin".into()));
    }
    if action == "stop" && before.status == "running" && !before.can_stop {
        return Err(AppError::Conflict("bu hizmet şu anda durdurma isteğini kabul etmiyor".into()));
    }
    let detail = serde_json::json!({
        "name": before.name,
        "displayName": before.display_name,
        "previousStatus": before.status,
        "previousStartType": before.start_type,
        "requestedAction": action,
    });
    let title_fail = format!(
        "Hizmet {}: {}",
        if action == "start" { "başlatılamadı" } else { "durdurulamadı" },
        before.display_name
    );
    let args: Vec<String> = vec![verb.into(), before.name.clone()];
    match crate::platform::run_elevated("sc.exe", &args) {
        Ok(0) => {}
        Ok(code) => {
            let e = AppError::System(format!("hizmet {}: {}", if action == "start" { "başlatılamadı" } else { "durdurulamadı" }, sc_error_message(code)));
            record_failure(db, title_fail, detail, &e);
            return Err(e);
        }
        Err(e) => {
            record_failure(db, title_fail, detail, &e);
            return Err(e);
        }
    }
    // Hizmetin hedef duruma geçmesini en fazla ~15 saniye bekle.
    let mut after = query(name)?;
    for _ in 0..30 {
        if after.status == target || !after.status.ends_with("_pending") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
        after = query(name)?;
    }
    let reached = after.status == target;
    let outcome = if reached { Outcome::Success } else { Outcome::Partial };
    let oplog_id = oplog::record(
        db,
        "service",
        &format!("Hizmet {}: {}", label, after.display_name),
        outcome,
        true,
        serde_json::json!({
            "action": action,
            "name": after.name,
            "displayName": after.display_name,
            "previousStatus": before.status,
            "previousStartType": before.start_type,
            "newStatus": after.status,
        }),
    )
    .ok();
    let message = if reached {
        format!("Hizmet {label}. Değişiklik yönetici onayıyla (UAC) uygulandı.")
    } else {
        format!(
            "Komut gönderildi ancak hizmet henüz hedef duruma geçmedi (şu an: {}). Birkaç saniye sonra listeyi yenileyin.",
            after.status_label
        )
    };
    Ok(ChangeResult { item: after, message, elevated: true, oplog_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_services() {
        for n in ["RpcSs", "rpcss", "WINDEFEND", "wuauserv", "Dnscache", "Appinfo", "TrustedInstaller", "AudioSrv"] {
            assert!(protection(n).is_some(), "{n} korunmalı");
        }
        for n in ["Themes", "Spooler", "WSearch", "SysMain", "DiagTrack", "XblGameSave"] {
            assert!(protection(n).is_none(), "{n} korunmamalı");
        }
        // Liste geçerli adlardan oluşmalı.
        for (n, r) in PROTECTED {
            assert!(validate_service_name(n).is_ok());
            assert!(!r.is_empty());
        }
    }

    #[test]
    fn start_types() {
        assert_eq!(sc_start_value("auto").unwrap(), "auto");
        assert_eq!(sc_start_value("auto_delayed").unwrap(), "delayed-auto");
        assert_eq!(sc_start_value("manual").unwrap(), "demand");
        assert_eq!(sc_start_value("disabled").unwrap(), "disabled");
        assert!(sc_start_value("boot").is_err());
        assert!(sc_start_value("auto; del").is_err());
        assert_eq!(start_type_code(2, true), "auto_delayed");
        assert_eq!(start_type_code(2, false), "auto");
        assert_eq!(start_type_code(3, false), "manual");
        assert_eq!(start_type_code(4, false), "disabled");
        assert_eq!(status_code(4), "running");
        assert_eq!(status_label("stopped"), "Durduruldu");
        assert!(sc_error_message(1058).contains("devre dışı"));
        assert!(sc_error_message(9999).contains("9999"));
    }

    #[test]
    fn protected_refused_before_any_action() {
        let db = Db::open_in_memory().unwrap();
        assert!(matches!(set_start_type(&db, "WinDefend", "disabled"), Err(AppError::Protected(_))));
        assert!(matches!(control(&db, "RpcSs", "stop"), Err(AppError::Protected(_))));
        assert!(matches!(set_start_type(&db, "bad\"name", "manual"), Err(AppError::InvalidInput(_))));
        assert!(matches!(set_start_type(&db, "Spooler", "boot"), Err(AppError::InvalidInput(_))));
        assert!(matches!(control(&db, "Spooler", "pause"), Err(AppError::InvalidInput(_))));
    }

    #[cfg(windows)]
    #[test]
    fn list_readonly() {
        let v = list().unwrap();
        assert!(v.len() > 20);
        let rpc = v.iter().find(|s| s.name.eq_ignore_ascii_case("RpcSs")).expect("RpcSs");
        assert!(rpc.protected);
        assert_eq!(rpc.status, "running");
        let q = query("RpcSs").unwrap();
        assert_eq!(q.start_type, "auto");
    }
}
