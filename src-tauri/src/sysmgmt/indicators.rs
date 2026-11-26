//! "Dikkat gerektirebilir" göstergeleri.
//!
//! Bu göstergeler hiçbir zaman bir dosyanın veya sürecin zararlı olduğunu
//! iddia etmez; yalnızca kullanıcının göz atmak isteyebileceği durumları
//! açıklamasıyla birlikte belirtir. Tümü ucuz, yol tabanlı denetimlerdir.

use crate::paths::{is_within, normalize};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Indicator {
    pub code: String,
    pub text: String,
}

impl Indicator {
    pub fn new(code: &str) -> Self {
        Self { code: code.to_string(), text: text_of(code).to_string() }
    }
}

pub fn text_of(code: &str) -> &'static str {
    match code {
        "missing_file" => "Hedef dosya bulunamadı; kayıt artık işe yaramıyor olabilir.",
        "unsigned" => "Dosya dijital olarak imzalanmamış. Bu tek başına zararlı olduğu anlamına gelmez; birçok küçük yazılım imzasızdır.",
        "invalid_signature" => "Dosyanın dijital imzası geçerli değil (dosya imzalandıktan sonra değişmiş veya sertifikaya güvenilmiyor olabilir). Bu durum dikkat gerektirebilir; kaynağını kontrol etmeniz önerilir.",
        "user_writable_location" => "Dosya, geçici dosyalar veya İndirilenler gibi kalıcı programlar için genellikle kullanılmayan bir konumda bulunuyor. Bu durum dikkat gerektirebilir; tek başına bir sorun olduğunu göstermez.",
        "script_host" => "Komut bir betik yorumlayıcısı (ör. PowerShell, cmd, wscript) üzerinden çalışıyor. Bazı yazılımlar bunu meşru olarak kullanır; neyi çalıştırdığını kontrol etmeniz önerilir.",
        "system_name_unexpected_location" => "Bu ad normalde Windows klasöründen çalışan bir sistem sürecine aittir, ancak bu süreç başka bir konumdan çalışıyor. Bu durum dikkat gerektirebilir; bazı meşru yazılımlar da benzer adlar kullanabilir.",
        "temp_location" => "Program geçici dosyalar veya İndirilenler klasöründen çalışıyor. Kurulum programları için olağandır; sürekli çalışan bir uygulama için dikkat gerektirebilir.",
        "exe_missing" => "Programın dosyası artık diskte bulunamıyor (çalışırken silinmiş, taşınmış veya güncellenmiş olabilir).",
        _ => "",
    }
}

/// Windows'un iyi bilinen süreç adları (yalnızca Windows klasöründen
/// çalışmaları beklenir).
pub const WELL_KNOWN_SYSTEM: &[&str] = &[
    "svchost.exe",
    "lsass.exe",
    "csrss.exe",
    "services.exe",
    "winlogon.exe",
    "smss.exe",
    "explorer.exe",
    "dwm.exe",
    "taskhostw.exe",
    "rundll32.exe",
    "conhost.exe",
];

/// Yol denetimleri için bir kez okunan konumlar.
#[derive(Debug, Clone, Default)]
pub struct PathContext {
    pub system_root: String,
    pub temp_dirs: Vec<String>,
    pub downloads: Option<String>,
    pub appdata: Option<String>,
    pub local_appdata: Option<String>,
    pub program_data: Option<String>,
    pub public: Option<String>,
}

impl PathContext {
    pub fn current() -> Self {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let system_root = env("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        let mut temp_dirs: Vec<String> = Vec::new();
        for t in
            [env("TEMP"), env("TMP"), Some(std::env::temp_dir().to_string_lossy().into_owned())].into_iter().flatten()
        {
            temp_dirs.push(t);
        }
        if let Some(l) = env("LOCALAPPDATA") {
            temp_dirs.push(format!("{l}\\Temp"));
        }
        temp_dirs.push(format!("{system_root}\\Temp"));
        temp_dirs.sort_by_key(|s| normalize(s));
        temp_dirs.dedup_by(|a, b| normalize(a) == normalize(b));
        Self {
            downloads: dirs::download_dir().map(|p| p.to_string_lossy().into_owned()),
            appdata: env("APPDATA"),
            local_appdata: env("LOCALAPPDATA"),
            program_data: env("ProgramData"),
            public: env("PUBLIC"),
            temp_dirs,
            system_root,
        }
    }

    pub fn in_system_root(&self, path: &str) -> bool {
        is_within(path, &self.system_root)
    }

    pub fn in_temp_or_downloads(&self, path: &str) -> bool {
        self.temp_dirs.iter().any(|t| is_within(path, t))
            || self.downloads.as_deref().is_some_and(|d| is_within(path, d))
    }

    /// Kalıcı programlar için olağan dışı, kullanıcının yazabildiği konum:
    /// geçici klasörler, İndirilenler, Ortak (Public) klasör veya doğrudan
    /// %APPDATA% / %LOCALAPPDATA% / %ProgramData% kökündeki dosyalar.
    /// `AppData\Local\Programs` gibi olağan kurulum konumları işaretlenmez.
    pub fn user_writable_unusual(&self, path: &str) -> bool {
        if self.in_temp_or_downloads(path) || self.public.as_deref().is_some_and(|p| is_within(path, p)) {
            return true;
        }
        let parent = normalize(path).rsplit_once('\\').map(|(p, _)| p.to_string()).unwrap_or_default();
        [&self.appdata, &self.local_appdata, &self.program_data]
            .into_iter()
            .flatten()
            .any(|root| normalize(root) == parent)
    }
}

fn file_name_lower(path: &str) -> String {
    path.rsplit(['\\', '/']).next().unwrap_or(path).to_lowercase()
}

/// Süreçler için ucuz göstergeler (imza denetimi yok).
pub fn process_indicators(
    ctx: &PathContext,
    name: &str,
    exe: Option<&str>,
    exe_exists: impl FnOnce(&str) -> bool,
) -> Vec<Indicator> {
    let mut v = Vec::new();
    let Some(exe) = exe.filter(|e| !e.is_empty()) else { return v };
    let lname = name.to_lowercase();
    if (WELL_KNOWN_SYSTEM.contains(&lname.as_str()) || WELL_KNOWN_SYSTEM.contains(&file_name_lower(exe).as_str()))
        && !ctx.in_system_root(exe)
    {
        v.push(Indicator::new("system_name_unexpected_location"));
    }
    if ctx.in_temp_or_downloads(exe) {
        v.push(Indicator::new("temp_location"));
    }
    if !exe_exists(exe) {
        v.push(Indicator::new("exe_missing"));
    }
    v
}

const SCRIPT_HOSTS: &[&str] = &["wscript.exe", "cscript.exe", "mshta.exe", "powershell.exe", "pwsh.exe", "cmd.exe"];
const SCRIPT_EXTS: &[&str] = &["vbs", "vbe", "js", "jse", "wsf", "hta", "ps1", "bat", "cmd"];

/// Komut bir betik yorumlayıcısı üzerinden mi çalışıyor?
pub fn is_script_host(command: &str, resolved: Option<&str>) -> bool {
    let target =
        resolved.map(str::to_string).or_else(|| super::startup::parse_command_exe(command)).unwrap_or_default();
    let fname = file_name_lower(&target);
    if let Some(ext) = fname.rsplit_once('.').map(|(_, e)| e) {
        if SCRIPT_EXTS.contains(&ext) {
            return true;
        }
    }
    if !SCRIPT_HOSTS.contains(&fname.as_str()) {
        return false;
    }
    match fname.as_str() {
        "wscript.exe" | "cscript.exe" | "mshta.exe" => true,
        // PowerShell / cmd: yalnızca bir komut veya betik verildiyse.
        _ => {
            let exe = super::startup::parse_command_exe(command).unwrap_or_default();
            let rest = command.trim().trim_start_matches('"');
            let rest = rest.get(exe.len()..).unwrap_or("").trim_start_matches('"').trim();
            !rest.is_empty()
        }
    }
}

/// Başlangıç öğeleri için göstergeler.
pub fn startup_indicators(
    ctx: &PathContext,
    command: &str,
    resolved: Option<&str>,
    exists: bool,
    signature: Option<&crate::sysmgmt::signature::SignatureStatus>,
) -> Vec<Indicator> {
    let mut v = Vec::new();
    if !exists {
        v.push(Indicator::new("missing_file"));
    }
    if let Some(s) = signature {
        match s.state.as_str() {
            "unsigned" => v.push(Indicator::new("unsigned")),
            "invalid" => v.push(Indicator::new("invalid_signature")),
            _ => {}
        }
    }
    if let Some(p) = resolved {
        if ctx.user_writable_unusual(p) {
            v.push(Indicator::new("user_writable_location"));
        }
    }
    if is_script_host(command, resolved) {
        v.push(Indicator::new("script_host"));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> PathContext {
        PathContext {
            system_root: "C:\\Windows".into(),
            temp_dirs: vec!["C:\\Users\\a\\AppData\\Local\\Temp".into(), "C:\\Windows\\Temp".into()],
            downloads: Some("C:\\Users\\a\\Downloads".into()),
            appdata: Some("C:\\Users\\a\\AppData\\Roaming".into()),
            local_appdata: Some("C:\\Users\\a\\AppData\\Local".into()),
            program_data: Some("C:\\ProgramData".into()),
            public: Some("C:\\Users\\Public".into()),
        }
    }

    #[test]
    fn process_rules() {
        let c = ctx();
        let yes = |_: &str| true;
        assert!(process_indicators(&c, "svchost.exe", Some("C:\\Windows\\System32\\svchost.exe"), yes).is_empty());
        let v = process_indicators(&c, "svchost.exe", Some("C:\\Users\\a\\AppData\\Roaming\\svchost.exe"), yes);
        assert_eq!(v[0].code, "system_name_unexpected_location");
        let v = process_indicators(&c, "setup.exe", Some("C:\\Users\\a\\Downloads\\setup.exe"), yes);
        assert_eq!(v.iter().map(|i| i.code.as_str()).collect::<Vec<_>>(), vec!["temp_location"]);
        let v = process_indicators(&c, "x.exe", Some("C:\\Users\\a\\AppData\\Local\\Temp\\x\\x.exe"), |_| false);
        assert_eq!(v.iter().map(|i| i.code.as_str()).collect::<Vec<_>>(), vec!["temp_location", "exe_missing"]);
        assert!(process_indicators(&c, "explorer.exe", None, yes).is_empty());
        assert!(process_indicators(&c, "explorer.exe", Some("C:\\Windows\\explorer.exe"), yes).is_empty());
        assert!(process_indicators(&c, "app.exe", Some("C:\\Program Files\\App\\app.exe"), yes).is_empty());
    }

    #[test]
    fn writable_locations() {
        let c = ctx();
        assert!(c.user_writable_unusual("C:\\Users\\a\\AppData\\Local\\Temp\\x.exe"));
        assert!(c.user_writable_unusual("C:\\Users\\a\\Downloads\\tool.exe"));
        assert!(c.user_writable_unusual("C:\\Users\\a\\AppData\\Roaming\\updater.exe"));
        assert!(c.user_writable_unusual("C:\\Users\\a\\AppData\\Local\\x.exe"));
        assert!(c.user_writable_unusual("C:\\Users\\Public\\x.exe"));
        assert!(!c.user_writable_unusual("C:\\Users\\a\\AppData\\Local\\Programs\\App\\app.exe"));
        assert!(!c.user_writable_unusual("C:\\Users\\a\\AppData\\Local\\Discord\\app-1.0\\Discord.exe"));
        assert!(!c.user_writable_unusual("C:\\Program Files\\App\\app.exe"));
    }

    #[test]
    fn script_hosts() {
        assert!(is_script_host("wscript.exe \"C:\\x\\a.vbs\"", Some("C:\\Windows\\System32\\wscript.exe")));
        assert!(is_script_host("C:\\x\\run.bat", Some("C:\\x\\run.bat")));
        assert!(is_script_host(
            "powershell.exe -WindowStyle Hidden -File C:\\x.ps1",
            Some("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe")
        ));
        assert!(is_script_host("\"C:\\Windows\\System32\\cmd.exe\" /c start x", None));
        assert!(!is_script_host("cmd.exe", None));
        assert!(!is_script_host("\"C:\\Program Files\\App\\app.exe\" --min", Some("C:\\Program Files\\App\\app.exe")));
    }

    #[test]
    fn startup_rules() {
        let c = ctx();
        let uns = crate::sysmgmt::signature::SignatureStatus::new("unsigned", None);
        let v = startup_indicators(
            &c,
            "C:\\Users\\a\\Downloads\\x.exe",
            Some("C:\\Users\\a\\Downloads\\x.exe"),
            false,
            Some(&uns),
        );
        let codes: Vec<_> = v.iter().map(|i| i.code.as_str()).collect();
        assert_eq!(codes, vec!["missing_file", "unsigned", "user_writable_location"]);
        assert!(v.iter().all(|i| !i.text.is_empty()));
    }
}
