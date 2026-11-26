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
            if crate::cleanup::rules::is_protected_path(&r.path) {
                return Err(AppError::Protected(format!("'{}' korunan bir sistem konumu", r.path)));
            }
            r.extensions = r.extensions.iter().map(|e| clean_ext(e)).filter(|e| !e.is_empty()).collect();
        }
        Ok(self)
    }
}

const KEY: &str = "settings";

pub fn load(db: &Db) -> Settings {
    match db.kv_get(KEY) {
        Ok(Some(json)) => match serde_json::from_str::<Settings>(&json) {
            Ok(mut s) => {
                // 0.1 → 0.2 geçişi: eski pano düzeni yeni bileşenleri içermez.
                if !json.contains("\"dashboardWide\"") {
                    s.dashboard_widgets = DEFAULT_WIDGETS.iter().map(|w| w.to_string()).collect();
                    s.dashboard_wide = Settings::default().dashboard_wide;
                }
                s.validate().unwrap_or_default()
            }
            Err(e) => {
                tracing::warn!(error = %e, "ayarlar bozuk, varsayılanlar kullanılıyor");
                Settings::default()
            }
        },
        _ => Settings::default(),
    }
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
        assert_eq!(load(&db), Settings::default());
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
