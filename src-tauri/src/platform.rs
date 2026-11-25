//! Windows'a özgü yardımcılar: yetki denetimi, Explorer'da gösterme, Geri
//! Dönüşüm Kutusu, tek seferlik yönetici işlemleri ve dosya kimliği.
//!
//! Güvenlik: Dış programlar hiçbir zaman bir kabuk (cmd /c) üzerinden
//! çalıştırılmaz; bağımsız değişkenler tek tek aktarılır. Yönetici hakkı
//! gerektiren işlemler, uygulamanın tamamını yükseltmek yerine yalnızca o
//! komut için Windows UAC onayı ister.

use crate::error::{AppError, AppResult};
use std::path::Path;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Konsol penceresi açmadan alt süreç oluşturma bayrağı.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Arka planda, pencere açmadan bir komut hazırlar.
pub fn hidden_command(program: &str) -> Command {
    let mut c = Command::new(program);
    #[cfg(windows)]
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

/// System32 altındaki bir aracın tam yolu (PATH ele geçirme saldırılarına
/// karşı).
pub fn system_tool(name: &str) -> String {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    format!("{root}\\System32\\{name}")
}

#[cfg(windows)]
pub fn is_elevated() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elev = TOKEN_ELEVATION::default();
        let mut len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elev as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elev.TokenIsElevated != 0
    }
}
#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Dosyayı Explorer'da seçili olarak gösterir; klasörse klasörü açar.
pub fn reveal_in_explorer(path: &str) -> AppResult<()> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(AppError::NotFound(path.to_string()));
    }
    let explorer = format!("{}\\explorer.exe", std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into()));
    let mut cmd = Command::new(explorer);
    if p.is_dir() {
        cmd.arg(p);
    } else {
        // explorer.exe /select,"yol" biçimini bekler; raw_arg ile tırnaklar
        // korunur. Yol tırnak içeremez (Windows dosya adlarında yasak).
        #[cfg(windows)]
        cmd.raw_arg(format!("/select,\"{}\"", path.replace('"', "")));
    }
    cmd.spawn().map_err(|e| AppError::System(format!("Explorer açılamadı: {e}")))?;
    Ok(())
}

/// Yalnızca izin verilen sabit adreslerin varsayılan tarayıcıda açılması.
pub fn open_url(url: &str) -> AppResult<()> {
    const ALLOWED: &[&str] = &["https://github.com/efecrtgl"];
    if !ALLOWED.iter().any(|a| url == *a || url.starts_with(&format!("{a}/"))) {
        return Err(AppError::InvalidInput("bu adres açılamaz".into()));
    }
    shell_open(url)
}

#[cfg(windows)]
fn shell_open(target: &str) -> AppResult<()> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let op = to_wide("open");
    let file = to_wide(target);
    let r = unsafe { ShellExecuteW(None, PCWSTR(op.as_ptr()), PCWSTR(file.as_ptr()), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
    if r.0 as isize <= 32 {
        return Err(AppError::System("adres açılamadı".into()));
    }
    Ok(())
}
#[cfg(not(windows))]
fn shell_open(_t: &str) -> AppResult<()> {
    Err(AppError::Unsupported("yalnızca Windows".into()))
}

/// Tek bir sistem aracını, UAC onayıyla yönetici olarak çalıştırır ve
/// bitmesini bekler. Dönen değer çıkış kodudur.
///
/// `args` içindeki değerler çağıran tarafından doğrulanmış olmalıdır.
#[cfg(windows)]
pub fn run_elevated(tool: &str, args: &[String]) -> AppResult<u32> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let file = to_wide(&system_tool(tool));
    let params = to_wide(&args.iter().map(|a| quote_arg(a)).collect::<Vec<_>>().join(" "));
    let verb = to_wide("runas");
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    unsafe {
        if ShellExecuteExW(&mut info).is_err() {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(1223) {
                return Err(AppError::Cancelled);
            }
            return Err(AppError::PermissionDenied(format!("yönetici izni alınamadı ({err})")));
        }
        if info.hProcess.is_invalid() {
            return Err(AppError::System("yükseltilmiş süreç başlatılamadı".into()));
        }
        WaitForSingleObject(info.hProcess, INFINITE);
        let mut code = 0u32;
        let _ = GetExitCodeProcess(info.hProcess, &mut code);
        let _ = CloseHandle(info.hProcess);
        Ok(code)
    }
}
#[cfg(not(windows))]
pub fn run_elevated(_tool: &str, _args: &[String]) -> AppResult<u32> {
    Err(AppError::Unsupported("yalnızca Windows".into()))
}

/// Windows komut satırı kurallarına göre bir bağımsız değişkeni tırnaklar.
pub fn quote_arg(a: &str) -> String {
    if !a.is_empty() && !a.chars().any(|c| c == ' ' || c == '\t' || c == '"') {
        return a.to_string();
    }
    let mut out = String::from("\"");
    let mut backslashes = 0;
    for c in a.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                out.push_str(&"\\".repeat(backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.push_str(&"\\".repeat(backslashes));
                out.push(c);
                backslashes = 0;
            }
        }
    }
    out.push_str(&"\\".repeat(backslashes * 2));
    out.push('"');
    out
}

/// Dosyaları Windows Geri Dönüşüm Kutusu'na taşır (geri alınabilir).
#[cfg(windows)]
pub fn move_to_recycle_bin(paths: &[String]) -> AppResult<()> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE, SHFILEOPSTRUCTW,
    };
    if paths.is_empty() {
        return Ok(());
    }
    // Çift sıfır sonlandırmalı yol listesi.
    let mut buf: Vec<u16> = Vec::new();
    for p in paths {
        buf.extend(p.encode_utf16());
        buf.push(0);
    }
    buf.push(0);
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(buf.as_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT).0 as u16,
        ..Default::default()
    };
    let r = unsafe { SHFileOperationW(&mut op) };
    if r != 0 {
        return Err(AppError::System(format!("Geri Dönüşüm Kutusu'na taşınamadı (kod {r})")));
    }
    if op.fAnyOperationsAborted.as_bool() {
        return Err(AppError::Cancelled);
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn move_to_recycle_bin(_p: &[String]) -> AppResult<()> {
    Err(AppError::Unsupported("yalnızca Windows".into()))
}

/// Geri Dönüşüm Kutusu'ndaki toplam boyut ve öğe sayısı.
#[cfg(windows)]
pub fn recycle_bin_info() -> Option<(u64, u64)> {
    use windows::Win32::UI::Shell::{SHQueryRecycleBinW, SHQUERYRBINFO};
    let mut info = SHQUERYRBINFO { cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32, ..Default::default() };
    unsafe { SHQueryRecycleBinW(windows::core::PCWSTR::null(), &mut info).ok()? };
    Some((info.i64Size as u64, info.i64NumItems as u64))
}
#[cfg(not(windows))]
pub fn recycle_bin_info() -> Option<(u64, u64)> {
    None
}

#[cfg(windows)]
pub fn empty_recycle_bin() -> AppResult<()> {
    use windows::Win32::UI::Shell::{SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND};
    unsafe {
        SHEmptyRecycleBinW(None, windows::core::PCWSTR::null(), SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND)
            .map_err(|e| AppError::System(format!("Geri Dönüşüm Kutusu boşaltılamadı: {e}")))
    }
}
#[cfg(not(windows))]
pub fn empty_recycle_bin() -> AppResult<()> {
    Err(AppError::Unsupported("yalnızca Windows".into()))
}

/// Dosyanın birim seri numarası + dosya dizini: sabit bağlantıları (hard
/// link) tespit etmek için kullanılır.
#[cfg(windows)]
pub fn file_identity(path: &Path) -> Option<(u32, u64)> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION};
    let f = std::fs::File::open(path).ok()?;
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(HANDLE(f.as_raw_handle()), &mut info).ok()? };
    Some((info.dwVolumeSerialNumber, ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64))
}
#[cfg(not(windows))]
pub fn file_identity(_p: &Path) -> Option<(u32, u64)> {
    None
}

/// Windows ile birlikte oturum açıldığında başlatma (HKCU\...\Run).
#[cfg(windows)]
pub fn set_launch_at_startup(enable: bool) -> AppResult<()> {
    use winreg::enums::*;
    let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu
        .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .map_err(|e| AppError::io(e, "başlangıç kaydı"))?;
    if enable {
        let exe = std::env::current_exe().map_err(|e| AppError::io(e, "uygulama yolu"))?;
        key.set_value("Ivily Optimizer", &format!("\"{}\" --minimized", exe.display()))
            .map_err(|e| AppError::io(e, "başlangıç kaydı"))?;
    } else {
        match key.delete_value("Ivily Optimizer") {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(AppError::io(e, "başlangıç kaydı")),
        }
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn set_launch_at_startup(_e: bool) -> AppResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quoting() {
        assert_eq!(quote_arg("abc"), "abc");
        assert_eq!(quote_arg("a b"), "\"a b\"");
        assert_eq!(quote_arg("a\"b"), "\"a\\\"b\"");
        assert_eq!(quote_arg("C:\\x y\\"), "\"C:\\x y\\\\\"");
        assert_eq!(quote_arg(""), "\"\"");
    }

    #[cfg(windows)]
    #[test]
    fn identity_detects_hard_links() {
        let t = tempfile::tempdir().unwrap();
        let a = t.path().join("a");
        let b = t.path().join("b");
        let c = t.path().join("c");
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&c, b"x").unwrap();
        std::fs::hard_link(&a, &b).unwrap();
        assert_eq!(file_identity(&a), file_identity(&b));
        assert_ne!(file_identity(&a), file_identity(&c));
    }
}

/// Sistem sürücüsündeki boş alan (bayt).
#[cfg(windows)]
pub fn system_drive_free() -> Option<u64> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let root = to_wide(&format!("{drive}\\"));
    let mut free = 0u64;
    unsafe { GetDiskFreeSpaceExW(PCWSTR(root.as_ptr()), Some(&mut free), None, None).ok()? };
    Some(free)
}
#[cfg(not(windows))]
pub fn system_drive_free() -> Option<u64> {
    None
}
