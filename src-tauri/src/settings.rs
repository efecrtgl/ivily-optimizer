//! Kullanıcı ayarları. Veritabanında JSON olarak saklanır, yüklenirken ve
//! kaydedilirken doğrulanır.

use crate::db::Db;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Exclusions {
    /// Taramalarda atlanacak klasör ve dosya yolları.
    pub paths: Vec<String>,
    /// Taramalarda atlanacak uzantılar (noktasız, küçük harf).
    pub extensions: Vec<String>,
    /// Temizlik taramasına dahil edilmeyecek kategori kimlikleri.
    pub cleanup_categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct UserCleanupRule {
    pub id: String,
    pub name: String,
    pub path: String,
    /// Yalnızca bu uzantılar (boşsa tümü).
    pub extensions: Vec<String>,
    /// Yalnızca en az bu kadar gün önce değiştirilmiş dosyalar.
    pub min_age_days: u32,
}

impl Default for UserCleanupRule {
    fn default() -> Self {
        Self { id: String::new(), name: String::new(), path: String::new(), extensions: vec![], min_age_days: 7 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    // Görünüm
    pub theme: String,
    /// Arayüz dili: "tr" (birincil) veya "en".
    pub language: String,
    pub reduce_motion: bool,
    pub compact_mode: bool,
    /// Cam (saydamlık + bulanıklık) efektleri. Kapalıyken düz yüzeyler kullanılır.
    pub glass_effects: bool,
    /// Panodaki bileşenlerin görünürlüğü ve sırası.
    pub dashboard_widgets: Vec<String>,
    /// Geniş (iki sütun) gösterilen bileşenler.
    pub dashboard_wide: Vec<String>,
    pub onboarding_done: bool,
    // Genel
    pub launch_at_startup: bool,
    pub notifications: bool,
    // İzleme
    pub monitor_interval_ms: u64,
    pub monitor_in_background: bool,
    // Tarama
    pub scan_threads: usize,
    pub small_file_threshold_kb: u64,
    pub small_file_min_count: u64,
    pub large_file_min_mb: u64,
    pub exclusions: Exclusions,
    // Temizlik
    pub cleanup_use_quarantine: bool,
    pub temp_min_age_hours: u64,
    pub user_rules: Vec<UserCleanupRule>,
    // Karantina ve geçmiş
    pub quarantine_retention_days: u32,
    pub snapshot_retention_days: u32,
    pub snapshot_max_count: u32,
    pub history_retention_days: u32,
    // Güncellemeler
    pub update_check_on_start: bool,
    pub update_check_interval_hours: u32,
    pub update_include_prerelease: bool,
}

pub const ALL_WIDGETS: &[&str] = &[
    "welcome",
    "quick",
    "recommend",
    "health",
    "hardware",
    "cpu",
    "memory",
    "disk",
    "storage",
    "network",
    "cleanup",
    "largest",
    "growth",
    "startup",
    "events",
    "notifications",
    "recent",
    "update",
    "shortcuts",
];

/// Varsayılan pano düzeni (sıra önemlidir).
pub const DEFAULT_WIDGETS: &[&str] =
    &["welcome", "quick", "recommend", "cpu", "memory", "disk", "storage", "events", "recent", "update"];

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            language: "tr".into(),
            reduce_motion: false,
            compact_mode: false,
            glass_effects: true,
            dashboard_widgets: DEFAULT_WIDGETS.iter().map(|s| s.to_string()).collect(),
            dashboard_wide: vec!["welcome".into(), "quick".into(), "recommend".into()],
            onboarding_done: false,
            launch_at_startup: false,
            notifications: true,
            monitor_interval_ms: 2000,
            monitor_in_background: false,
            scan_threads: 2,
            small_file_threshold_kb: 16,
            small_file_min_count: 2000,
            large_file_min_mb: 256,
            exclusions: Exclusions::default(),
            cleanup_use_quarantine: false,
            temp_min_age_hours: 24,
            user_rules: vec![],
            quarantine_retention_days: 30,
            snapshot_retention_days: 180,
            snapshot_max_count: 60,
            history_retention_days: 365,
            update_check_on_start: true,
            update_check_interval_hours: 24,
            update_include_prerelease: false,
        }
    }
}

fn clean_ext(e: &str) -> String {
    e.trim().trim_start_matches('.').to_lowercase()
}

impl Settings {
    /// Değerleri güvenli aralıklara sıkıştırır; geçersiz metinleri reddeder.
    pub fn validate(mut self) -> AppResult<Self> {
        if !matches!(self.theme.as_str(), "dark" | "light" | "system") {
            return Err(AppError::InvalidInput("tema 'dark', 'light' veya 'system' olmalı".into()));
        }
        if !matches!(self.language.as_str(), "tr" | "en") {
            return Err(AppError::InvalidInput("dil 'tr' veya 'en' olmalı".into()));
        }
        self.monitor_interval_ms = self.monitor_interval_ms.clamp(1000, 30_000);
        self.scan_threads = self.scan_threads.clamp(1, 8);
        self.small_file_threshold_kb = self.small_file_threshold_kb.clamp(1, 1024);
        self.small_file_min_count = self.small_file_min_count.clamp(100, 10_000_000);
        self.large_file_min_mb = self.large_file_min_mb.clamp(1, 1_000_000);
        self.temp_min_age_hours = self.temp_min_age_hours.min(24 * 365);
        self.quarantine_retention_days = self.quarantine_retention_days.clamp(1, 3650);
        self.snapshot_retention_days = self.snapshot_retention_days.clamp(1, 3650);
        self.snapshot_max_count = self.snapshot_max_count.clamp(2, 1000);
        self.history_retention_days = self.history_retention_days.clamp(7, 3650);
        self.update_check_interval_hours = self.update_check_interval_hours.clamp(1, 24 * 30);

        let mut seen = std::collections::HashSet::new();
        self.dashboard_widgets.retain(|w| ALL_WIDGETS.contains(&w.as_str()) && seen.insert(w.clone()));
        let mut seen_wide = std::collections::HashSet::new();
        self.dashboard_wide.retain(|w| ALL_WIDGETS.contains(&w.as_str()) && seen_wide.insert(w.clone()));

        self.exclusions.extensions = self
            .exclusions
            .extensions
            .iter()
            .map(|e| clean_ext(e))
            .filter(|e| !e.is_empty() && e.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'))
            .collect();
        self.exclusions.paths =
            self.exclusions.paths.iter().map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();

        for r in &mut self.user_rules {
            r.path = r.path.trim().to_string();
            r.name = r.name.trim().to_string();
            if r.name.is_empty() {
                return Err(AppError::InvalidInput("temizlik kuralının bir adı olmalı".into()));
            }
            if r.path.is_empty() || !std::path::Path::new(&r.path).is_absolute() {
                return Err(AppError::InvalidInput(format!("'{}' kuralının yolu mutlak bir yol olmalı", r.name)));
            }
            if crate::cleanup::rules::is_protected_resolved(&r.path) {
                return Err(AppError::Protected(format!("'{}' korunan bir sistem konumu", r.path)));
            }
            r.extensions = r.extensions.iter().map(|e| clean_ext(e)).filter(|e| !e.is_empty()).collect();
        }
        Ok(self)
    }
}

impl Settings {
    /// Kayıtlı veya içe aktarılan ayarları, yalnızca geçersiz parçaları
    /// atarak kabul eder. `validate` tek bir hatada tüm nesneyi reddeder; bu
    /// işlev ise ör. artık korunan bir konumu hedefleyen tek bir kuralı atar
    /// ve diğer tercihleri korur. Atılan her parça için bir açıklama döner.
    pub fn sanitize(mut self) -> (Settings, Vec<String>) {
        let mut notes = Vec::new();
        if !matches!(self.theme.as_str(), "dark" | "light" | "system") {
            notes.push(format!("Geçersiz tema (\"{}\") varsayılana döndürüldü.", self.theme));
            self.theme = Settings::default().theme;
        }
        if !matches!(self.language.as_str(), "tr" | "en") {
            notes.push(format!("Geçersiz dil (\"{}\") Türkçeye döndürüldü.", self.language));
            self.language = Settings::default().language;
        }
        let rules = std::mem::take(&mut self.user_rules);
        for r in rules {
            let one = Settings { user_rules: vec![r.clone()], ..Settings::default() };
            match one.validate() {
                Ok(mut v) => self.user_rules.append(&mut v.user_rules),
                Err(e) => notes.push(format!("\"{}\" temizlik kuralı devre dışı bırakıldı: {e}.", r.name.trim())),
            }
        }
        match self.validate() {
            Ok(s) => (s, notes),
            Err(e) => {
                notes.push(format!("Ayarlar doğrulanamadı ({e}); varsayılanlar kullanılıyor."));
                (Settings::default(), notes)
            }
        }
    }
}

const KEY: &str = "settings";
/// Ayrıştırılamayan son ayar metninin saklandığı anahtar (elle kurtarma için).
const CORRUPT_BACKUP_KEY: &str = "settings_corrupt_backup";

pub fn load(db: &Db) -> Settings {
    load_with_notes(db).0
}

/// Ayarları yükler; kullanıcıya bildirilmesi gereken düzeltmeleri de döndürür.
pub fn load_with_notes(db: &Db) -> (Settings, Vec<String>) {
    match db.kv_get(KEY) {
        Ok(Some(json)) => match serde_json::from_str::<Settings>(&json) {
            Ok(mut s) => {
                // 0.1 → 0.2 geçişi: eski pano düzeni yeni bileşenleri içermez.
                if !json.contains("\"dashboardWide\"") {
                    s.dashboard_widgets = DEFAULT_WIDGETS.iter().map(|w| w.to_string()).collect();
                    s.dashboard_wide = Settings::default().dashboard_wide;
                }
                let (s, notes) = s.sanitize();
                for n in &notes {
                    tracing::warn!(note = %n, "kayıtlı ayar düzeltildi");
                }
                (s, notes)
            }
            Err(e) => {
                tracing::warn!(error = %e, "ayarlar bozuk, varsayılanlar kullanılıyor; özgün metin yedeklendi");
                let _ = db.kv_set(CORRUPT_BACKUP_KEY, &json);
                (Settings::default(), vec!["Kayıtlı ayarlar okunamadı; varsayılan ayarlar kullanılıyor.".into()])
            }
        },
        _ => (Settings::default(), vec![]),
    }
}

/// Bir dosyadan içe aktarılacak ayarları ayrıştırır. Hem Ayarlar > Gizlilik
/// bölümündeki tam veri dışa aktarımını (`"ayarlar"` alanı) hem de yalnızca
/// ayar nesnesini kabul eder. İşlem geçmişi, anlık görüntü veya karantina
/// kayıtları **hiçbir zaman** içe aktarılmaz.
pub fn parse_import(text: &str) -> AppResult<(Settings, Vec<String>)> {
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|e| AppError::InvalidInput(format!("dosya geçerli bir JSON değil: {e}")))?;
    let obj = v.as_object().ok_or_else(|| AppError::InvalidInput("dosya bir JSON nesnesi içermiyor".into()))?;
    let settings_value = if let Some(app) = obj.get("uygulama") {
        if app.as_str() != Some("Ivily Optimizer") {
            return Err(AppError::InvalidInput("dosya Ivily Optimizer'a ait değil".into()));
        }
        obj.get("ayarlar").ok_or_else(|| AppError::InvalidInput("dosyada ayar bulunamadı".into()))?.clone()
    } else {
        v.clone()
    };
    if !settings_value.is_object() {
        return Err(AppError::InvalidInput("ayarlar bir JSON nesnesi olmalı".into()));
    }
    let s: Settings = serde_json::from_value(settings_value)
        .map_err(|e| AppError::InvalidInput(format!("ayarlar okunamadı: {e}")))?;
    let (mut s, notes) = s.sanitize();
    // İlk kullanım rehberi içe aktarmayla yeniden açılmasın.
    s.onboarding_done = true;
    Ok((s, notes))
}

pub fn save(db: &Db, s: &Settings) -> AppResult<()> {
    db.kv_set(KEY, &serde_json::to_string(s)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_and_validates() {
        let db = Db::open_in_memory().unwrap();
        let mut s = Settings { monitor_interval_ms: 10, ..Settings::default() };
        s.exclusions.extensions = vec![".LOG".into(), "  ".into(), "a b".into()];
        let s = s.validate().unwrap();
        assert_eq!(s.monitor_interval_ms, 1000);
        assert_eq!(s.exclusions.extensions, vec!["log".to_string()]);
        save(&db, &s).unwrap();
        assert_eq!(load(&db), s);
    }

    #[test]
    fn rejects_bad_theme() {
        let s = Settings { theme: "pink".into(), ..Default::default() };
        assert!(s.validate().is_err());
    }

    #[test]
    fn rejects_rule_on_protected_path() {
        let s = Settings {
            user_rules: vec![UserCleanupRule {
                id: "r".into(),
                name: "kötü".into(),
                path: "C:\\Windows\\System32".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn corrupt_settings_fall_back() {
        let db = Db::open_in_memory().unwrap();
        db.kv_set(KEY, "{not json").unwrap();
        let (s, notes) = load_with_notes(&db);
        assert_eq!(s, Settings::default());
        assert_eq!(notes.len(), 1);
        // Özgün metin elle kurtarma için saklanır.
        assert_eq!(db.kv_get(CORRUPT_BACKUP_KEY).unwrap().as_deref(), Some("{not json"));
    }

    fn rule(name: &str, path: &str) -> UserCleanupRule {
        UserCleanupRule { id: name.into(), name: name.into(), path: path.into(), ..Default::default() }
    }

    #[test]
    fn one_invalid_rule_does_not_reset_other_preferences() {
        let db = Db::open_in_memory().unwrap();
        let good = tempfile::tempdir().unwrap();
        // Doğrulamadan geçmeyen bir kural doğrudan veritabanına yazılmış olsun
        // (ör. sonraki bir sürümde korunan konum listesi genişlemiş).
        let stored = Settings {
            theme: "light".into(),
            monitor_interval_ms: 5000,
            user_rules: vec![rule("iyi", &good.path().to_string_lossy()), rule("kötü", r"C:\Windows\System32")],
            ..Default::default()
        };
        db.kv_set(KEY, &serde_json::to_string(&stored).unwrap()).unwrap();
        let (s, notes) = load_with_notes(&db);
        assert_eq!(s.theme, "light");
        assert_eq!(s.monitor_interval_ms, 5000);
        assert_eq!(s.user_rules.len(), 1);
        assert_eq!(s.user_rules[0].name, "iyi");
        assert_eq!(notes.len(), 1);
        assert!(notes[0].contains("kötü"), "{notes:?}");
    }

    #[test]
    fn language_defaults_to_turkish_and_is_validated() {
        assert_eq!(Settings::default().language, "tr");
        assert!(Settings { language: "en".into(), ..Default::default() }.validate().is_ok());
        assert!(Settings { language: "de".into(), ..Default::default() }.validate().is_err());
        let (s, notes) = Settings { language: "de".into(), theme: "light".into(), ..Default::default() }.sanitize();
        assert_eq!((s.language.as_str(), s.theme.as_str(), notes.len()), ("tr", "light", 1));
        // 0.2.0'da kaydedilmiş ayarlarda alan yoktur: varsayılan kullanılır.
        let db = Db::open_in_memory().unwrap();
        db.kv_set(KEY, r#"{"theme":"light","dashboardWide":[]}"#).unwrap();
        assert_eq!(load(&db).language, "tr");
    }

    #[test]
    fn invalid_theme_is_repaired_not_reset() {
        let (s, notes) = Settings { theme: "pembe".into(), scan_threads: 4, ..Default::default() }.sanitize();
        assert_eq!(s.theme, "dark");
        assert_eq!(s.scan_threads, 4);
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn import_accepts_full_export_and_bare_settings() {
        let export = serde_json::json!({
            "uygulama": "Ivily Optimizer",
            "surum": "0.2.0",
            "ayarlar": { "theme": "light", "scanThreads": 3, "onboardingDone": false },
            "islemGecmisi": [{ "id": 1 }],
        });
        let (s, notes) = parse_import(&export.to_string()).unwrap();
        assert_eq!((s.theme.as_str(), s.scan_threads, s.onboarding_done), ("light", 3, true));
        assert!(notes.is_empty());
        let (s, _) = parse_import(r#"{"theme":"system","monitorIntervalMs":10}"#).unwrap();
        assert_eq!(s.theme, "system");
        assert_eq!(s.monitor_interval_ms, 1000, "değerler sınırlara sıkıştırılır");
    }

    #[test]
    fn import_rejects_foreign_or_malformed_files() {
        for bad in [
            "",
            "{yarım",
            "[1,2]",
            "42",
            r#"{"uygulama":"Başka Uygulama","ayarlar":{}}"#,
            r#"{"uygulama":"Ivily Optimizer"}"#,
            r#"{"uygulama":"Ivily Optimizer","ayarlar":"metin"}"#,
            r#"{"theme": 5}"#,
        ] {
            assert!(parse_import(bad).is_err(), "kabul edilmemeliydi: {bad}");
        }
    }

    #[test]
    fn missing_fields_use_defaults() {
        let db = Db::open_in_memory().unwrap();
        db.kv_set(KEY, r#"{"theme":"light"}"#).unwrap();
        let s = load(&db);
        assert_eq!(s.theme, "light");
        assert_eq!(s.scan_threads, 2);
    }

    #[test]
    fn dedups_widgets() {
        let s = Settings { dashboard_widgets: vec!["cpu".into(), "cpu".into(), "bogus".into()], ..Default::default() }
            .validate()
            .unwrap();
        assert_eq!(s.dashboard_widgets, vec!["cpu".to_string()]);
    }
}
