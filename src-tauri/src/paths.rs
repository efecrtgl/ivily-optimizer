//! Uygulamanın yerel veri dizinleri.
//!
//! Tüm veriler `%LOCALAPPDATA%\IvilyOptimizer` altında tutulur. Hiçbir veri
//! dışarıya gönderilmez.

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
}
