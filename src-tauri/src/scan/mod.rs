//! Tarama motoru ve disk zekâsı özellikleri.

pub mod classify;
pub mod dupes;
pub mod tree;
pub mod walker;

use serde::Serialize;
use std::path::Path;
use walker::{collect_files, WalkOptions};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColdFile {
    pub path: String,
    pub size: u64,
    pub modified: i64,
    pub created: i64,
    pub ext: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColdReport {
    pub root: String,
    pub days: u32,
    pub min_size: u64,
    pub files: Vec<ColdFile>,
    pub total_matched: u64,
    pub total_size: u64,
    pub truncated: bool,
    pub unknown_dates: u64,
    pub errors: u64,
    pub cancelled: bool,
}

pub const COLD_LIMIT: usize = 20_000;

/// Belirtilen günden daha uzun süredir **değiştirilmemiş** dosyaları bulur.
/// Son erişim zamanı Windows'ta güvenilir şekilde güncellenmediği için
/// kullanılmaz.
pub fn cold_files(root: &Path, opts: &WalkOptions, days: u32, min_size: u64, ctl: &crate::jobs::JobCtl) -> ColdReport {
    let cutoff = chrono::Utc::now().timestamp() - days as i64 * 86_400;
    let mut unknown = 0u64;
    let (found, outcome) = collect_files(root, opts, &ctl.cancel, &ctl.progress, |_, f| {
        if f.size < min_size {
            return false;
        }
        if f.modified <= 0 {
            unknown += 1;
            return false;
        }
        f.modified < cutoff
    });
    let total_matched = found.len() as u64;
    let total_size = found.iter().map(|(_, f)| f.size).sum();
    let mut files: Vec<ColdFile> = found
        .into_iter()
        .map(|(p, f)| {
            let ext = f.extension();
            ColdFile {
                category: classify::category_of(&ext, f.size).to_string(),
                ext,
                path: p.to_string_lossy().into_owned(),
                size: f.size,
                modified: f.modified,
                created: f.created,
            }
        })
        .collect();
    files.sort_by_key(|e| std::cmp::Reverse(e.size));
    let truncated = files.len() > COLD_LIMIT;
    files.truncate(COLD_LIMIT);
    ColdReport {
        root: root.to_string_lossy().into_owned(),
        days,
        min_size,
        files,
        total_matched,
        total_size,
        truncated,
        unknown_dates: unknown,
        errors: outcome.error_count,
        cancelled: outcome.cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_old_files_by_mtime() {
        let t = tempfile::tempdir().unwrap();
        let old = t.path().join("old.doc");
        let new = t.path().join("new.doc");
        std::fs::write(&old, vec![0u8; 100]).unwrap();
        std::fs::write(&new, vec![0u8; 100]).unwrap();
        let ancient = std::time::SystemTime::now() - std::time::Duration::from_secs(400 * 86_400);
        std::fs::File::options().write(true).open(&old).unwrap().set_modified(ancient).unwrap();
        let jobs = crate::jobs::Jobs::default();
        let ctl = jobs.start("cold").unwrap();
        let r = cold_files(t.path(), &WalkOptions::new(1), 365, 0, &ctl);
        assert_eq!(r.files.len(), 1);
        assert!(r.files[0].path.ends_with("old.doc"));
        assert_eq!(r.files[0].category, "document");
    }
}
