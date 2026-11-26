//! Uygulamanın yerel veri dizinleri.
//!
//! Tüm veriler `%LOCALAPPDATA%\IvilyOptimizer` altında tutulur. Hiçbir veri
//! dışarıya gönderilmez.

use crate::error::{AppError, AppResult};
use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir);
    base.join("IvilyOptimizer")
}

pub fn db_path() -> PathBuf {
    data_dir().join("ivily.db")
}

pub fn logs_dir() -> PathBuf {
    data_dir().join("logs")
}

pub fn quarantine_dir() -> PathBuf {
    data_dir().join("quarantine")
}

pub fn ensure_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir())?;
    std::fs::create_dir_all(logs_dir())?;
    std::fs::create_dir_all(quarantine_dir())?;
    Ok(())
}

/// Windows yollarını karşılaştırma için normalleştirir: küçük harf, `\`
/// ayırıcı, `\\?\` öneki ve sondaki ayırıcı olmadan.
pub fn normalize(p: &str) -> String {
    let mut s = p.replace('/', "\\").to_lowercase();
    if let Some(rest) = s.strip_prefix("\\\\?\\") {
        s = rest.to_string();
    }
    while s.len() > 3 && s.ends_with('\\') {
        s.pop();
    }
    s
}

/// Arayüzden gelen bir yolu, yerel bir sürücüdeki mutlak yol olarak doğrular.
///
/// Reddedilenler:
/// - göreli yollar ve yalnızca sürücü harfi (`C:`),
/// - ağ (UNC, `\\sunucu\paylaşım`) ve aygıt (`\\.\`) yolları: bunlara erişmek
///   dışarıya SMB bağlantısı açıp Windows kimlik bilgisi özetini gönderebilir,
/// - `..` ve `.` bölümleri: metin tabanlı korunan konum denetiminin
///   atlatılmasını önler,
/// - sürücü ayırıcısı dışındaki `:` (NTFS alternatif veri akışları),
/// - denetim karakterleri ve Windows yol uzunluğu sınırını aşan değerler.
pub fn check_local(input: &str) -> AppResult<PathBuf> {
    let s = input.trim();
    if s.is_empty() || s.len() > 32_767 || s.chars().any(|c| c.is_control()) {
        return Err(AppError::InvalidInput("yol geçersiz".into()));
    }
    let rest = s.strip_prefix(r"\\?\").unwrap_or(s);
    let b = rest.as_bytes();
    let drive_ok = b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/');
    if !drive_ok {
        return Err(AppError::InvalidInput(
            r"yalnızca yerel bir sürücüdeki tam yol kullanılabilir (ör. C:\Users); ağ ve aygıt yolları desteklenmez"
                .into(),
        ));
    }
    // İlk üç bayt ASCII olduğundan bu dilimler karakter sınırındadır.
    if rest[2..].contains(':') {
        return Err(AppError::InvalidInput("yol ':' karakteri içeremez".into()));
    }
    if rest[3..].split(['\\', '/']).any(|seg| seg == ".." || seg == ".") {
        return Err(AppError::InvalidInput("yol '..' veya '.' bölümü içeremez".into()));
    }
    Ok(PathBuf::from(s))
}

/// `child`, `parent` ile aynı yol ya da onun altında mı?
pub fn is_within(child: &str, parent: &str) -> bool {
    let c = normalize(child);
    let p = normalize(parent);
    if p.is_empty() {
        return false;
    }
    if c == p {
        return true;
    }
    let p = if p.ends_with('\\') { p } else { format!("{p}\\") };
    c.starts_with(&p)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn within_works() {
        assert!(is_within(r"C:\Windows\System32", r"c:\windows"));
        assert!(is_within(r"C:\Windows", r"C:\Windows\"));
        assert!(!is_within(r"C:\WindowsApps", r"C:\Windows"));
        assert!(is_within(r"C:\a\b", r"C:\"));
        assert!(is_within(r"\\?\C:\a\b", r"C:\a"));
        assert!(!is_within(r"C:\a", ""));
    }

    #[test]
    fn check_local_accepts_drive_paths() {
        assert!(check_local(r"C:\Users\a").is_ok());
        assert!(check_local("  d:/veri/x.txt ").is_ok());
        assert!(check_local(r"\\?\C:\Users\a").is_ok());
        assert!(check_local(r"C:\").is_ok());
        assert!(check_local(r"C:\a\..b\c.").is_ok(), "'..' yalnızca tam bölüm olarak reddedilir");
    }

    #[test]
    fn check_local_rejects_unsafe_paths() {
        for bad in [
            "",
            "   ",
            "C:",
            r"Users\a",
            r"\Users\a",
            r"\\sunucu\paylasim\x",
            r"\\localhost\C$\Windows",
            r"\\.\C:\x",
            r"\\?\UNC\sunucu\x",
            r"\\?\GLOBALROOT\Device\x",
            "//sunucu/paylasim",
            r"C:\Users\a\..",
            r"C:\Users\..\Windows\System32",
            r"C:\a\.\b",
            r"C:\a\dosya.txt:gizli",
            "C:\\a\u{0}b",
            "€:\\x",
        ] {
            assert!(check_local(bad).is_err(), "kabul edilmemeliydi: {bad:?}");
        }
    }
}
