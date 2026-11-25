//! Başlangıç öğeleri.
//!
//! Etkin/devre dışı durumu Windows Görev Yöneticisi ile aynı mekanizma
//! kullanılarak yönetilir: `...\Explorer\StartupApproved\{Run,Run32,StartupFolder}`
//! altındaki ikili değerler. Kayıt girdisi veya kısayol hiçbir zaman silinmez;
//! işlem tamamen geri alınabilir.

use super::ChangeResult;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::oplog::{self, Outcome};
use serde::Serialize;
use std::path::{Path, PathBuf};

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_ONCE_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce";
const RUN32_KEY: &str = "Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run";
const APPROVED_BASE: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved";

/// Başlangıç öğesinin kaynağı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    UserRun,
    MachineRun,
    MachineRun32,
    UserRunOnce,
    MachineRunOnce,
    UserFolder,
    CommonFolder,
}

impl Source {
    pub const ALL: [Source; 7] = [
        Source::UserRun,
        Source::MachineRun,
        Source::MachineRun32,
        Source::UserRunOnce,
        Source::MachineRunOnce,
        Source::UserFolder,
        Source::CommonFolder,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Source::UserRun => "hkcu_run",
            Source::MachineRun => "hklm_run",
            Source::MachineRun32 => "hklm_run32",
            Source::UserRunOnce => "hkcu_runonce",
            Source::MachineRunOnce => "hklm_runonce",
            Source::UserFolder => "user_folder",
            Source::CommonFolder => "common_folder",
        }
    }

    pub fn from_id(s: &str) -> Option<Source> {
        Source::ALL.into_iter().find(|x| x.id() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Source::UserRun => "Kayıt defteri (Kullanıcı)",
            Source::MachineRun => "Kayıt defteri (Tüm kullanıcılar)",
            Source::MachineRun32 => "Kayıt defteri (Tüm kullanıcılar, 32 bit)",
            Source::UserRunOnce => "Kayıt defteri – bir kez çalıştır (Kullanıcı)",
            Source::MachineRunOnce => "Kayıt defteri – bir kez çalıştır (Tüm kullanıcılar)",
            Source::UserFolder => "Başlangıç klasörü (Kullanıcı)",
            Source::CommonFolder => "Başlangıç klasörü (Tüm kullanıcılar)",
        }
    }

    pub fn is_machine(self) -> bool {
        matches!(self, Source::MachineRun | Source::MachineRun32 | Source::MachineRunOnce | Source::CommonFolder)
    }

    /// İlgili StartupApproved alt anahtarı (RunOnce için yok: salt okunur).
    pub fn approved_subkey(self) -> Option<&'static str> {
        match self {
            Source::UserRun | Source::MachineRun => Some("Run"),
            Source::MachineRun32 => Some("Run32"),
            Source::UserFolder | Source::CommonFolder => Some("StartupFolder"),
            Source::UserRunOnce | Source::MachineRunOnce => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupItem {
    /// Kararlı kimlik: `kaynak|ad`.
    pub id: String,
    pub name: String,
    pub command: String,
    pub resolved_path: Option<String>,
    pub exists: bool,
    /// Dosya sürüm bilgisindeki şirket adı (dijital imza ile doğrulanmamıştır).
    pub publisher: Option<String>,
    pub source: String,
    pub source_id: String,
    /// "user" veya "machine".
    pub scope: String,
    pub enabled: bool,
    pub can_toggle: bool,
    pub needs_admin: bool,
}

// ---------------------------------------------------------------------------
// Saf mantık (birim testli)
// ---------------------------------------------------------------------------

/// StartupApproved ikili değerinin yorumu: değer yoksa etkin; ilk baytın en
/// düşük biti 1 ise (0x03, 0x01, 0x07) devre dışı, 0 ise (0x02, 0x06) etkin.
pub fn approved_enabled(data: Option<&[u8]>) -> bool {
    match data.and_then(|d| d.first()) {
        None => true,
        Some(b) => b & 0x01 == 0,
    }
}

/// Unix zamanını Windows FILETIME değerine çevirir (1601'den beri 100 ns).
pub fn unix_to_filetime(secs: u64, nanos: u32) -> u64 {
    (secs + 11_644_473_600) * 10_000_000 + (nanos / 100) as u64
}

/// Görev Yöneticisi ile aynı biçimde 12 baytlık değer üretir.
pub fn approved_bytes(enabled: bool, filetime: u64) -> Vec<u8> {
    if enabled {
        let mut v = vec![0u8; 12];
        v[0] = 0x02;
        v
    } else {
        let mut v = vec![0x03, 0, 0, 0];
        v.extend_from_slice(&filetime.to_le_bytes());
        v
    }
}

fn now_filetime() -> u64 {
    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    unix_to_filetime(d.as_secs(), d.subsec_nanos())
}

pub fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect()
}

/// `%DEGISKEN%` biçimindeki ortam değişkenlerini genişletir; bilinmeyenler
/// olduğu gibi bırakılır.
pub fn expand_env_with(s: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 => {
                let name = &after[..end];
                match lookup(name) {
                    Some(v) => out.push_str(&v),
                    None => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn expand_env(s: &str) -> String {
    expand_env_with(s, |n| std::env::var(n).ok())
}

/// Komut satırından çalıştırılabilir dosya yolunu çıkarır (ortam değişkenleri
/// önceden genişletilmiş olmalıdır).
///
/// - `"C:\Yol\a.exe" -x` → `C:\Yol\a.exe`
/// - `C:\Program Files\X\x.exe /bg` → `C:\Program Files\X\x.exe`
/// - `rundll32.exe a.dll,Giris` → `rundll32.exe`
pub fn parse_command_exe(cmd: &str) -> Option<String> {
    let t = cmd.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(rest) = t.strip_prefix('"') {
        let end = rest.find('"').unwrap_or(rest.len());
        let p = rest[..end].trim();
        return (!p.is_empty()).then(|| p.to_string());
    }
    // Tırnaksız: boşluk içeren yollar için bilinen uzantılardan sonra kes.
    let lower = t.to_ascii_lowercase();
    for ext in [".exe", ".com", ".bat", ".cmd", ".lnk", ".vbs", ".js", ".ps1", ".scr"] {
        let mut from = 0;
        while let Some(pos) = lower[from..].find(ext) {
            let end = from + pos + ext.len();
            if end == t.len() || t[end..].starts_with(|c: char| c.is_whitespace() || c == ',') {
                return Some(t[..end].to_string());
            }
            from = end;
        }
    }
    t.split_whitespace().next().map(|s| s.to_string())
}

/// Komut satırını yorumlayıp tam yola çevirir. Dizin içermeyen adlar için
/// System32 ve Windows klasörlerine bakılır.
pub fn resolve_command(cmd: &str) -> Option<PathBuf> {
    let exe = parse_command_exe(&expand_env(cmd))?;
    let p = PathBuf::from(&exe);
    if p.is_absolute() || exe.contains('\\') || exe.contains('/') {
        return Some(p);
    }
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    for dir in [format!("{root}\\System32"), root.clone()] {
        let cand = Path::new(&dir).join(&exe);
        if cand.exists() {
            return Some(cand);
        }
        if Path::new(&exe).extension().is_none() {
            let cand = Path::new(&dir).join(format!("{exe}.exe"));
            if cand.exists() {
                return Some(cand);
            }
        }
    }
    None
}

fn split_id(id: &str) -> AppResult<(Source, &str)> {
    let (src, name) = id.split_once('|').ok_or_else(|| AppError::InvalidInput("başlangıç öğesi kimliği geçersiz".into()))?;
    let source = Source::from_id(src).ok_or_else(|| AppError::InvalidInput("başlangıç öğesi kaynağı geçersiz".into()))?;
    if name.is_empty() || name.len() > 1024 || name.chars().any(|c| c.is_control()) {
        return Err(AppError::InvalidInput("başlangıç öğesi adı geçersiz".into()));
    }
    Ok((source, name))
}

// ---------------------------------------------------------------------------
// Windows uygulaması
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use super::*;
    use winreg::enums::*;
    use winreg::{RegKey, RegValue};

    pub fn hive(s: Source) -> RegKey {
        RegKey::predef(if s.is_machine() { HKEY_LOCAL_MACHINE } else { HKEY_CURRENT_USER })
    }

    fn open_read(s: Source, path: &str) -> Option<RegKey> {
        hive(s).open_subkey_with_flags(path, KEY_READ | KEY_WOW64_64KEY).ok()
    }

    pub fn approved_raw(s: Source, name: &str) -> Option<Vec<u8>> {
        let sub = s.approved_subkey()?;
        let key = open_read(s, &format!("{APPROVED_BASE}\\{sub}"))?;
        key.get_raw_value(name).ok().map(|v| v.bytes.into_owned())
    }

    pub fn read_run_key(s: Source) -> Vec<(String, String)> {
        let path = match s {
            Source::UserRun | Source::MachineRun => RUN_KEY,
            Source::MachineRun32 => RUN32_KEY,
            Source::UserRunOnce | Source::MachineRunOnce => RUN_ONCE_KEY,
            _ => return Vec::new(),
        };
        let Some(key) = open_read(s, path) else { return Vec::new() };
        key.enum_values()
            .flatten()
            .filter(|(n, _)| !n.is_empty())
            .filter_map(|(n, v)| {
                let cmd: String = match v.vtype {
                    REG_SZ | REG_EXPAND_SZ => String::from_utf16_lossy(
                        &v.bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>(),
                    )
                    .trim_end_matches('\0')
                    .to_string(),
                    _ => return None,
                };
                Some((n, cmd))
            })
            .collect()
    }

    pub fn folder(s: Source) -> Option<PathBuf> {
        match s {
            Source::UserFolder => std::env::var_os("APPDATA")
                .map(|a| PathBuf::from(a).join("Microsoft\\Windows\\Start Menu\\Programs\\Startup")),
            Source::CommonFolder => std::env::var_os("ProgramData")
                .map(|a| PathBuf::from(a).join("Microsoft\\Windows\\Start Menu\\Programs\\StartUp")),
            _ => None,
        }
    }

    pub fn write_user_approved(sub: &str, name: &str, bytes: &[u8]) -> AppResult<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey(format!("{APPROVED_BASE}\\{sub}"))
            .map_err(|e| AppError::io(e, "başlangıç onay kaydı açılamadı"))?;
        key.set_raw_value(name, &RegValue { bytes: bytes.to_vec().into(), vtype: REG_BINARY })
            .map_err(|e| AppError::io(e, "başlangıç onay kaydı yazılamadı"))
    }

    pub fn write_machine_approved_direct(sub: &str, name: &str, bytes: &[u8]) -> AppResult<()> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let (key, _) = hklm
            .create_subkey_with_flags(format!("{APPROVED_BASE}\\{sub}"), KEY_ALL_ACCESS | KEY_WOW64_64KEY)
            .map_err(|e| AppError::io(e, "başlangıç onay kaydı açılamadı"))?;
        key.set_raw_value(name, &RegValue { bytes: bytes.to_vec().into(), vtype: REG_BINARY })
            .map_err(|e| AppError::io(e, "başlangıç onay kaydı yazılamadı"))
    }

    /// Kısayolun (.lnk) hedefini ve bağımsız değişkenlerini okur.
    pub fn resolve_shortcut(path: &Path) -> Option<(String, String)> {
        use windows::core::{Interface, PCWSTR};
        use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED, STGM_READ,
        };
        use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
        unsafe {
            let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let result = (|| {
                let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
                let pf: IPersistFile = link.cast().ok()?;
                let w = super::super::wide(&path.to_string_lossy());
                pf.Load(PCWSTR(w.as_ptr()), STGM_READ).ok()?;
                let mut buf = vec![0u16; 1024];
                let mut fd = WIN32_FIND_DATAW::default();
                link.GetPath(&mut buf, &mut fd, 0).ok()?;
                let target = String::from_utf16_lossy(&buf[..buf.iter().position(|c| *c == 0).unwrap_or(0)]);
                let mut abuf = vec![0u16; 2048];
                let args = if link.GetArguments(&mut abuf).is_ok() {
                    String::from_utf16_lossy(&abuf[..abuf.iter().position(|c| *c == 0).unwrap_or(0)])
                } else {
                    String::new()
                };
                (!target.is_empty()).then_some((target, args))
            })();
            if init.is_ok() {
                CoUninitialize();
            }
            result
        }
    }

    /// Dosya sürüm bilgisindeki şirket adı.
    pub fn company_name(path: &Path) -> Option<String> {
        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
        let w = super::super::wide(&path.to_string_lossy());
        unsafe {
            let size = GetFileVersionInfoSizeW(PCWSTR(w.as_ptr()), None);
            if size == 0 {
                return None;
            }
            let mut data = vec![0u8; size as usize];
            GetFileVersionInfoW(PCWSTR(w.as_ptr()), None, size, data.as_mut_ptr() as *mut _).ok()?;
            let mut ptr: *mut core::ffi::c_void = std::ptr::null_mut();
            let mut len = 0u32;
            let tr = super::super::wide("\\VarFileInfo\\Translation");
            let mut langs: Vec<(u16, u16)> = Vec::new();
            if VerQueryValueW(data.as_ptr() as *const _, PCWSTR(tr.as_ptr()), &mut ptr, &mut len).as_bool() && len >= 4 {
                let n = (len / 4) as usize;
                let s = std::slice::from_raw_parts(ptr as *const u16, n * 2);
                for i in 0..n {
                    langs.push((s[i * 2], s[i * 2 + 1]));
                }
            }
            langs.push((0x0409, 0x04B0));
            langs.push((0x0409, 0x04E4));
            for (lang, cp) in langs {
                let q = super::super::wide(&format!("\\StringFileInfo\\{lang:04x}{cp:04x}\\CompanyName"));
                let mut p: *mut core::ffi::c_void = std::ptr::null_mut();
                let mut l = 0u32;
                if VerQueryValueW(data.as_ptr() as *const _, PCWSTR(q.as_ptr()), &mut p, &mut l).as_bool() && l > 0 {
                    let s = std::slice::from_raw_parts(p as *const u16, l as usize);
                    let v = String::from_utf16_lossy(s).trim_end_matches('\0').trim().to_string();
                    if !v.is_empty() {
                        return Some(v);
                    }
                }
            }
            None
        }
    }
}

#[cfg(windows)]
fn build_item(source: Source, name: String, command: String, resolved: Option<PathBuf>) -> StartupItem {
    let exists = resolved.as_ref().map(|p| p.exists()).unwrap_or(false);
    let publisher = if exists { resolved.as_deref().and_then(imp::company_name) } else { None };
    let toggle = source.approved_subkey().is_some();
    let enabled = if toggle { approved_enabled(imp::approved_raw(source, &name).as_deref()) } else { true };
    StartupItem {
        id: format!("{}|{}", source.id(), name),
        name,
        command,
        resolved_path: resolved.map(|p| p.to_string_lossy().into_owned()),
        exists,
        publisher,
        source: source.label().to_string(),
        source_id: source.id().to_string(),
        scope: if source.is_machine() { "machine" } else { "user" }.to_string(),
        enabled,
        can_toggle: toggle,
        needs_admin: toggle && source.is_machine(),
    }
}

#[cfg(windows)]
fn items_of(source: Source) -> Vec<StartupItem> {
    match source {
        Source::UserFolder | Source::CommonFolder => {
            let Some(dir) = imp::folder(source) else { return Vec::new() };
            let Ok(rd) = std::fs::read_dir(&dir) else { return Vec::new() };
            let mut v = Vec::new();
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.eq_ignore_ascii_case("desktop.ini") || e.path().is_dir() {
                    continue;
                }
                let path = e.path();
                let is_lnk = path.extension().map(|x| x.eq_ignore_ascii_case("lnk")).unwrap_or(false);
                let (command, resolved) = match is_lnk.then(|| imp::resolve_shortcut(&path)).flatten() {
                    Some((target, args)) => {
                        let cmd = if args.is_empty() { format!("\"{target}\"") } else { format!("\"{target}\" {args}") };
                        (cmd, Some(PathBuf::from(target)))
                    }
                    None => (path.to_string_lossy().into_owned(), Some(path.clone())),
                };
                v.push(build_item(source, name, command, resolved));
            }
            v
        }
        _ => imp::read_run_key(source)
            .into_iter()
            .map(|(name, cmd)| {
                let resolved = resolve_command(&cmd);
                build_item(source, name, cmd, resolved)
            })
            .collect(),
    }
}

/// Tüm başlangıç öğelerini listeler (salt okunur).
#[cfg(windows)]
pub fn list() -> Vec<StartupItem> {
    let mut v: Vec<StartupItem> = Source::ALL.into_iter().flat_map(items_of).collect();
    v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    v
}
#[cfg(not(windows))]
pub fn list() -> Vec<StartupItem> {
    Vec::new()
}

#[cfg(windows)]
fn find(id: &str) -> AppResult<StartupItem> {
    let (source, name) = split_id(id)?;
    items_of(source)
        .into_iter()
        .find(|i| i.name == name)
        .ok_or_else(|| AppError::NotFound("başlangıç öğesi artık mevcut değil".into()))
}

/// Bir başlangıç öğesini etkinleştirir veya devre dışı bırakır.
#[cfg(windows)]
pub fn set_enabled(db: &Db, id: &str, enabled: bool) -> AppResult<ChangeResult<StartupItem>> {
    let item = find(id)?;
    let (source, name) = split_id(id)?;
    let Some(sub) = source.approved_subkey() else {
        return Err(AppError::Unsupported(
            "\"bir kez çalıştır\" öğeleri yalnızca görüntülenebilir; Windows bunları bir sonraki oturum açılışında kendisi kaldırır".into(),
        ));
    };
    if item.enabled == enabled {
        return Ok(ChangeResult {
            message: if enabled { "Öğe zaten etkin." } else { "Öğe zaten devre dışı." }.into(),
            item,
            elevated: false,
            oplog_id: None,
        });
    }
    let previous_raw = imp::approved_raw(source, name);
    let bytes = approved_bytes(enabled, now_filetime());
    let mut elevated = false;

    let res: AppResult<()> = if !source.is_machine() {
        imp::write_user_approved(sub, name, &bytes)
    } else if crate::platform::is_elevated() {
        imp::write_machine_approved_direct(sub, name, &bytes)
    } else {
        if name.contains('"') {
            return Err(AppError::InvalidInput("öğe adı desteklenmeyen karakterler içeriyor".into()));
        }
        elevated = true;
        let key = format!("HKLM\\{APPROVED_BASE}\\{sub}");
        let args: Vec<String> = vec![
            "add".into(),
            key,
            "/v".into(),
            name.to_string(),
            "/t".into(),
            "REG_BINARY".into(),
            "/d".into(),
            to_hex(&bytes),
            "/f".into(),
            "/reg:64".into(),
        ];
        match crate::platform::run_elevated("reg.exe", &args) {
            Ok(0) => Ok(()),
            Ok(code) => Err(AppError::System(format!("kayıt defteri değeri yazılamadı (reg.exe çıkış kodu {code})"))),
            Err(e) => Err(e),
        }
    };

    if let Err(e) = res {
        if !matches!(e, AppError::Cancelled) {
            let _ = oplog::record(
                db,
                "startup",
                &format!("Başlangıç öğesi değiştirilemedi: {}", item.name),
                Outcome::Failed,
                false,
                serde_json::json!({ "id": id, "requestedEnabled": enabled, "error": e.to_string() }),
            );
        }
        return Err(e);
    }

    let after = find(id)?;
    if after.enabled != enabled {
        return Err(AppError::System("değişiklik uygulanmadı; öğenin durumu değişmedi".into()));
    }
    let title = format!("Başlangıç öğesi {}: {}", if enabled { "etkinleştirildi" } else { "devre dışı bırakıldı" }, item.name);
    let oplog_id = oplog::record(
        db,
        "startup",
        &title,
        Outcome::Success,
        true,
        serde_json::json!({
            "id": id,
            "name": item.name,
            "source": source.id(),
            "scope": item.scope,
            "previousEnabled": item.enabled,
            "previousApprovedHex": previous_raw.as_deref().map(to_hex),
            "newEnabled": enabled,
            "newApprovedHex": to_hex(&bytes),
            "elevated": elevated,
        }),
    )
    .ok();
    let mut message = if enabled {
        "Öğe etkinleştirildi; bir sonraki oturum açılışında başlatılacak.".to_string()
    } else {
        "Öğe devre dışı bırakıldı; kaydı silinmedi ve istediğiniz zaman yeniden etkinleştirilebilir.".to_string()
    };
    if elevated {
        message.push_str(" Değişiklik yönetici onayıyla (UAC) uygulandı.");
    }
    Ok(ChangeResult { item: after, message, elevated, oplog_id })
}
#[cfg(not(windows))]
pub fn set_enabled(_db: &Db, _id: &str, _enabled: bool) -> AppResult<ChangeResult<StartupItem>> {
    Err(AppError::Unsupported("yalnızca Windows".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approved_interpretation() {
        assert!(approved_enabled(None));
        assert!(approved_enabled(Some(&[])));
        assert!(approved_enabled(Some(&[0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])));
        assert!(approved_enabled(Some(&[0x06, 0, 0, 0])));
        assert!(!approved_enabled(Some(&[0x03, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8])));
        assert!(!approved_enabled(Some(&[0x07])));
    }

    #[test]
    fn approved_bytes_format() {
        let on = approved_bytes(true, 123);
        assert_eq!(on.len(), 12);
        assert_eq!(on[0], 0x02);
        assert!(on[1..].iter().all(|b| *b == 0));
        let ft = unix_to_filetime(1_700_000_000, 0);
        let off = approved_bytes(false, ft);
        assert_eq!(off.len(), 12);
        assert_eq!(&off[..4], &[0x03, 0, 0, 0]);
        assert_eq!(u64::from_le_bytes(off[4..12].try_into().unwrap()), ft);
        assert!(approved_enabled(Some(&on)));
        assert!(!approved_enabled(Some(&off)));
        // 1970-01-01 → bilinen FILETIME değeri
        assert_eq!(unix_to_filetime(0, 0), 116_444_736_000_000_000);
        assert_eq!(to_hex(&[0x03, 0x00, 0xAB]), "0300AB");
    }

    #[test]
    fn command_parsing() {
        assert_eq!(parse_command_exe("\"C:\\Program Files\\App\\app.exe\" --min").as_deref(), Some("C:\\Program Files\\App\\app.exe"));
        assert_eq!(parse_command_exe("\"C:\\a b\\x.exe\"").as_deref(), Some("C:\\a b\\x.exe"));
        assert_eq!(parse_command_exe("C:\\Program Files\\App\\app.exe /background").as_deref(), Some("C:\\Program Files\\App\\app.exe"));
        assert_eq!(parse_command_exe("C:\\Tools\\tool.EXE").as_deref(), Some("C:\\Tools\\tool.EXE"));
        assert_eq!(parse_command_exe("rundll32.exe shell32.dll,Control_RunDLL").as_deref(), Some("rundll32.exe"));
        assert_eq!(parse_command_exe("C:\\x.exe.bak\\y.exe -a").as_deref(), Some("C:\\x.exe.bak\\y.exe"));
        assert_eq!(parse_command_exe("  ").as_deref(), None);
        assert_eq!(parse_command_exe("\"unterminated").as_deref(), Some("unterminated"));
        assert_eq!(parse_command_exe("foo bar").as_deref(), Some("foo"));
    }

    #[test]
    fn env_expansion() {
        let look = |n: &str| match n.to_ascii_uppercase().as_str() {
            "PROGRAMFILES" => Some("C:\\Program Files".to_string()),
            "WINDIR" => Some("C:\\Windows".to_string()),
            _ => None,
        };
        assert_eq!(expand_env_with("%ProgramFiles%\\A\\a.exe", look), "C:\\Program Files\\A\\a.exe");
        assert_eq!(expand_env_with("%windir%\\x %NOPE% 100%", look), "C:\\Windows\\x %NOPE% 100%");
        assert_eq!(expand_env_with("%%", look), "%%");
        let cmd = expand_env_with("%ProgramFiles%\\Vendor App\\v.exe -s", look);
        assert_eq!(parse_command_exe(&cmd).as_deref(), Some("C:\\Program Files\\Vendor App\\v.exe"));
    }

    #[test]
    fn id_parsing() {
        let (s, n) = split_id("hkcu_run|OneDrive").unwrap();
        assert_eq!(s, Source::UserRun);
        assert_eq!(n, "OneDrive");
        assert_eq!(split_id("user_folder|a|b.lnk").unwrap().1, "a|b.lnk");
        assert!(split_id("bogus|x").is_err());
        assert!(split_id("hkcu_run|").is_err());
        assert!(split_id("noseparator").is_err());
        assert!(Source::UserRunOnce.approved_subkey().is_none());
        assert_eq!(Source::MachineRun32.approved_subkey(), Some("Run32"));
    }

    #[cfg(windows)]
    #[test]
    fn list_is_readonly_and_works() {
        let items = list();
        for i in &items {
            assert!(i.id.contains('|'));
            assert!(i.scope == "user" || i.scope == "machine");
        }
    }
}
