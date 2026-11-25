//! Yerel anlık görüntüler: disk taramalarının özet klasör boyutları zaman
//! içinde saklanır ve karşılaştırılır. Dosya içeriği asla saklanmaz.

use crate::db::{now_ts, Db};
use crate::error::AppResult;
use crate::scan::tree::{DiskScan, ScanStatus};
use rusqlite::params;
use serde::Serialize;
use std::collections::HashMap;

/// Bu derinliğe kadar tüm klasörler saklanır; daha derindekiler yalnızca
/// boyutları eşiği aşıyorsa saklanır.
pub const FULL_DEPTH: u16 = 2;
pub const DEEP_MAX_DEPTH: u16 = 6;
pub const DEEP_MIN_SIZE: u64 = 50 * 1024 * 1024;
pub const MAX_ROWS: usize = 25_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotInfo {
    pub id: i64,
    pub ts: i64,
    pub root: String,
    pub status: String,
    pub total_size: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub error_count: u64,
    pub duration_ms: u64,
    pub volume_total: Option<u64>,
    pub volume_free: Option<u64>,
}

pub fn save(db: &Db, scan: &DiskScan, volume: Option<(u64, u64)>) -> AppResult<i64> {
    let root_node = match scan.nodes.first() {
        Some(n) => n,
        None => return Ok(0),
    };
    let mut ids: Vec<u32> = (0..scan.nodes.len() as u32)
        .filter(|&i| {
            let n = &scan.nodes[i as usize];
            n.depth <= FULL_DEPTH || (n.depth <= DEEP_MAX_DEPTH && n.size >= DEEP_MIN_SIZE)
        })
        .collect();
    if ids.len() > MAX_ROWS {
        ids.sort_by(|a, b| scan.nodes[*b as usize].size.cmp(&scan.nodes[*a as usize].size));
        ids.truncate(MAX_ROWS);
    }
    let root = scan.root.to_string_lossy().into_owned();
    db.with_mut(|c| {
        let tx = c.transaction()?;
        tx.execute(
            "INSERT INTO snapshots(ts, root, status, total_size, file_count, dir_count, error_count, duration_ms, volume_total, volume_free)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                scan.started_at,
                root,
                scan.status.as_str(),
                root_node.size as i64,
                root_node.files as i64,
                root_node.dirs as i64,
                scan.error_count as i64,
                scan.duration_ms as i64,
                volume.map(|v| v.0 as i64),
                volume.map(|v| v.1 as i64),
            ],
        )?;
        let sid = tx.last_insert_rowid();
        {
            let mut st = tx.prepare(
                "INSERT OR IGNORE INTO snapshot_dirs(snapshot_id, path, depth, size, files, dirs) VALUES(?1,?2,?3,?4,?5,?6)",
            )?;
            for i in ids {
                let n = &scan.nodes[i as usize];
                st.execute(params![
                    sid,
                    crate::paths::normalize(&scan.path_of(i).to_string_lossy()),
                    n.depth,
                    n.size as i64,
                    n.files as i64,
                    n.dirs as i64
                ])?;
            }
        }
        tx.commit()?;
        Ok(sid)
    })
}

fn row_to_info(r: &rusqlite::Row) -> rusqlite::Result<SnapshotInfo> {
    Ok(SnapshotInfo {
        id: r.get(0)?,
        ts: r.get(1)?,
        root: r.get(2)?,
        status: r.get(3)?,
        total_size: r.get::<_, i64>(4)? as u64,
        file_count: r.get::<_, i64>(5)? as u64,
        dir_count: r.get::<_, i64>(6)? as u64,
        error_count: r.get::<_, i64>(7)? as u64,
        duration_ms: r.get::<_, i64>(8)? as u64,
        volume_total: r.get::<_, Option<i64>>(9)?.map(|v| v as u64),
        volume_free: r.get::<_, Option<i64>>(10)?.map(|v| v as u64),
    })
}

const COLS: &str = "id, ts, root, status, total_size, file_count, dir_count, error_count, duration_ms, volume_total, volume_free";

pub fn list(db: &Db, root: Option<&str>) -> AppResult<Vec<SnapshotInfo>> {
    db.with(|c| {
        let mut st = c.prepare(&format!(
            "SELECT {COLS} FROM snapshots WHERE (?1 IS NULL OR root = ?1) ORDER BY ts DESC, id DESC"
        ))?;
        let rows = st.query_map(params![root], row_to_info)?;
        rows.collect()
    })
}

pub fn get(db: &Db, id: i64) -> AppResult<Option<SnapshotInfo>> {
    use rusqlite::OptionalExtension;
    db.with(|c| c.query_row(&format!("SELECT {COLS} FROM snapshots WHERE id = ?1"), [id], row_to_info).optional())
}

pub fn delete(db: &Db, id: i64) -> AppResult<()> {
    db.with(|c| c.execute("DELETE FROM snapshots WHERE id = ?1", [id]).map(|_| ()))
}

/// Saklama politikası: süresi dolan ve kök başına sınırı aşan anlık
/// görüntüleri siler.
pub fn prune(db: &Db, retention_days: u32, max_per_root: u32) -> AppResult<usize> {
    let cutoff = now_ts() - retention_days as i64 * 86_400;
    db.with(|c| {
        let mut n = c.execute("DELETE FROM snapshots WHERE ts < ?1", [cutoff])?;
        n += c.execute(
            "DELETE FROM snapshots WHERE id IN (
               SELECT id FROM (
                 SELECT id, ROW_NUMBER() OVER (PARTITION BY root ORDER BY ts DESC, id DESC) AS rn FROM snapshots
               ) WHERE rn > ?1)",
            [max_per_root],
        )?;
        Ok(n)
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirRow {
    pub path: String,
    pub depth: u16,
    pub size: u64,
    pub files: u64,
}

fn dirs_of(db: &Db, sid: i64) -> AppResult<HashMap<String, DirRow>> {
    db.with(|c| {
        let mut st = c.prepare("SELECT path, depth, size, files FROM snapshot_dirs WHERE snapshot_id = ?1")?;
        let rows = st.query_map([sid], |r| {
            Ok(DirRow {
                path: r.get(0)?,
                depth: r.get(1)?,
                size: r.get::<_, i64>(2)? as u64,
                files: r.get::<_, i64>(3)? as u64,
            })
        })?;
        let mut m = HashMap::new();
        for r in rows {
            let r = r?;
            m.insert(r.path.clone(), r);
        }
        Ok(m)
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirChange {
    pub path: String,
    pub depth: u16,
    pub old_size: Option<u64>,
    pub new_size: Option<u64>,
    pub delta: i64,
    pub pct: Option<f64>,
    pub old_files: Option<u64>,
    pub new_files: Option<u64>,
    pub files_delta: i64,
    /// "changed" | "added" | "removed"
    pub change: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub older: SnapshotInfo,
    pub newer: SnapshotInfo,
    pub total_delta: i64,
    pub files_delta: i64,
    pub changes: Vec<DirChange>,
    /// Karşılaştırmanın güvenilirliği hakkında uyarılar.
    pub warnings: Vec<String>,
}

pub fn compare(db: &Db, a: i64, b: i64) -> AppResult<Comparison> {
    let ia = get(db, a)?.ok_or_else(|| crate::error::AppError::NotFound("anlık görüntü".into()))?;
    let ib = get(db, b)?.ok_or_else(|| crate::error::AppError::NotFound("anlık görüntü".into()))?;
    let (older, newer) = if ia.ts <= ib.ts { (ia, ib) } else { (ib, ia) };
    let mut warnings = Vec::new();
    if older.root != newer.root {
        warnings.push("Farklı konumların anlık görüntüleri karşılaştırılıyor; sonuçlar anlamlı olmayabilir.".into());
    }
    for s in [&older, &newer] {
        if s.status != "complete" {
            warnings.push(format!(
                "{} tarihli tarama tamamlanmamış ({}); farklar eksik veriden kaynaklanabilir.",
                chrono::DateTime::from_timestamp(s.ts, 0).map(|d| d.format("%d.%m.%Y %H:%M").to_string()).unwrap_or_default(),
                status_label(&s.status)
            ));
        }
    }
    let da = dirs_of(db, older.id)?;
    let db_ = dirs_of(db, newer.id)?;
    let changes = diff_dirs(&da, &db_);
    Ok(Comparison {
        total_delta: newer.total_size as i64 - older.total_size as i64,
        files_delta: newer.file_count as i64 - older.file_count as i64,
        older,
        newer,
        changes,
        warnings,
    })
}

pub fn status_label(s: &str) -> &'static str {
    match s {
        "complete" => "tamamlandı",
        "cancelled" => "kısmi — iptal edildi",
        "failed" => "başarısız",
        _ => "bilinmiyor",
    }
}

/// İki klasör kümesini karşılaştırır. Derin klasörler yalnızca eşiği aşınca
/// saklandığından, yalnızca bir tarafta bulunan derin klasörler "yeni" veya
/// "silinmiş" sayılmaz (eşiğin altına düşmüş olabilirler).
pub fn diff_dirs(old: &HashMap<String, DirRow>, new: &HashMap<String, DirRow>) -> Vec<DirChange> {
    let mut out = Vec::new();
    for (p, n) in new {
        match old.get(p) {
            Some(o) => {
                let delta = n.size as i64 - o.size as i64;
                let fd = n.files as i64 - o.files as i64;
                if delta != 0 || fd != 0 {
                    out.push(DirChange {
                        path: p.clone(),
                        depth: n.depth,
                        old_size: Some(o.size),
                        new_size: Some(n.size),
                        delta,
                        pct: if o.size > 0 { Some(delta as f64 * 100.0 / o.size as f64) } else { None },
                        old_files: Some(o.files),
                        new_files: Some(n.files),
                        files_delta: fd,
                        change: "changed".into(),
                    });
                }
            }
            None if n.depth <= FULL_DEPTH => out.push(DirChange {
                path: p.clone(),
                depth: n.depth,
                old_size: None,
                new_size: Some(n.size),
                delta: n.size as i64,
                pct: None,
                old_files: None,
                new_files: Some(n.files),
                files_delta: n.files as i64,
                change: "added".into(),
            }),
            None => {}
        }
    }
    for (p, o) in old {
        if !new.contains_key(p) && o.depth <= FULL_DEPTH {
            out.push(DirChange {
                path: p.clone(),
                depth: o.depth,
                old_size: Some(o.size),
                new_size: None,
                delta: -(o.size as i64),
                pct: None,
                old_files: Some(o.files),
                new_files: None,
                files_delta: -(o.files as i64),
                change: "removed".into(),
            });
        }
    }
    out.sort_by(|a, b| b.delta.abs().cmp(&a.delta.abs()));
    out
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrowthItem {
    pub path: String,
    pub old_size: u64,
    pub new_size: u64,
    pub delta: i64,
    pub pct: Option<f64>,
    pub files_delta: i64,
    pub per_day: f64,
    /// Yalnızca gözlenen veriye dayalı işaret.
    pub fast: bool,
    pub timeline: Vec<(i64, u64)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrowthReport {
    pub root: String,
    pub enough_data: bool,
    pub snapshot_count: usize,
    pub from_ts: Option<i64>,
    pub to_ts: Option<i64>,
    pub items: Vec<GrowthItem>,
    pub note: String,
}

/// Bir kök için en eski ve en yeni *tamamlanmış* anlık görüntüyü (gün
/// sınırı içinde) karşılaştırarak büyüyen klasörleri bulur.
pub fn growth(db: &Db, root: &str, days: u32, limit: usize) -> AppResult<GrowthReport> {
    let since = now_ts() - days as i64 * 86_400;
    let snaps: Vec<SnapshotInfo> = list(db, Some(root))?
        .into_iter()
        .filter(|s| s.status == "complete" && s.ts >= since)
        .collect();
    if snaps.len() < 2 {
        return Ok(GrowthReport {
            root: root.into(),
            enough_data: false,
            snapshot_count: snaps.len(),
            from_ts: None,
            to_ts: None,
            items: vec![],
            note: "Büyüme analizi için bu konumun seçilen dönemde en az iki tamamlanmış taraması gerekir. Kısmi veya iptal edilen taramalar hesaba katılmaz.".into(),
        });
    }
    let newest = &snaps[0];
    let oldest = &snaps[snaps.len() - 1];
    let old = dirs_of(db, oldest.id)?;
    let new = dirs_of(db, newest.id)?;
    let span_days = ((newest.ts - oldest.ts) as f64 / 86_400.0).max(1.0 / 24.0);
    let mut items: Vec<GrowthItem> = new
        .values()
        .filter_map(|n| {
            let o = old.get(&n.path)?;
            let delta = n.size as i64 - o.size as i64;
            if delta <= 0 {
                return None;
            }
            let pct = if o.size > 0 { Some(delta as f64 * 100.0 / o.size as f64) } else { None };
            let fast = delta as u64 >= 1 << 30 || (delta as u64 >= 200 << 20 && pct.unwrap_or(100.0) >= 50.0);
            Some(GrowthItem {
                path: n.path.clone(),
                old_size: o.size,
                new_size: n.size,
                delta,
                pct,
                files_delta: n.files as i64 - o.files as i64,
                per_day: delta as f64 / span_days,
                fast,
                timeline: vec![],
            })
        })
        .collect();
    // Üst klasörler alt klasörlerinin büyümesini de içerir; en anlamlı
    // sonuçları göstermek için büyümeye göre sıralarız.
    items.sort_by(|a, b| b.delta.cmp(&a.delta));
    items.truncate(limit);

    // Zaman çizelgesi: listelenen klasörlerin her anlık görüntüdeki boyutu.
    let ids: Vec<i64> = snaps.iter().rev().map(|s| s.id).collect();
    db.with(|c| {
        let mut st = c.prepare("SELECT size FROM snapshot_dirs WHERE snapshot_id = ?1 AND path = ?2")?;
        for it in items.iter_mut() {
            for (sid, ts) in ids.iter().zip(snaps.iter().rev().map(|s| s.ts)) {
                if let Ok(sz) = st.query_row(params![sid, it.path], |r| r.get::<_, i64>(0)) {
                    it.timeline.push((ts, sz as u64));
                }
            }
        }
        Ok(())
    })?;

    Ok(GrowthReport {
        root: root.into(),
        enough_data: true,
        snapshot_count: snaps.len(),
        from_ts: Some(oldest.ts),
        to_ts: Some(newest.ts),
        items,
        note: "Değerler gözlenen tarama sonuçlarıdır. \"Hızlı büyüme\" işareti yalnızca bu dönemde 1 GB'tan fazla ya da %50 ve 200 MB'tan fazla artışı belirtir; bir sorun olduğu anlamına gelmez.".into(),
    })
}

pub fn last_scan_status(status: ScanStatus) -> &'static str {
    status.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::tree;
    use crate::scan::walker::{WalkOptions, WalkProgress};
    use std::sync::atomic::AtomicBool;

    fn scan(p: &std::path::Path) -> DiskScan {
        tree::scan(p, &WalkOptions::new(1), 1024, &AtomicBool::new(false), &WalkProgress::default())
    }

    #[test]
    fn save_compare_and_growth() {
        let db = Db::open_in_memory().unwrap();
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("a")).unwrap();
        std::fs::write(t.path().join("a/f"), vec![0u8; 100]).unwrap();
        let mut s1 = scan(t.path());
        s1.started_at -= 3600;
        let id1 = save(&db, &s1, None).unwrap();
        std::fs::write(t.path().join("a/g"), vec![0u8; 400]).unwrap();
        std::fs::create_dir_all(t.path().join("b")).unwrap();
        let s2 = scan(t.path());
        let id2 = save(&db, &s2, None).unwrap();

        let cmp = compare(&db, id2, id1).unwrap();
        assert_eq!(cmp.older.id, id1);
        assert_eq!(cmp.total_delta, 400);
        assert!(cmp.warnings.is_empty());
        let a = cmp.changes.iter().find(|c| c.path.ends_with("\\a")).unwrap();
        assert_eq!(a.delta, 400);
        assert_eq!(a.files_delta, 1);
        assert!(cmp.changes.iter().any(|c| c.change == "added" && c.path.ends_with("\\b")));

        let root = t.path().to_string_lossy().into_owned();
        let g = growth(&db, &root, 30, 10).unwrap();
        assert!(g.enough_data);
        assert_eq!(g.items[0].delta, 400);
        assert_eq!(g.items[0].timeline.len(), 2);
    }

    #[test]
    fn growth_needs_two_complete_scans() {
        let db = Db::open_in_memory().unwrap();
        let t = tempfile::tempdir().unwrap();
        let mut s = scan(t.path());
        save(&db, &s, None).unwrap();
        s.status = ScanStatus::Cancelled;
        save(&db, &s, None).unwrap();
        let g = growth(&db, &t.path().to_string_lossy(), 30, 10).unwrap();
        assert!(!g.enough_data);
        assert_eq!(g.snapshot_count, 1);
    }

    #[test]
    fn incomplete_scans_produce_warning() {
        let db = Db::open_in_memory().unwrap();
        let t = tempfile::tempdir().unwrap();
        let s1 = scan(t.path());
        let a = save(&db, &s1, None).unwrap();
        let mut s2 = scan(t.path());
        s2.status = ScanStatus::Cancelled;
        let b = save(&db, &s2, None).unwrap();
        assert_eq!(compare(&db, a, b).unwrap().warnings.len(), 1);
    }

    #[test]
    fn prune_limits_per_root() {
        let db = Db::open_in_memory().unwrap();
        let t = tempfile::tempdir().unwrap();
        let s = scan(t.path());
        for _ in 0..5 {
            save(&db, &s, None).unwrap();
        }
        prune(&db, 3650, 2).unwrap();
        assert_eq!(list(&db, None).unwrap().len(), 2);
    }
}
