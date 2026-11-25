//! Arka plan işleri: kimlik, iptal ve ön yüze ilerleme olayları.
//!
//! Uzun süren her işlem (tarama, temizlik, yinelenen dosya analizi) ayrı bir
//! iş parçacığında çalışır; arayüz iş parçacığı asla bloke edilmez.

use crate::error::{AppError, AppResult};
use crate::scan::walker::WalkProgress;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

#[derive(Clone)]
pub struct JobCtl {
    pub id: String,
    pub kind: &'static str,
    pub cancel: Arc<AtomicBool>,
    pub progress: Arc<WalkProgress>,
    /// Aşama tabanlı ilerleme (ör. "312 / 1200 dosya doğrulandı").
    pub phase: Arc<Mutex<String>>,
    pub done: Arc<AtomicU64>,
    pub total: Arc<AtomicU64>,
}

impl JobCtl {
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
    pub fn set_phase(&self, p: &str, total: u64) {
        *self.phase.lock() = p.to_string();
        self.total.store(total, Ordering::Relaxed);
        self.done.store(0, Ordering::Relaxed);
    }
    pub fn add_done(&self, n: u64) {
        self.done.fetch_add(n, Ordering::Relaxed);
    }
}

#[derive(Default)]
pub struct Jobs {
    inner: Mutex<HashMap<String, JobCtl>>,
    seq: AtomicU64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgressEvent {
    pub id: String,
    pub kind: String,
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub errors: u64,
    pub current: String,
    pub phase: String,
    pub done: u64,
    pub total: u64,
    /// Yalnızca güvenilir bir tahmin varsa (ör. birim kökü taranırken
    /// kullanılan alan) doldurulur.
    pub estimated_bytes: Option<u64>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDoneEvent {
    pub id: String,
    pub kind: String,
    /// "complete" | "cancelled" | "failed"
    pub status: String,
    pub message: Option<String>,
    pub summary: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobInfo {
    pub id: String,
    pub kind: String,
}

impl Jobs {
    /// Aynı türde çalışan bir iş varsa `Busy` döner.
    pub fn start(&self, kind: &'static str) -> AppResult<JobCtl> {
        let mut m = self.inner.lock();
        if m.values().any(|j| j.kind == kind) {
            return Err(AppError::Busy(kind_label(kind).into()));
        }
        let id = format!("{kind}-{}", self.seq.fetch_add(1, Ordering::Relaxed) + 1);
        let ctl = JobCtl {
            id: id.clone(),
            kind,
            cancel: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(WalkProgress::default()),
            phase: Arc::new(Mutex::new(String::new())),
            done: Arc::new(AtomicU64::new(0)),
            total: Arc::new(AtomicU64::new(0)),
        };
        m.insert(id, ctl.clone());
        Ok(ctl)
    }

    pub fn finish(&self, id: &str) {
        self.inner.lock().remove(id);
    }

    pub fn cancel(&self, id: &str) -> bool {
        match self.inner.lock().get(id) {
            Some(j) => {
                j.cancel.store(true, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }

    pub fn running(&self) -> Vec<JobInfo> {
        self.inner.lock().values().map(|j| JobInfo { id: j.id.clone(), kind: j.kind.to_string() }).collect()
    }
}

pub fn kind_label(kind: &str) -> &'static str {
    match kind {
        "disk-scan" => "disk taraması",
        "cleanup-scan" => "temizlik taraması",
        "cleanup-run" => "temizlik",
        "dupes" => "yinelenen dosya analizi",
        "cold" => "eski dosya analizi",
        "quarantine" => "karantina işlemi",
        "update" => "güncelleme",
        _ => "işlem",
    }
}

/// İş bitene kadar düzenli aralıklarla ilerleme olayı yayınlayan bir
/// iş parçacığı başlatır. Dönen bayrak `true` yapıldığında durur.
pub fn spawn_reporter(app: AppHandle, ctl: JobCtl, estimated_bytes: Option<u64>) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let t0 = std::time::Instant::now();
    std::thread::spawn(move || {
        while !stop2.load(Ordering::Relaxed) {
            let p = &ctl.progress;
            let ev = JobProgressEvent {
                id: ctl.id.clone(),
                kind: ctl.kind.to_string(),
                files: p.files.load(Ordering::Relaxed),
                dirs: p.dirs.load(Ordering::Relaxed),
                bytes: p.bytes.load(Ordering::Relaxed),
                errors: p.errors.load(Ordering::Relaxed),
                current: p.current.lock().clone(),
                phase: ctl.phase.lock().clone(),
                done: ctl.done.load(Ordering::Relaxed),
                total: ctl.total.load(Ordering::Relaxed),
                estimated_bytes,
                elapsed_ms: t0.elapsed().as_millis() as u64,
            };
            let _ = app.emit("job-progress", ev);
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    });
    stop
}

pub fn emit_done(app: &AppHandle, ctl: &JobCtl, status: &str, message: Option<String>, summary: serde_json::Value) {
    let _ = app.emit(
        "job-done",
        JobDoneEvent {
            id: ctl.id.clone(),
            kind: ctl.kind.to_string(),
            status: status.to_string(),
            message,
            summary,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prevents_duplicate_kinds() {
        let j = Jobs::default();
        let a = j.start("disk-scan").unwrap();
        assert!(j.start("disk-scan").is_err());
        assert!(j.start("dupes").is_ok());
        assert!(j.cancel(&a.id));
        assert!(a.cancelled());
        j.finish(&a.id);
        assert!(j.start("disk-scan").is_ok());
    }
}
