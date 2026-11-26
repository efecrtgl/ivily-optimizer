//! Windows sistem yönetimi modülleri: başlangıç öğeleri, hizmetler, zamanlanmış
//! görevler, güç planları, ağ bilgisi ve güncelleme denetimi.
//!
//! Güvenlik ilkeleri:
//! - Dış programlar hiçbir zaman bir kabuk dizesi üzerinden çalıştırılmaz.
//! - PowerShell yalnızca sabit betiklerle çalıştırılır; değerler ortam
//!   değişkeniyle ve önceden doğrulanarak aktarılır.
//! - Toplu devre dışı bırakma ya da "önerilen" listeleri yoktur.
//! - Kritik Windows bileşenleri korunur; her değişiklik geri alınabilmesi için
//!   önceki durumuyla birlikte işlem geçmişine yazılır.

pub mod events;
pub mod indicators;
pub mod network;
pub mod power;
pub mod procinfo;
pub mod services;
pub mod signature;
pub mod startup;
pub mod tasks;
pub mod update;

use crate::error::{AppError, AppResult};
use crate::platform::{hidden_command, system_tool};
use serde::Serialize;

/// Değişiklik komutlarının ortak sonucu.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeResult<T: Serialize> {
    /// Değişiklikten sonra sistemden yeniden okunan gerçek durum.
    pub item: T,
    /// Kullanıcıya gösterilecek Türkçe özet.
    pub message: String,
    /// Değişiklik için Windows UAC (yönetici) onayı istendi mi?
    pub elevated: bool,
    /// İşlem geçmişi kaydının kimliği (geri alma için).
    pub oplog_id: Option<i64>,
}

/// Konsol araçlarının çıktısını çözer: önce UTF-8, olmazsa OEM kod sayfası.
pub fn decode_console(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.trim_start_matches('\u{feff}').to_string(),
        Err(_) => decode_oem(bytes),
    }
}

#[cfg(windows)]
fn decode_oem(bytes: &[u8]) -> String {
    use windows::Win32::Globalization::{GetOEMCP, MultiByteToWideChar, MULTI_BYTE_TO_WIDE_CHAR_FLAGS};
    if bytes.is_empty() {
        return String::new();
    }
    unsafe {
        let cp = GetOEMCP();
        let n = MultiByteToWideChar(cp, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0), bytes, None);
        if n <= 0 {
            return String::from_utf8_lossy(bytes).into_owned();
        }
        let mut buf = vec![0u16; n as usize];
        let n2 = MultiByteToWideChar(cp, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0), bytes, Some(&mut buf));
        if n2 <= 0 {
            return String::from_utf8_lossy(bytes).into_owned();
        }
        String::from_utf16_lossy(&buf[..n2 as usize])
    }
}
#[cfg(not(windows))]
fn decode_oem(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Sabit bir PowerShell betiğini çalıştırır ve standart çıktısını döndürür.
///
/// `script` her zaman derleme zamanında sabit olmalıdır; kullanıcıdan gelen
/// değerler yalnızca `env` üzerinden (doğrulandıktan sonra) aktarılır.
pub fn run_powershell(script: &'static str, env: &[(&str, &str)], what: &str) -> AppResult<String> {
    let ps = system_tool("WindowsPowerShell\\v1.0\\powershell.exe");
    let mut cmd = hidden_command(&ps);
    cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script]);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().map_err(|e| AppError::io(e, &format!("{what} için PowerShell başlatılamadı")))?;
    let stdout = decode_console(&out.stdout);
    if !out.status.success() && stdout.trim().is_empty() {
        let stderr = decode_console(&out.stderr);
        tracing::warn!(what, stderr = %stderr.trim(), "PowerShell betiği başarısız");
        return Err(AppError::System(format!("{what} alınamadı (PowerShell hatası)")));
    }
    Ok(stdout)
}

/// PowerShell `ConvertTo-Json` çıktısını her zaman bir dizi olarak yorumlar
/// (tek nesne veya boş çıktı durumlarını da karşılar).
pub fn json_array(text: &str) -> Result<Vec<serde_json::Value>, serde_json::Error> {
    let t = text.trim().trim_start_matches('\u{feff}');
    if t.is_empty() || t == "null" {
        return Ok(Vec::new());
    }
    match serde_json::from_str::<serde_json::Value>(t)? {
        serde_json::Value::Array(a) => Ok(a),
        serde_json::Value::Null => Ok(Vec::new()),
        other => Ok(vec![other]),
    }
}

fn has_control(s: &str) -> bool {
    s.chars().any(|c| c.is_control())
}

/// Hizmet adı: `[A-Za-z0-9_.\-$ ]`, en fazla 256 karakter.
pub fn validate_service_name(name: &str) -> AppResult<()> {
    if name.is_empty() || name.len() > 256 || name.trim() != name {
        return Err(AppError::InvalidInput("hizmet adı geçersiz".into()));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '$' | ' ')) {
        return Err(AppError::InvalidInput("hizmet adı izin verilmeyen karakterler içeriyor".into()));
    }
    Ok(())
}

/// Görev klasörü yolu: `\` ile başlar ve biter, tırnak veya denetim karakteri
/// içermez.
pub fn validate_task_path(path: &str) -> AppResult<()> {
    if path.is_empty() || path.len() > 1024 || !path.starts_with('\\') || !path.ends_with('\\') {
        return Err(AppError::InvalidInput("görev yolu geçersiz".into()));
    }
    if path.contains('"') || has_control(path) || path.contains("\\\\") {
        return Err(AppError::InvalidInput("görev yolu izin verilmeyen karakterler içeriyor".into()));
    }
    Ok(())
}

/// Görev adı: boş olmayan, ters eğik çizgi, tırnak veya denetim karakteri
/// içermeyen, en fazla 256 karakter.
pub fn validate_task_name(name: &str) -> AppResult<()> {
    if name.is_empty() || name.chars().count() > 256 || name.trim() != name {
        return Err(AppError::InvalidInput("görev adı geçersiz".into()));
    }
    if name.contains('"') || name.contains('\\') || has_control(name) {
        return Err(AppError::InvalidInput("görev adı izin verilmeyen karakterler içeriyor".into()));
    }
    Ok(())
}

/// GUID: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` (yalnızca onaltılık, süslü
/// parantezsiz).
pub fn is_guid(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 36 {
        return false;
    }
    b.iter().enumerate().all(|(i, c)| match i {
        8 | 13 | 18 | 23 => *c == b'-',
        _ => c.is_ascii_hexdigit(),
    })
}

pub fn validate_guid(s: &str) -> AppResult<String> {
    if !is_guid(s) {
        return Err(AppError::InvalidInput("GUID biçimi geçersiz".into()));
    }
    Ok(s.to_ascii_lowercase())
}

/// Ana bilgisayar adı: harf, rakam, tire ve nokta; en fazla 253 karakter;
/// her etiket 1–63 karakter, tireyle başlayıp bitemez.
pub fn validate_hostname(host: &str) -> AppResult<String> {
    let h = host.trim().trim_end_matches('.');
    let err = || AppError::InvalidInput("ana bilgisayar adı geçersiz".into());
    if h.is_empty() || h.len() > 253 {
        return Err(err());
    }
    for label in h.split('.') {
        if label.is_empty() || label.len() > 63 || label.starts_with('-') || label.ends_with('-') {
            return Err(err());
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(err());
        }
    }
    Ok(h.to_ascii_lowercase())
}

/// UTF-16 sıfır sonlandırmalı dizi.
pub(crate) fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Sıfır sonlandırmalı bir PWSTR'yi okur (boş/null için boş dize).
#[cfg(windows)]
pub(crate) unsafe fn pwstr_to_string(p: windows::core::PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    unsafe { p.to_string().unwrap_or_default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_names() {
        assert!(validate_service_name("wuauserv").is_ok());
        assert!(validate_service_name("MSSQL$SQLEXPRESS").is_ok());
        assert!(validate_service_name("Foo Bar.Svc-1_x").is_ok());
        assert!(validate_service_name("").is_err());
        assert!(validate_service_name(" lead").is_err());
        assert!(validate_service_name("a\"b").is_err());
        assert!(validate_service_name("a&b").is_err());
        assert!(validate_service_name("a\\b").is_err());
        assert!(validate_service_name("çay").is_err());
        assert!(validate_service_name(&"a".repeat(257)).is_err());
        assert!(validate_service_name(&"a".repeat(256)).is_ok());
    }

    #[test]
    fn task_paths_and_names() {
        assert!(validate_task_path("\\").is_ok());
        assert!(validate_task_path("\\Microsoft\\Windows\\Defrag\\").is_ok());
        assert!(validate_task_path("Microsoft\\").is_err());
        assert!(validate_task_path("\\a\"b\\").is_err());
        assert!(validate_task_path("\\a\nb\\").is_err());
        assert!(validate_task_path("\\a").is_err());
        assert!(validate_task_path("\\a\\\\b\\").is_err());
        assert!(validate_task_name("GoogleUpdateTaskMachineCore{ABC}").is_ok());
        assert!(validate_task_name("Görev 1").is_ok());
        assert!(validate_task_name("a\\b").is_err());
        assert!(validate_task_name("a\"b").is_err());
        assert!(validate_task_name("a\tb").is_err());
        assert!(validate_task_name("").is_err());
    }

    #[test]
    fn guids() {
        assert!(is_guid("381b4222-f694-41f0-9685-ff5bb260df2e"));
        assert!(is_guid("8C5E7FDA-E8BF-4A96-9A85-A6E23A8C635C"));
        assert!(!is_guid("{381b4222-f694-41f0-9685-ff5bb260df2e}"));
        assert!(!is_guid("381b4222-f694-41f0-9685-ff5bb260df2"));
        assert!(!is_guid("381b4222xf694-41f0-9685-ff5bb260df2e"));
        assert!(!is_guid("381b4222-f694-41f0-9685-ff5bb260df2g"));
        assert!(!is_guid("381b4222-f694-41f0-9685-ff5bb260df2e "));
        assert_eq!(
            validate_guid("8C5E7FDA-E8BF-4A96-9A85-A6E23A8C635C").unwrap(),
            "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"
        );
    }

    #[test]
    fn hostnames() {
        assert_eq!(validate_hostname("GitHub.com").unwrap(), "github.com");
        assert!(validate_hostname("www.microsoft.com.").is_ok());
        assert!(validate_hostname("a-b.c1").is_ok());
        assert!(validate_hostname("localhost").is_ok());
        assert!(validate_hostname("").is_err());
        assert!(validate_hostname("-a.com").is_err());
        assert!(validate_hostname("a..com").is_err());
        assert!(validate_hostname("a_b.com").is_err());
        assert!(validate_hostname("a b.com").is_err());
        assert!(validate_hostname("a.com;calc").is_err());
        assert!(validate_hostname(&format!("{}.com", "a".repeat(64))).is_err());
        assert!(validate_hostname(&vec!["abcdefghi"; 30].join(".")).is_err());
    }

    #[test]
    fn json_array_shapes() {
        assert_eq!(json_array("").unwrap().len(), 0);
        assert_eq!(json_array("null").unwrap().len(), 0);
        assert_eq!(json_array("{\"a\":1}").unwrap().len(), 1);
        assert_eq!(json_array("[{\"a\":1},{\"a\":2}]").unwrap().len(), 2);
        assert!(json_array("{bozuk").is_err());
    }

    #[test]
    fn decode_utf8_and_bom() {
        assert_eq!(decode_console("\u{feff}Dengeli".as_bytes()), "Dengeli");
        assert_eq!(decode_console("Güç".as_bytes()), "Güç");
    }
}
