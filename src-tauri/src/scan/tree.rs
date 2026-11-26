//! Disk analizi: klasör ağacı, dosya türü dağılımı, en büyük dosyalar ve
//! küçük dosya yoğunluğu analizi.

use super::classify;
use super::walker::{self, DirVisit, ScanErrorItem, WalkOptions, WalkProgress};
use parking_lot::Mutex;
use serde::Serialize;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Dosya boyutu histogramı sınırları (bayt). Son kova üst sınırsızdır.
pub const HIST_BOUNDS: [u64; 7] = [1 << 10, 4 << 10, 16 << 10, 64 << 10, 256 << 10, 1 << 20, 16 << 20];
pub const HIST_LABELS: [&str; 8] =
    ["< 1 KB", "1–4 KB", "4–16 KB", "16–64 KB", "64–256 KB", "256 KB–1 MB", "1–16 MB", "> 16 MB"];

fn bucket(size: u64) -> usize {
    HIST_BOUNDS.iter().position(|b| size < *b).unwrap_or(HIST_BOUNDS.len())
}

#[derive(Debug, Clone, Default)]
pub struct DirNode {
    pub name: String,
    pub parent: u32,
    pub depth: u16,
    pub modified: i64,
    /// Bu klasörün doğrudan içerdiği dosyalar.
    pub own_files: u64,
    pub own_size: u64,
    /// Alt klasörler dahil toplamlar.
    pub size: u64,
    pub files: u64,
    pub dirs: u64,
    pub small_files: u64,
    pub newest_file: i64,
    pub max_depth_below: u16,
    pub hist: [u64; 8],
    pub children: Vec<u32>,
    pub skipped_links: u32,
    pub unreadable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeFile {
    pub path: String,
    pub size: u64,
    pub modified: i64,
    pub ext: String,
    pub category: String,
    pub cloud: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtStat {
    pub ext: String,
    pub category: String,
    pub count: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanStatus {
    Complete,
    Cancelled,
    Failed,
}

impl ScanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScanStatus::Complete => "complete",
            ScanStatus::Cancelled => "cancelled",
            ScanStatus::Failed => "failed",
        }
    }
}

pub struct DiskScan {
    pub root: PathBuf,
    pub started_at: i64,
    pub duration_ms: u64,
    pub status: ScanStatus,
    pub nodes: Vec<DirNode>,
    pub exts: HashMap<String, ExtStat>,
    pub largest: Vec<LargeFile>,
    pub errors: Vec<ScanErrorItem>,
    pub error_count: u64,
    pub small_threshold: u64,
    pub cloud_bytes: u64,
}

pub const TOP_FILES: usize = 1000;

/// En büyük dosyalar yığınındaki öğe: boyut, yol, değişiklik zamanı, bulut yer tutucusu mu.
type TopFile = (u64, String, i64, bool);

struct Collector {
    nodes: Mutex<Vec<DirNode>>,
    exts: Mutex<HashMap<String, ExtStat>>,
    heap: Mutex<BinaryHeap<Reverse<TopFile>>>,
    heap_min: AtomicU64,
    cloud_bytes: AtomicU64,
    small_threshold: u64,
}

pub fn scan(
    root: &Path,
    opts: &WalkOptions,
    small_threshold: u64,
    cancel: &AtomicBool,
    progress: &WalkProgress,
) -> DiskScan {
    let started_at = chrono::Utc::now().timestamp();
    let t0 = std::time::Instant::now();
    let col = Collector {
        nodes: Mutex::new(Vec::with_capacity(4096)),
        exts: Mutex::new(HashMap::new()),
        heap: Mutex::new(BinaryHeap::with_capacity(TOP_FILES + 1)),
        heap_min: AtomicU64::new(0),
        cloud_bytes: AtomicU64::new(0),
        small_threshold,
    };

    let outcome = walker::walk(root, opts, cancel, progress, &|v: DirVisit| visit(&col, v));

    let mut nodes = col.nodes.into_inner();
    aggregate(&mut nodes);

    let mut largest: Vec<LargeFile> = col
        .heap
        .into_inner()
        .into_iter()
        .map(|Reverse((size, path, modified, cloud))| {
            let ext = walker::extension_of(&path);
            LargeFile { category: classify::category_of(&ext, size).to_string(), ext, path, size, modified, cloud }
        })
        .collect();
    largest.sort_by_key(|e| std::cmp::Reverse(e.size));

    let status = if outcome.cancelled {
        ScanStatus::Cancelled
    } else if nodes.first().map(|n| n.unreadable).unwrap_or(true) {
        ScanStatus::Failed
    } else {
        ScanStatus::Complete
    };

    DiskScan {
        root: root.to_path_buf(),
        started_at,
        duration_ms: t0.elapsed().as_millis() as u64,
        status,
        nodes,
        exts: col.exts.into_inner(),
        largest,
        errors: outcome.errors,
        error_count: outcome.error_count,
        small_threshold,
        cloud_bytes: col.cloud_bytes.into_inner(),
    }
}

fn visit(col: &Collector, v: DirVisit) -> u32 {
    let mut node = DirNode {
        name: v.name.to_string(),
        parent: v.parent,
        depth: v.depth,
        modified: v.modified,
        skipped_links: v.skipped_links,
        unreadable: v.unreadable,
        ..Default::default()
    };
    let mut local_ext: HashMap<String, (u64, u64)> = HashMap::new();
    let mut big: Vec<(u64, String, i64, bool)> = Vec::new();
    let heap_min = col.heap_min.load(Ordering::Relaxed);
    let mut cloud = 0u64;

    for f in &v.files {
        node.own_files += 1;
        node.own_size += f.size;
        node.hist[bucket(f.size)] += 1;
        if f.size < col.small_threshold {
            node.small_files += 1;
        }
        node.newest_file = node.newest_file.max(f.modified);
        let e = local_ext.entry(f.extension()).or_default();
        e.0 += 1;
        e.1 += f.size;
        let is_cloud = f.is_cloud_placeholder();
        if is_cloud {
            cloud += f.size;
        }
        if f.size > heap_min {
            big.push((f.size, v.path.join(&f.name).to_string_lossy().into_owned(), f.modified, is_cloud));
        }
    }
    if cloud > 0 {
        col.cloud_bytes.fetch_add(cloud, Ordering::Relaxed);
    }
    if !local_ext.is_empty() {
        let mut exts = col.exts.lock();
        for (ext, (count, size)) in local_ext {
            let e = exts.entry(ext.clone()).or_insert_with(|| ExtStat { ext, ..Default::default() });
            e.count += count;
            e.size += size;
        }
    }
    if !big.is_empty() {
        let mut heap = col.heap.lock();
        for item in big {
            heap.push(Reverse(item));
            if heap.len() > TOP_FILES {
                heap.pop();
            }
        }
        if heap.len() >= TOP_FILES {
            if let Some(Reverse((min, ..))) = heap.peek() {
                col.heap_min.store(*min, Ordering::Relaxed);
            }
        }
    }

    let mut nodes = col.nodes.lock();
    nodes.push(node);
    (nodes.len() - 1) as u32
}

/// Alt klasör toplamlarını üst klasörlere yansıtır ve çocuk listelerini kurar.
fn aggregate(nodes: &mut [DirNode]) {
    for n in nodes.iter_mut() {
        n.size = n.own_size;
        n.files = n.own_files;
    }
    // Derinliğe göre azalan sırada işlemek, her düğümün üst düğümünden önce
    // tamamlanmasını garanti eder.
    let mut order: Vec<u32> = (0..nodes.len() as u32).collect();
    order.sort_by_key(|&i| Reverse(nodes[i as usize].depth));
    for &i in &order {
        let parent = nodes[i as usize].parent;
        if parent == u32::MAX || parent as usize >= nodes.len() {
            continue;
        }
        let c = nodes[i as usize].clone_stats();
        let p = &mut nodes[parent as usize];
        p.size += c.size;
        p.files += c.files;
        p.dirs += c.dirs + 1;
        p.small_files += c.small_files;
        p.newest_file = p.newest_file.max(c.newest_file);
        p.max_depth_below = p.max_depth_below.max(c.max_depth_below + 1);
        for k in 0..8 {
            p.hist[k] += c.hist[k];
        }
        p.children.push(i);
    }
    for n in nodes.iter_mut() {
        n.children.shrink_to_fit();
    }
}

struct Stats {
    size: u64,
    files: u64,
    dirs: u64,
    small_files: u64,
    newest_file: i64,
    max_depth_below: u16,
    hist: [u64; 8],
}

impl DirNode {
    fn clone_stats(&self) -> Stats {
        Stats {
            size: self.size,
            files: self.files,
            dirs: self.dirs,
            small_files: self.small_files,
            newest_file: self.newest_file,
            max_depth_below: self.max_depth_below,
            hist: self.hist,
        }
    }
}

// ---------------------------------------------------------------------------
// Sorgular
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeView {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub files: u64,
    pub dirs: u64,
    pub own_files: u64,
    pub own_size: u64,
    pub modified: i64,
    pub newest_file: i64,
    pub has_children: bool,
    pub unreadable: bool,
    pub skipped_links: u32,
}

impl DiskScan {
    pub fn path_of(&self, id: u32) -> PathBuf {
        let mut parts = Vec::new();
        let mut cur = id;
        while let Some(n) = self.nodes.get(cur as usize) {
            if n.parent == u32::MAX {
                break;
            }
            parts.push(n.name.as_str());
            cur = n.parent;
        }
        let mut p = self.root.clone();
        for part in parts.iter().rev() {
            p.push(part);
        }
        p
    }

    pub fn view(&self, id: u32) -> Option<NodeView> {
        let n = self.nodes.get(id as usize)?;
        Some(NodeView {
            id,
            name: if n.parent == u32::MAX { self.root.to_string_lossy().into_owned() } else { n.name.clone() },
            path: self.path_of(id).to_string_lossy().into_owned(),
            size: n.size,
            files: n.files,
            dirs: n.dirs,
            own_files: n.own_files,
            own_size: n.own_size,
            modified: n.modified,
            newest_file: n.newest_file,
            has_children: !n.children.is_empty(),
            unreadable: n.unreadable,
            skipped_links: n.skipped_links,
        })
    }

    pub fn children(&self, id: u32) -> Vec<NodeView> {
        let Some(n) = self.nodes.get(id as usize) else { return vec![] };
        let mut v: Vec<NodeView> = n.children.iter().filter_map(|&c| self.view(c)).collect();
        v.sort_by_key(|e| std::cmp::Reverse(e.size));
        v
    }

    /// Kökten itibaren verilen yolun düğüm kimliğini bulur.
    pub fn find(&self, path: &str) -> Option<u32> {
        let root = crate::paths::normalize(&self.root.to_string_lossy());
        let target = crate::paths::normalize(path);
        if target == root {
            return Some(0);
        }
        let rest = target.strip_prefix(&root)?.trim_start_matches('\\');
        let mut cur = 0u32;
        for part in rest.split('\\').filter(|s| !s.is_empty()) {
            let n = self.nodes.get(cur as usize)?;
            cur = *n.children.iter().find(|&&c| self.nodes[c as usize].name.to_lowercase() == part)?;
        }
        Some(cur)
    }

    pub fn largest_dirs(&self, limit: usize, max_depth: u16) -> Vec<NodeView> {
        let mut ids: Vec<u32> =
            (1..self.nodes.len() as u32).filter(|&i| self.nodes[i as usize].depth <= max_depth).collect();
        ids.sort_by(|a, b| self.nodes[*b as usize].size.cmp(&self.nodes[*a as usize].size));
        ids.truncate(limit);
        ids.into_iter().filter_map(|i| self.view(i)).collect()
    }

    pub fn ext_stats(&self) -> Vec<ExtStat> {
        let mut v: Vec<ExtStat> = self
            .exts
            .values()
            .map(|e| ExtStat {
                category: classify::category_of(&e.ext, e.size.checked_div(e.count).unwrap_or(0)).to_string(),
                ..e.clone()
            })
            .collect();
        v.sort_by_key(|e| std::cmp::Reverse(e.size));
        v
    }

    pub fn category_stats(&self) -> Vec<ExtStat> {
        let mut m: HashMap<String, ExtStat> = HashMap::new();
        for e in self.ext_stats() {
            let c = m.entry(e.category.clone()).or_insert_with(|| ExtStat {
                ext: String::new(),
                category: e.category.clone(),
                ..Default::default()
            });
            c.count += e.count;
            c.size += e.size;
        }
        let mut v: Vec<ExtStat> = m.into_values().collect();
        v.sort_by_key(|e| std::cmp::Reverse(e.size));
        v
    }

    /// Tek bir klasör alt ağacının dosya boyutu histogramı.
    pub fn hist(&self, id: u32) -> [u64; 8] {
        self.nodes.get(id as usize).map(|n| n.hist).unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Küçük dosya yoğunluğu ("küçük dosya mezarlığı") analizi
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmallFileHotspot {
    pub id: u32,
    pub path: String,
    pub total_files: u64,
    pub small_files: u64,
    pub small_pct: f64,
    pub total_size: u64,
    pub avg_size: u64,
    /// Histogramdan elde edilen yaklaşık medyan kovası.
    pub median_bucket: String,
    pub hist: [u64; 8],
    pub dirs: u64,
    pub max_depth: u16,
    pub newest_file: i64,
    pub top_exts: Vec<(String, u64)>,
    /// Alt klasör yoğunluğu: dosya / klasör.
    pub files_per_dir: f64,
    pub impact: String,
    pub kind: Option<String>,
    pub suggestion: String,
}

/// Bilinen klasör adlarına göre muhafazakâr bir açıklama döndürür.
pub fn recognize_dir(path: &str) -> Option<(&'static str, &'static str)> {
    let lower = path.to_lowercase();
    let last = lower.rsplit('\\').next().unwrap_or("");
    let has = |seg: &str| lower.contains(&format!("\\{seg}\\")) || last == seg;
    if last == "node_modules" {
        return Some(("node_modules", "Bir JavaScript projesinin bağımlılıkları. Proje artık kullanılmıyorsa proje klasöründe silinip gerektiğinde `npm install` ile yeniden oluşturulabilir."));
    }
    if (last == "target"
        && (std::path::Path::new(path).parent().map(|p| p.join("Cargo.toml").exists()).unwrap_or(false)))
        || has("target\\debug")
    {
        return Some((
            "rust-target",
            "Rust derleme çıktıları. `cargo clean` ile güvenle silinebilir ve gerektiğinde yeniden derlenir.",
        ));
    }
    if last == ".git" || last == "objects" && lower.contains("\\.git\\") {
        return Some(("git", "Git deposu verisi. Silmeyin; `git gc` ile sıkıştırılabilir."));
    }
    if last == "__pycache__" || last == ".venv" || last == "venv" || last == "site-packages" {
        return Some((
            "python",
            "Python ortamı veya önbelleği. Sanal ortamlar proje klasöründen yeniden oluşturulabilir.",
        ));
    }
    if has("npm-cache")
        || has("yarn\\cache")
        || has("pnpm-store")
        || has(".nuget\\packages")
        || has("pip\\cache")
        || has(".gradle")
        || has(".m2")
        || has(".cargo\\registry")
    {
        return Some((
            "package-cache",
            "Paket yöneticisi önbelleği. Kendi aracıyla temizlenmesi önerilir (ör. `npm cache clean`).",
        ));
    }
    if has("winsxs") || has("system32") || has("windows\\installer") {
        return Some(("windows", "Windows sistem bileşenleri. Elle değiştirilmemelidir."));
    }
    if has("cache") || has("caches") || has("code cache") || has("gpucache") {
        return Some(("cache", "Uygulama önbelleği gibi görünüyor. Uygulamanın kendi temizleme seçeneğini kullanın."));
    }
    None
}

pub fn small_file_hotspots(scan: &DiskScan, min_count: u64, limit: usize) -> Vec<SmallFileHotspot> {
    let nodes = &scan.nodes;
    let mut out = Vec::new();
    for (i, n) in nodes.iter().enumerate() {
        if n.small_files < min_count {
            continue;
        }
        // Küçük dosyaların %80'inden fazlası tek bir alt klasördeyse, asıl
        // yoğunluk o alt klasördedir; bu klasörü ayrıca listeleme.
        let dominated = n.children.iter().any(|&c| nodes[c as usize].small_files * 10 >= n.small_files * 8);
        if dominated {
            continue;
        }
        let path = scan.path_of(i as u32).to_string_lossy().into_owned();
        let small_pct = if n.files > 0 { n.small_files as f64 * 100.0 / n.files as f64 } else { 0.0 };
        let avg = n.size.checked_div(n.files).unwrap_or(0);
        let median_bucket = {
            let half = n.files.div_ceil(2);
            let mut acc = 0;
            let mut label = HIST_LABELS[0];
            for k in 0..8 {
                acc += n.hist[k];
                if acc >= half {
                    label = HIST_LABELS[k];
                    break;
                }
            }
            label.to_string()
        };
        let files_per_dir = n.files as f64 / (n.dirs + 1) as f64;
        let impact = if n.small_files >= min_count * 10 && small_pct >= 70.0 {
            "Yüksek"
        } else if small_pct >= 50.0 {
            "Orta"
        } else {
            "Düşük"
        };
        let rec = recognize_dir(&path);
        let suggestion = match rec {
            Some((_, s)) => s.to_string(),
            None => "Klasörün hangi uygulamaya ait olduğunu inceleyin. Tanınmayan klasörleri silmeyin; gerekirse yedekleme ve dizin oluşturma (indeksleme) kapsamından çıkarmayı değerlendirin.".to_string(),
        };
        out.push(SmallFileHotspot {
            id: i as u32,
            path,
            total_files: n.files,
            small_files: n.small_files,
            small_pct,
            total_size: n.size,
            avg_size: avg,
            median_bucket,
            hist: n.hist,
            dirs: n.dirs,
            max_depth: n.max_depth_below,
            newest_file: n.newest_file,
            top_exts: vec![],
            files_per_dir,
            impact: impact.to_string(),
            kind: rec.map(|(k, _)| k.to_string()),
            suggestion,
        });
    }
    out.sort_by_key(|e| std::cmp::Reverse(e.small_files));
    out.truncate(limit);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn run(root: &Path, thr: u64) -> DiskScan {
        scan(root, &WalkOptions::new(2), thr, &AtomicBool::new(false), &WalkProgress::default())
    }

    #[test]
    fn aggregates_sizes_and_counts() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        fs::create_dir_all(r.join("a/b")).unwrap();
        fs::create_dir_all(r.join("c")).unwrap();
        fs::write(r.join("a/1.txt"), vec![0u8; 10]).unwrap();
        fs::write(r.join("a/b/2.jpg"), vec![0u8; 2000]).unwrap();
        fs::write(r.join("c/3.mkv"), vec![0u8; 5000]).unwrap();
        let s = run(r, 1024);
        assert_eq!(s.status, ScanStatus::Complete);
        let root = &s.nodes[0];
        assert_eq!(root.size, 7010);
        assert_eq!(root.files, 3);
        assert_eq!(root.dirs, 3);
        assert_eq!(root.small_files, 1);
        let a = s.find(&r.join("a").to_string_lossy()).unwrap();
        assert_eq!(s.nodes[a as usize].size, 2010);
        assert_eq!(s.path_of(a), r.join("a"));
        let kids = s.children(0);
        assert_eq!(kids[0].size, 5000); // en büyük önce
        assert_eq!(s.largest[0].size, 5000);
        assert_eq!(s.largest[0].category, "video");
        let cats = s.category_stats();
        assert_eq!(cats.iter().map(|c| c.size).sum::<u64>(), 7010);
        assert_eq!(root.hist.iter().sum::<u64>(), 3);
    }

    #[test]
    fn finds_small_file_hotspot_at_right_level() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        let dense = r.join("proj/node_modules/pkg");
        fs::create_dir_all(&dense).unwrap();
        for i in 0..150 {
            fs::write(dense.join(format!("f{i}.js")), b"x").unwrap();
        }
        fs::write(r.join("proj/big.bin"), vec![0u8; 50_000]).unwrap();
        let s = run(r, 1024);
        let hs = small_file_hotspots(&s, 100, 10);
        assert_eq!(hs.len(), 1, "yalnızca en yoğun düzey listelenmeli");
        assert!(hs[0].path.ends_with("pkg"));
        assert_eq!(hs[0].small_files, 150);
        assert!((hs[0].small_pct - 100.0).abs() < 0.01);
    }

    #[test]
    fn recognizes_known_dirs() {
        assert_eq!(recognize_dir("C:\\p\\node_modules").unwrap().0, "node_modules");
        assert!(recognize_dir("C:\\Users\\me\\Belgeler").is_none());
    }
}
