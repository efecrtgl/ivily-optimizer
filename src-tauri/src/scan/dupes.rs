//! Yinelenen dosya analizi.
//!
//! Aşamalar (gereksiz okumadan kaçınmak için):
//! 1. Dosyaları listele, boyuta göre grupla (tek kalanları ele).
//! 2. İlk ve son 64 KB'ın BLAKE3 özeti ile ön eleme.
//! 3. Tüm içeriğin BLAKE3 özeti ile kesin doğrulama.
//! 4. Aynı fiziksel dosyayı gösteren sabit bağlantıları (hard link) ayıkla;
//!    bunları silmek yer kazandırmaz.

use super::walker::{collect_files, WalkOptions, WalkProgress};
use crate::jobs::JobCtl;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const PARTIAL: u64 = 64 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupFile {
    pub path: String,
    pub modified: i64,
    pub created: i64,
    pub protected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupGroup {
    pub id: usize,
    pub size: u64,
    pub hash: String,
    pub files: Vec<DupFile>,
    /// Silinirse kazanılabilecek alan: (n - 1) × boyut.
    pub wasted: u64,
    /// Önerilen saklanacak dosyanın dizini (en eski oluşturulan).
    pub keep_index: usize,
    pub verification: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DupReport {
    pub root: String,
    pub groups: Vec<DupGroup>,
    pub total_wasted: u64,
    pub scanned_files: u64,
    pub hashed_bytes: u64,
    pub hard_links_ignored: u64,
    pub errors: u64,
    pub cancelled: bool,
    pub min_size: u64,
}

fn hash_partial(p: &Path, size: u64) -> Option<[u8; 32]> {
    let mut f = std::fs::File::open(p).ok()?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; PARTIAL.min(size) as usize];
    f.read_exact(&mut buf).ok()?;
    h.update(&buf);
    if size > PARTIAL * 2 {
        f.seek(SeekFrom::End(-(PARTIAL as i64))).ok()?;
        f.read_exact(&mut buf).ok()?;
        h.update(&buf);
    }
    h.update(&size.to_le_bytes());
    Some(*h.finalize().as_bytes())
}

fn hash_full(p: &Path, ctl: Option<&JobCtl>) -> Option<[u8; 32]> {
    let mut f = std::fs::File::open(p).ok()?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        if ctl.map(|c| c.cancelled()).unwrap_or(false) {
            return None;
        }
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        if let Some(c) = ctl {
            c.add_done(n as u64);
        }
    }
    Some(*h.finalize().as_bytes())
}

fn group_by<K: std::hash::Hash + Eq + Send>(
    items: Vec<(PathBuf, u64, i64, i64)>,
    key: impl Fn(&(PathBuf, u64, i64, i64)) -> Option<K> + Sync,
) -> Vec<(K, Vec<(PathBuf, u64, i64, i64)>)> {
    let keyed: Vec<(K, (PathBuf, u64, i64, i64))> =
        items.into_par_iter().filter_map(|it| key(&it).map(|k| (k, it))).collect();
    let mut m: HashMap<K, Vec<(PathBuf, u64, i64, i64)>> = HashMap::new();
    for (k, it) in keyed {
        m.entry(k).or_default().push(it);
    }
    m.into_iter().filter(|(_, g)| g.len() > 1).collect()
}

pub fn find(root: &Path, opts: &WalkOptions, min_size: u64, ctl: &JobCtl) -> DupReport {
    let progress: &WalkProgress = &ctl.progress;
    ctl.set_phase("Dosyalar listeleniyor", 0);
    let (files, outcome) = collect_files(root, opts, &ctl.cancel, progress, |_, f| f.size >= min_size && !f.is_cloud_placeholder());
    let scanned = files.len() as u64;
    let mut report = DupReport {
        root: root.to_string_lossy().into_owned(),
        groups: vec![],
        total_wasted: 0,
        scanned_files: scanned,
        hashed_bytes: 0,
        hard_links_ignored: 0,
        errors: outcome.error_count,
        cancelled: false,
        min_size,
    };
    if ctl.cancelled() {
        report.cancelled = true;
        return report;
    }

    // 1. Boyuta göre grupla.
    let mut by_size: HashMap<u64, Vec<(PathBuf, u64, i64, i64)>> = HashMap::new();
    for (p, f) in files {
        by_size.entry(f.size).or_default().push((p, f.size, f.modified, f.created));
    }
    let candidates: Vec<(PathBuf, u64, i64, i64)> = by_size.into_values().filter(|g| g.len() > 1).flatten().collect();

    // 2. Kısmi özet.
    ctl.set_phase("Ön karşılaştırma yapılıyor", candidates.len() as u64);
    let partial_groups = group_by(candidates, |it| {
        if ctl.cancelled() {
            return None;
        }
        let r = hash_partial(&it.0, it.1).map(|h| (it.1, h));
        ctl.add_done(1);
        r
    });
    if ctl.cancelled() {
        report.cancelled = true;
        return report;
    }

    // 3. Tam içerik özeti.
    let full_items: Vec<(PathBuf, u64, i64, i64)> = partial_groups.into_iter().flat_map(|(_, g)| g).collect();
    let total_bytes: u64 = full_items.iter().map(|i| i.1).sum();
    ctl.set_phase("Tam içerik doğrulanıyor", total_bytes);
    let full_groups = group_by(full_items, |it| hash_full(&it.0, Some(ctl)).map(|h| (it.1, h)));
    report.hashed_bytes = total_bytes;
    if ctl.cancelled() {
        report.cancelled = true;
        return report;
    }

    // 4. Sabit bağlantıları ayıkla ve grupları oluştur.
    let mut groups = Vec::new();
    for ((_, digest), g) in full_groups {
        let mut seen = std::collections::HashSet::new();
        let mut unique = Vec::new();
        for it in g {
            match crate::platform::file_identity(&it.0) {
                Some(id) if !seen.insert(id) => report.hard_links_ignored += 1,
                _ => unique.push(it),
            }
        }
        if unique.len() < 2 {
            continue;
        }
        let size = unique[0].1;
        let hash_hex = hex(&digest[..8]);
        let mut files: Vec<DupFile> = unique
            .into_iter()
            .map(|(p, _, m, c)| {
                let s = p.to_string_lossy().into_owned();
                DupFile { protected: crate::cleanup::rules::is_protected_path(&s), path: s, modified: m, created: c }
            })
            .collect();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let keep_index = files
            .iter()
            .enumerate()
            .min_by_key(|(_, f)| (!f.protected, if f.created > 0 { f.created } else { i64::MAX }))
            .map(|(i, _)| i)
            .unwrap_or(0);
        let wasted = size * (files.len() as u64 - 1);
        groups.push(DupGroup {
            id: 0,
            size,
            hash: hash_hex,
            files,
            wasted,
            keep_index,
            verification: "Tüm içerik BLAKE3 ile doğrulandı".into(),
        });
    }
    groups.sort_by(|a, b| b.wasted.cmp(&a.wasted));
    for (i, g) in groups.iter_mut().enumerate() {
        g.id = i;
    }
    report.total_wasted = groups.iter().map(|g| g.wasted).sum();
    report.groups = groups;
    report
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::Jobs;

    #[test]
    fn finds_true_duplicates_only() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        std::fs::create_dir_all(r.join("x")).unwrap();
        let data: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(r.join("a.bin"), &data).unwrap();
        std::fs::write(r.join("x/b.bin"), &data).unwrap();
        // Aynı boyut, aynı baş ve son, farklı orta: yalnızca tam özet ayırır.
        let mut other = data.clone();
        other[150_000] ^= 0xff;
        std::fs::write(r.join("c.bin"), &other).unwrap();
        // Sabit bağlantı: yer kazandırmaz.
        std::fs::hard_link(r.join("a.bin"), r.join("x/a_link.bin")).unwrap();
        // Küçük dosya: eşik altında.
        std::fs::write(r.join("s1"), b"aa").unwrap();
        std::fs::write(r.join("s2"), b"aa").unwrap();

        let jobs = Jobs::default();
        let ctl = jobs.start("dupes").unwrap();
        let rep = find(r, &WalkOptions::new(2), 1024, &ctl);
        assert_eq!(rep.groups.len(), 1);
        let g = &rep.groups[0];
        assert_eq!(g.files.len(), 2);
        assert_eq!(g.wasted, 300_000);
        assert!(g.files.iter().all(|f| !f.path.ends_with("c.bin")));
        #[cfg(windows)]
        assert_eq!(rep.hard_links_ignored, 1);
    }
}
