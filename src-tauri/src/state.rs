//! Uygulama genelindeki paylaşılan durum.

use crate::cleanup::CleanupScan;
use crate::db::Db;
use crate::jobs::Jobs;
use crate::monitor::Monitor;
use crate::scan::dupes::DupReport;
use crate::scan::tree::DiskScan;
use crate::scan::ColdReport;
use crate::settings::Settings;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;

pub struct AppState {
    pub db: Arc<Db>,
    pub settings: RwLock<Settings>,
    pub jobs: Jobs,
    pub monitor: Mutex<Monitor>,
    pub disk_scan: RwLock<Option<Arc<DiskScan>>>,
    pub cleanup_scan: Mutex<Option<Arc<CleanupScan>>>,
    pub dupes: RwLock<Option<Arc<DupReport>>>,
    pub cold: RwLock<Option<Arc<ColdReport>>>,
}

impl AppState {
    pub fn new(db: Db) -> Self {
        let settings = crate::settings::load(&db);
        Self {
            db: Arc::new(db),
            settings: RwLock::new(settings),
            jobs: Jobs::default(),
            monitor: Mutex::new(Monitor::new()),
            disk_scan: RwLock::new(None),
            cleanup_scan: Mutex::new(None),
            dupes: RwLock::new(None),
            cold: RwLock::new(None),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().clone()
    }

    pub fn quarantine(&self) -> crate::quarantine::Quarantine<'_> {
        crate::quarantine::Quarantine::new(&self.db, crate::paths::quarantine_dir())
    }

    pub fn walk_options(&self) -> crate::scan::walker::WalkOptions {
        let s = self.settings.read();
        crate::scan::walker::WalkOptions::new(s.scan_threads).with_exclusions(&s.exclusions)
    }
}
