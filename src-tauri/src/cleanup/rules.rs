//! Temizlik kategorileri ve korunan konumlar.
//!
//! Her kategori, davranışı bilinen ve yeniden oluşturulabilen verileri
//! hedefler. Bilinmeyen dosyalar asla "Güvenli" sayılmaz. Aşağıdakiler
//! bilerek **dahil edilmez**: Windows Update verileri, sistem geri yükleme
//! noktaları, `C:\Windows\Installer`, Prefetch, tarayıcı çerezleri / oturum /
//! parola verileri, kayıt defteri.

use crate::paths;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    /// Güvenli
    Safe,
    /// İncelenmeli
    Review,
    /// Riskli
    Risky,
}

#[derive(Debug, Clone)]
pub struct Target {
    pub path: PathBuf,
    /// Yalnızca bu önekle başlayan dosya adları (küçük harf).
    pub name_prefix: Option<&'static str>,
    /// Yalnızca bu uzantılar (boşsa tümü).
    pub extensions: Vec<String>,
    pub recursive: bool,
    /// Yalnızca en az bu kadar saat önce değiştirilmiş dosyalar.
    pub min_age_hours: u64,
}

impl Target {
    pub fn dir(path: PathBuf) -> Self {
        Self { path, name_prefix: None, extensions: vec![], recursive: true, min_age_hours: 0 }
    }
    pub fn aged(path: PathBuf, hours: u64) -> Self {
        Self { min_age_hours: hours, ..Self::dir(path) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Files,
    RecycleBin,
}

#[derive(Debug, Clone)]
pub struct Category {
    pub id: String,
    pub group: &'static str,
    pub name: String,
    pub description: &'static str,
    pub consequence: &'static str,
    pub risk: Risk,
    pub kind: Kind,
    pub targets: Vec<Target>,
    /// Bu süreçlerden biri çalışıyorsa kategori atlanır.
    pub blocking_processes: Vec<&'static str>,
    pub needs_admin: bool,
}

fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from).filter(|p| p.is_absolute())
}

fn local() -> Option<PathBuf> {
    env_path("LOCALAPPDATA")
}
fn roaming() -> Option<PathBuf> {
    env_path("APPDATA")
}

/// Chromium tabanlı tarayıcı profillerini bulur ("Default", "Profile 1"...).
fn chromium_profiles(user_data: &Path) -> Vec<(String, PathBuf)> {
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir(user_data) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if (name == "Default" || name.starts_with("Profile ") || name == "Guest Profile")
                && e.file_type().map(|t| t.is_dir()).unwrap_or(false)
            {
                v.push((name, e.path()));
            }
        }
    }
    v.sort();
    v
}

pub fn categories(temp_min_age_hours: u64, user_rules: &[crate::settings::UserCleanupRule]) -> Vec<Category> {
    let mut out = Vec::new();
    let windir = env_path("SystemRoot").unwrap_or_else(|| PathBuf::from("C:\\Windows"));

    // --- Windows -----------------------------------------------------------
    if let Some(t) = env_path("TEMP") {
        out.push(Category {
            id: "user-temp".into(),
            group: "Windows",
            name: "Kullanıcı geçici dosyaları".into(),
            description: "Uygulamaların %TEMP% klasörüne bıraktığı geçici dosyalar. Yalnızca belirtilen süreden daha eski dosyalar seçilir; çalışan uygulamaların kullandığı dosyalar kilitli olduğu için atlanır.",
            consequence: "Genellikle hiçbir etkisi olmaz. Yarım kalmış bir kurulumun geçici dosyaları silinirse kurulumun yeniden başlatılması gerekebilir.",
            risk: Risk::Safe,
            kind: Kind::Files,
            targets: vec![Target::aged(t, temp_min_age_hours)],
            blocking_processes: vec![],
            needs_admin: false,
        });
    }
    out.push(Category {
        id: "windows-temp".into(),
        group: "Windows",
        name: "Windows geçici dosyaları".into(),
        description: "Windows ve hizmetlerin C:\\Windows\\Temp klasöründeki geçici dosyaları. Çoğu dosya yönetici hakkı gerektirir; uygulama yönetici olarak çalışmıyorsa erişilemeyen dosyalar atlanır.",
        consequence: "Genellikle hiçbir etkisi olmaz. Kilitli dosyalar atlanır.",
        risk: Risk::Safe,
        kind: Kind::Files,
        targets: vec![Target::aged(windir.join("Temp"), temp_min_age_hours.max(24))],
        blocking_processes: vec![],
        needs_admin: true,
    });
    if let Some(l) = local() {
        out.push(Category {
            id: "thumbnail-cache".into(),
            group: "Windows",
            name: "Küçük resim ve simge önbelleği".into(),
            description: "Explorer'ın görsel önizlemeleri için tuttuğu thumbcache/iconcache dosyaları. Windows bunları gerektiğinde yeniden oluşturur.",
            consequence: "Klasörler ilk açıldığında önizlemeler kısa süre yeniden oluşturulur. Explorer'ın kullandığı dosyalar kilitli olduğu için atlanabilir.",
            risk: Risk::Safe,
            kind: Kind::Files,
            targets: vec![
                Target { name_prefix: Some("thumbcache_"), extensions: vec!["db".into()], recursive: false, ..Target::dir(l.join("Microsoft\\Windows\\Explorer")) },
                Target { name_prefix: Some("iconcache_"), extensions: vec!["db".into()], recursive: false, ..Target::dir(l.join("Microsoft\\Windows\\Explorer")) },
            ],
            blocking_processes: vec![],
            needs_admin: false,
        });
        let mut wer = vec![
            Target::dir(l.join("Microsoft\\Windows\\WER\\ReportArchive")),
            Target::dir(l.join("Microsoft\\Windows\\WER\\ReportQueue")),
        ];
        if let Some(pd) = env_path("ProgramData") {
            wer.push(Target::dir(pd.join("Microsoft\\Windows\\WER\\ReportArchive")));
            wer.push(Target::dir(pd.join("Microsoft\\Windows\\WER\\ReportQueue")));
        }
        out.push(Category {
            id: "error-reports".into(),
            group: "Windows",
            name: "Windows Hata Bildirimi raporları".into(),
            description: "Uygulama çökmeleri için oluşturulmuş, gönderilmiş veya kuyrukta bekleyen hata raporları.",
            consequence: "Henüz gönderilmemiş raporlar Microsoft'a iletilmez. Güvenilirlik geçmişi etkilenmez.",
            risk: Risk::Safe,
            kind: Kind::Files,
            targets: wer,
            blocking_processes: vec![],
            needs_admin: false,
        });
        out.push(Category {
            id: "crash-dumps".into(),
            group: "Windows",
            name: "Uygulama bellek dökümleri".into(),
            description: "Çöken uygulamaların %LOCALAPPDATA%\\CrashDumps klasörüne yazdığı döküm dosyaları. Geliştiriciler ve destek ekipleri hata ayıklamak için bunlara ihtiyaç duyabilir.",
            consequence: "Bir çökmeyi araştırıyorsanız gerekli bilgiler kaybolur.",
            risk: Risk::Review,
            kind: Kind::Files,
            targets: vec![Target::dir(l.join("CrashDumps"))],
            blocking_processes: vec![],
            needs_admin: false,
        });
        out.push(Category {
            id: "directx-shader-cache".into(),
            group: "Windows",
            name: "DirectX gölgelendirici önbelleği".into(),
            description: "Oyun ve uygulamaların derlenmiş gölgelendiricileri (D3DSCache). Gerektiğinde yeniden oluşturulur.",
            consequence: "Oyunlar ilk açılışta gölgelendiricileri yeniden derlerken kısa süreli takılmalar olabilir.",
            risk: Risk::Safe,
            kind: Kind::Files,
            targets: vec![Target::dir(l.join("D3DSCache"))],
            blocking_processes: vec![],
            needs_admin: false,
        });
        out.push(Category {
            id: "gpu-shader-cache".into(),
            group: "Windows",
            name: "Ekran kartı sürücüsü gölgelendirici önbelleği".into(),
            description: "NVIDIA, AMD ve Intel sürücülerinin gölgelendirici önbellekleri. Sürücü bunları yeniden oluşturur.",
            consequence: "Oyunlar ilk açılışta kısa süre daha yavaş yüklenebilir.",
            risk: Risk::Safe,
            kind: Kind::Files,
            targets: vec![
                Target::dir(l.join("NVIDIA\\DXCache")),
                Target::dir(l.join("NVIDIA\\GLCache")),
                Target::dir(l.join("AMD\\DxCache")),
                Target::dir(l.join("AMD\\DxcCache")),
                Target::dir(l.join("AMD\\GLCache")),
                Target::dir(l.join("AMD\\VkCache")),
                Target::dir(l.join("Intel\\ShaderCache")),
            ],
            blocking_processes: vec![],
            needs_admin: false,
        });
    }
    let logs = windir.join("Logs");
    out.push(Category {
        id: "windows-logs".into(),
        group: "Windows",
        name: "Windows bakım günlükleri".into(),
        description: "Windows bileşen (CBS) ve DISM bakım günlükleri. Yalnızca 7 günden eski .log ve arşivlenmiş .cab günlük dosyaları seçilir; Windows bu günlükleri yalnızca kayıt amacıyla tutar ve yenilerini kendisi oluşturur.",
        consequence: "Geçmiş bakım veya güncelleme sorunlarını incelemek için gereken günlükler kaybolur. Windows'un çalışması etkilenmez.",
        risk: Risk::Review,
        kind: Kind::Files,
        targets: vec![
            Target {
                extensions: vec!["log".into(), "cab".into()],
                recursive: false,
                min_age_hours: 7 * 24,
                ..Target::dir(logs.join("CBS"))
            },
            Target {
                extensions: vec!["log".into()],
                recursive: false,
                min_age_hours: 7 * 24,
                ..Target::dir(logs.join("DISM"))
            },
        ],
        blocking_processes: vec![],
        needs_admin: true,
    });
    out.push(Category {
        id: "recycle-bin".into(),
        group: "Windows",
        name: "Geri Dönüşüm Kutusu".into(),
        description: "Daha önce sildiğiniz ve Geri Dönüşüm Kutusu'nda bekleyen dosyalar.",
        consequence: "Geri Dönüşüm Kutusu'ndaki dosyalar kalıcı olarak silinir ve geri alınamaz. Önce içeriğini gözden geçirin.",
        risk: Risk::Review,
        kind: Kind::RecycleBin,
        targets: vec![],
        blocking_processes: vec![],
        needs_admin: false,
    });

    // --- Tarayıcılar ---------------------------------------------------------
    if let Some(l) = local() {
        let browsers: &[(&str, &str, &[&str])] = &[
            ("chrome", "Google Chrome", &["chrome.exe"]),
            ("edge", "Microsoft Edge", &["msedge.exe"]),
            ("brave", "Brave", &["brave.exe"]),
            ("vivaldi", "Vivaldi", &["vivaldi.exe"]),
        ];
        let dirs = [
            ("chrome", l.join("Google\\Chrome\\User Data")),
            ("edge", l.join("Microsoft\\Edge\\User Data")),
            ("brave", l.join("BraveSoftware\\Brave-Browser\\User Data")),
            ("vivaldi", l.join("Vivaldi\\User Data")),
        ];
        for (key, label, procs) in browsers {
            let Some((_, ud)) = dirs.iter().find(|(k, _)| k == key) else { continue };
            if !ud.exists() {
                continue;
            }
            let mut targets = vec![Target::dir(ud.join("ShaderCache")), Target::dir(ud.join("GrShaderCache"))];
            for (_, p) in chromium_profiles(ud) {
                targets.push(Target::dir(p.join("Cache\\Cache_Data")));
                targets.push(Target::dir(p.join("Code Cache")));
                targets.push(Target::dir(p.join("GPUCache")));
            }
            out.push(Category {
                id: format!("browser-{key}"),
                group: "Tarayıcılar",
                name: format!("{label} önbelleği"),
                description: "Web sayfalarının geçici olarak saklanan kopyaları, derlenmiş betikler ve GPU önbelleği. Çerezler, oturumlar, geçmiş, yer imleri ve kayıtlı parolalar etkilenmez.",
                consequence: "Siteler ilk ziyarette biraz daha yavaş açılabilir. Tarayıcı açıkken temizlik yapılmaz.",
                risk: Risk::Safe,
                kind: Kind::Files,
                targets,
                blocking_processes: procs.to_vec(),
                needs_admin: false,
            });
        }
        let ff = l.join("Mozilla\\Firefox\\Profiles");
        if let Ok(rd) = std::fs::read_dir(&ff) {
            let targets: Vec<Target> = rd
                .flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| Target::dir(e.path().join("cache2")))
                .collect();
            if !targets.is_empty() {
                out.push(Category {
                    id: "browser-firefox".into(),
                    group: "Tarayıcılar",
                    name: "Mozilla Firefox önbelleği".into(),
                    description: "Firefox'un disk önbelleği (cache2). Çerezler, oturumlar, geçmiş ve parolalar etkilenmez.",
                    consequence: "Siteler ilk ziyarette biraz daha yavaş açılabilir. Firefox açıkken temizlik yapılmaz.",
                    risk: Risk::Safe,
                    kind: Kind::Files,
                    targets,
                    blocking_processes: vec!["firefox.exe"],
                    needs_admin: false,
                });
            }
        }
    }

    // --- Uygulamalar ---------------------------------------------------------
    if let Some(r) = roaming() {
        let electron: &[(&str, &str, &str, &[&str])] = &[
            ("discord", "Discord", "discord", &["discord.exe"]),
            ("vscode", "Visual Studio Code", "Code", &["code.exe"]),
            ("slack", "Slack", "Slack", &["slack.exe"]),
        ];
        for (id, label, folder, procs) in electron {
            let base = r.join(folder);
            if !base.exists() {
                continue;
            }
            out.push(Category {
                id: format!("app-{id}"),
                group: "Uygulamalar",
                name: format!("{label} önbelleği"),
                description: "Uygulamanın web içeriği önbelleği (Cache, Code Cache, GPUCache). Hesap, ayar ve mesaj verileri etkilenmez.",
                consequence: "Uygulama bir sonraki açılışta önbelleği yeniden oluşturur. Uygulama açıkken temizlik yapılmaz.",
                risk: Risk::Safe,
                kind: Kind::Files,
                targets: vec![
                    Target::dir(base.join("Cache\\Cache_Data")),
                    Target::dir(base.join("Code Cache")),
                    Target::dir(base.join("GPUCache")),
                ],
                blocking_processes: procs.to_vec(),
                needs_admin: false,
            });
        }
    }
    if let Some(l) = local() {
        let steam = l.join("Steam\\htmlcache");
        if steam.exists() {
            out.push(Category {
                id: "app-steam-web".into(),
                group: "Oyunlar",
                name: "Steam tarayıcı önbelleği".into(),
                description: "Steam istemcisinin mağaza ve topluluk sayfaları için kullandığı web önbelleği. Oyunlar ve kayıtlar etkilenmez.",
                consequence: "Steam mağazası ilk açılışta biraz daha yavaş yüklenebilir. Steam açıkken temizlik yapılmaz.",
                risk: Risk::Safe,
                kind: Kind::Files,
                targets: vec![Target::dir(steam)],
                blocking_processes: vec!["steam.exe", "steamwebhelper.exe"],
                needs_admin: false,
            });
        }
    }

    // --- Geliştirme ----------------------------------------------------------
    let home = dirs::home_dir();
    let mut dev: Vec<(&str, &str, Option<PathBuf>, &'static str, &[&str])> = vec![
        ("npm", "npm önbelleği", local().map(|l| l.join("npm-cache\\_cacache")), "npm'in indirilen paketler için tuttuğu önbellek. Gerektiğinde yeniden indirilir. Önerilen yol `npm cache clean --force` komutudur.", &["node.exe"]),
        ("yarn", "Yarn önbelleği", local().map(|l| l.join("Yarn\\Cache")), "Yarn paket önbelleği. Gerektiğinde yeniden indirilir.", &["node.exe"]),
        ("pip", "pip önbelleği", local().map(|l| l.join("pip\\Cache")), "Python pip'in indirme ve derleme önbelleği. Gerektiğinde yeniden indirilir.", &["pip.exe", "python.exe"]),
        ("cargo", "Cargo indirme önbelleği", home.as_ref().map(|h| h.join(".cargo\\registry\\cache")), "Rust Cargo'nun indirdiği sıkıştırılmış .crate dosyaları. Çevrimdışı derleme yapıyorsanız gerekli olabilir.", &["cargo.exe"]),
        ("nuget-http", "NuGet HTTP önbelleği", local().map(|l| l.join("NuGet\\v3-cache")), "NuGet'in paket kaynaklarından aldığı yanıtların önbelleği. Paketlerin kendisi etkilenmez.", &["dotnet.exe", "devenv.exe"]),
        ("go-build", "Go derleme önbelleği", local().map(|l| l.join("go-build")), "Go derleyicisinin derleme önbelleği. `go clean -cache` ile aynı etkiye sahiptir.", &["go.exe"]),
    ];
    for (id, name, path, desc, procs) in dev.drain(..) {
        let Some(p) = path else { continue };
        if !p.exists() {
            continue;
        }
        out.push(Category {
            id: format!("dev-{id}"),
            group: "Geliştirme",
            name: name.into(),
            description: desc,
            consequence: "Sonraki derleme veya kurulumda paketler yeniden indirilir; internet bağlantısı gerekir ve işlem daha uzun sürer.",
            risk: Risk::Review,
            kind: Kind::Files,
            targets: vec![Target::dir(p)],
            blocking_processes: procs.to_vec(),
            needs_admin: false,
        });
    }

    // --- Kullanıcı kuralları -------------------------------------------------
    for r in user_rules {
        let p = PathBuf::from(&r.path);
        if !p.is_absolute() || is_protected_path(&r.path) {
            continue;
        }
        out.push(Category {
            id: format!("user-{}", r.id),
            group: "Özel kurallar",
            name: r.name.clone(),
            description: "Sizin tanımladığınız bir temizlik kuralı. Ivily Optimizer bu konumun içeriğini tanımaz; seçmeden önce gözden geçirin.",
            consequence: "Bu konumdaki eşleşen dosyalar silinir. Etkisi, klasörü hangi uygulamanın kullandığına bağlıdır.",
            risk: Risk::Review,
            kind: Kind::Files,
            targets: vec![Target {
                extensions: r.extensions.clone(),
                min_age_hours: r.min_age_days as u64 * 24,
                ..Target::dir(p)
            }],
            blocking_processes: vec![],
            needs_admin: false,
        });
    }
    out
}

/// Dar istisna: yalnızca `Windows\Logs\CBS` ve `Windows\Logs\DISM`
/// klasörlerinin **doğrudan** içindeki `.log`/`.cab` dosyaları. Klasörlerin
/// kendisi ve alt klasörleri korunmaya devam eder.
fn is_allowed_windows_log(normalized: &str, windir: &str) -> bool {
    let Some((parent, name)) = normalized.rsplit_once('\\') else { return false };
    let ext = crate::scan::walker::extension_of(name);
    if ext != "log" && ext != "cab" {
        return false;
    }
    let w = paths::normalize(windir);
    parent == format!("{w}\\logs\\cbs") || parent == format!("{w}\\logs\\dism")
}

/// Hiçbir koşulda silinmemesi, taşınmaması veya temizlik kuralına konu
/// edilmemesi gereken konumlar.
pub fn is_protected_path(path: &str) -> bool {
    let p = paths::normalize(path);
    if p.is_empty() {
        return true;
    }
    // Sürücü kökleri (ör. "c:\" veya "c:").
    if p.len() <= 3 && p.chars().nth(1) == Some(':') {
        return true;
    }
    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    if is_allowed_windows_log(&p, &windir) {
        return false;
    }
    let windir_temp = format!("{windir}\\Temp");
    if paths::is_within(&p, &windir) && !paths::is_within(&p, &windir_temp) {
        return true;
    }
    if paths::normalize(&windir_temp) == p {
        return true; // klasörün kendisi
    }
    for var in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
        if let Ok(v) = std::env::var(var) {
            if paths::is_within(&p, &v) {
                return true;
            }
        }
    }
    let drive = &p[..2.min(p.len())];
    for sys in ["system volume information", "$recycle.bin", "recovery", "boot", "$windows.~bt", "$windows.~ws", "$winreagent"] {
        if paths::is_within(&p, &format!("{drive}\\{sys}")) {
            return true;
        }
    }
    for f in ["pagefile.sys", "hiberfil.sys", "swapfile.sys", "bootmgr", "bootnxt", "dumpstack.log", "dumpstack.log.tmp"] {
        if p == format!("{drive}\\{f}") {
            return true;
        }
    }
    if let Ok(pd) = std::env::var("ProgramData") {
        let pdm = format!("{pd}\\Microsoft");
        let wer = format!("{pd}\\Microsoft\\Windows\\WER");
        if paths::is_within(&p, &pdm) && !(paths::is_within(&p, &wer) && p != paths::normalize(&wer)) {
            return true;
        }
        if paths::normalize(&pd) == p {
            return true;
        }
    }
    // Klasörlerin kendisi (içerikleri değil).
    let exact: Vec<Option<PathBuf>> = vec![
        dirs::home_dir(),
        local(),
        roaming(),
        env_path("TEMP"),
        dirs::document_dir(),
        dirs::desktop_dir(),
        dirs::download_dir(),
        dirs::picture_dir(),
        dirs::video_dir(),
        dirs::audio_dir(),
        env_path("PUBLIC"),
        dirs::home_dir().and_then(|h| h.parent().map(Path::to_path_buf)),
    ];
    if exact.into_iter().flatten().any(|e| paths::normalize(&e.to_string_lossy()) == p) {
        return true;
    }
    // Ivily Optimizer'ın kendi verileri.
    if paths::is_within(&p, &crate::paths::data_dir().to_string_lossy()) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protects_system_locations() {
        let windir = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        assert!(is_protected_path("C:\\"));
        assert!(is_protected_path("c:"));
        assert!(is_protected_path(&format!("{windir}\\System32\\drivers\\x.sys")));
        assert!(is_protected_path(&format!("{windir}\\Temp")));
        assert!(!is_protected_path(&format!("{windir}\\Temp\\a.tmp")));
        assert!(is_protected_path("C:\\pagefile.sys"));
        assert!(is_protected_path("C:\\System Volume Information\\x"));
        assert!(is_protected_path(&std::env::var("ProgramFiles").unwrap()));
        assert!(is_protected_path(&dirs::home_dir().unwrap().to_string_lossy()));
        assert!(!is_protected_path(&dirs::home_dir().unwrap().join("Downloads\\a.zip").to_string_lossy()));
        assert!(is_protected_path(&crate::paths::db_path().to_string_lossy()));
        assert!(is_protected_path(""));
    }

    #[test]
    fn wer_inside_programdata_is_allowed_but_rest_is_not() {
        let pd = std::env::var("ProgramData").unwrap();
        assert!(!is_protected_path(&format!("{pd}\\Microsoft\\Windows\\WER\\ReportArchive\\x\\r.wer")));
        assert!(is_protected_path(&format!("{pd}\\Microsoft\\Windows Defender\\x")));
    }

    #[test]
    fn windows_log_exception_is_narrow() {
        let w = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        assert!(!is_protected_path(&format!("{w}\\Logs\\CBS\\CbsPersist_20240101.log")));
        assert!(!is_protected_path(&format!("{w}\\Logs\\CBS\\CbsPersist_20240101.cab")));
        assert!(!is_protected_path(&format!("{w}\\logs\\dism\\dism.log")));
        assert!(is_protected_path(&format!("{w}\\Logs\\CBS")));
        assert!(is_protected_path(&format!("{w}\\Logs\\CBS\\")));
        assert!(is_protected_path(&format!("{w}\\Logs\\DISM")));
        assert!(is_protected_path(&format!("{w}\\Logs\\x.log")));
        assert!(is_protected_path(&format!("{w}\\Logs\\CBS\\alt\\x.log")));
        assert!(is_protected_path(&format!("{w}\\Logs\\CBS\\x.dll")));
        assert!(is_protected_path(&format!("{w}\\Logs\\MeasuredBoot\\x.log")));
        assert!(is_protected_path("C:\\Windows\\Logs\\CBS"));
        assert!(is_protected_path("C:\\Windows\\Logs\\x.log"));
        let c = categories(24, &[]).into_iter().find(|c| c.id == "windows-logs").unwrap();
        assert_eq!(c.risk, Risk::Review);
        assert!(c.needs_admin);
        assert!(c.targets.iter().all(|t| !t.recursive && t.min_age_hours == 168));
    }

    #[test]
    fn categories_never_target_protected_roots() {
        for c in categories(24, &[]) {
            for t in &c.targets {
                let s = t.path.to_string_lossy();
                let ext = t.extensions.first().map(String::as_str).unwrap_or("tmp");
                assert!(!is_protected_path(&format!("{s}\\ornek.{ext}")), "{} korunan bir yolu hedefliyor: {s}", c.id);
            }
        }
    }
}
