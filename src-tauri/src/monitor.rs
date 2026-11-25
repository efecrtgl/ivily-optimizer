//! Sistem bilgisi ve canlı kaynak izleme.
//!
//! Arka planda sürekli çalışan bir döngü yoktur: ön yüz, kullanıcının
//! ayarladığı aralıkla `live_stats` çağırır ve pencere gizliyken (ayar
//! kapalıysa) çağırmayı bırakır. Böylece uygulama boştayken CPU tüketmez.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::Instant;
use sysinfo::{
    CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, Networks, Pid, ProcessRefreshKind,
    ProcessesToUpdate, RefreshKind, System, UpdateKind,
};

pub const HISTORY_LEN: usize = 150;

pub struct Monitor {
    sys: System,
    disks: Disks,
    networks: Networks,
    last_net: Option<Instant>,
    last_disk: Option<Instant>,
    history: VecDeque<Sample>,
    procs_refreshed: Option<Instant>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub ts: i64,
    pub cpu: f32,
    pub mem_used: u64,
    pub net_rx: u64,
    pub net_tx: u64,
    pub disk_read: u64,
    pub disk_write: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub name: String,
    pub mount: String,
    pub file_system: String,
    pub kind: String,
    pub total: u64,
    pub available: u64,
    pub used: u64,
    pub removable: bool,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub total_memory: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub host_name: String,
    pub os_name: String,
    pub os_version: String,
    pub kernel: String,
    pub arch: String,
    pub cpu_brand: String,
    pub cpu_logical: usize,
    pub cpu_physical: Option<usize>,
    pub cpu_freq_mhz: u64,
    pub total_memory: u64,
    pub total_swap: u64,
    pub boot_time: u64,
    pub gpus: Vec<GpuInfo>,
    pub disks: Vec<DiskInfo>,
    pub app_version: String,
    pub is_elevated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStats {
    pub ts: i64,
    pub cpu: f32,
    pub per_core: Vec<f32>,
    pub mem_total: u64,
    pub mem_used: u64,
    pub mem_available: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub uptime: u64,
    pub net_rx_per_sec: u64,
    pub net_tx_per_sec: u64,
    pub net_connected: bool,
    pub disk_read_per_sec: u64,
    pub disk_write_per_sec: u64,
    pub process_count: usize,
    pub history: Vec<Sample>,
}

impl Monitor {
    pub fn new() -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_usage().with_frequency())
                .with_memory(MemoryRefreshKind::everything()),
        )
        .unwrap_or_else(|_| System::new().expect("sistem bilgisi başlatılamadı"));
        Self {
            sys,
            disks: Disks::new_with_refreshed_list().unwrap_or_else(|_| Disks::new().expect("disk listesi")),
            networks: Networks::new_with_refreshed_list().unwrap_or_else(|_| Networks::new().expect("ağ listesi")),
            last_net: None,
            last_disk: None,
            history: VecDeque::with_capacity(HISTORY_LEN),
            procs_refreshed: None,
        }
    }

    pub fn disks(&mut self) -> Vec<DiskInfo> {
        self.disks.refresh_specifics(true, DiskRefreshKind::everything());
        let mut v: Vec<DiskInfo> = self
            .disks
            .iter()
            .map(|d| DiskInfo {
                name: d.name().to_string_lossy().into_owned(),
                mount: d.mount_point().to_string_lossy().into_owned(),
                file_system: d.file_system().to_string_lossy().into_owned(),
                kind: match d.kind() {
                    sysinfo::DiskKind::SSD => "SSD".into(),
                    sysinfo::DiskKind::HDD => "HDD".into(),
                    _ => "Bilinmiyor".into(),
                },
                total: d.total_space(),
                available: d.available_space(),
                used: d.total_space().saturating_sub(d.available_space()),
                removable: d.is_removable(),
                read_only: d.is_read_only(),
            })
            .collect();
        v.sort_by(|a, b| a.mount.cmp(&b.mount));
        v.dedup_by(|a, b| a.mount == b.mount);
        v
    }

    pub fn system_info(&mut self) -> SystemInfo {
        self.sys.refresh_cpu_all();
        let cpus = self.sys.cpus();
        let gpus = sysinfo::Gpus::new_with_refreshed_list()
            .map(|g| {
                g.iter()
                    .filter(|g| !g.model().unwrap_or("").contains("Basic Render"))
                    .map(|g| GpuInfo {
                        vendor: g.vendor().map(str::to_string),
                        model: g.model().map(str::to_string),
                        total_memory: g.total_memory(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        SystemInfo {
            host_name: System::host_name().unwrap_or_default(),
            os_name: System::long_os_version().or_else(|_| System::name()).unwrap_or_else(|_| "Windows".into()),
            os_version: System::os_version().unwrap_or_default(),
            kernel: System::kernel_version().unwrap_or_default(),
            arch: System::cpu_arch(),
            cpu_brand: cpus.first().map(|c| c.brand().trim().to_string()).unwrap_or_default(),
            cpu_logical: cpus.len(),
            cpu_physical: System::physical_core_count().ok(),
            cpu_freq_mhz: cpus.first().map(|c| c.frequency()).unwrap_or(0),
            total_memory: self.sys.total_memory(),
            total_swap: self.sys.total_swap(),
            boot_time: System::boot_time().unwrap_or(0),
            gpus,
            disks: self.disks(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            is_elevated: crate::platform::is_elevated(),
        }
    }

    pub fn live(&mut self) -> LiveStats {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        let now = Instant::now();

        self.networks.refresh(true);
        let net_secs = self.last_net.map(|t| now.duration_since(t).as_secs_f64()).unwrap_or(0.0);
        self.last_net = Some(now);
        let (mut rx, mut tx) = (0u64, 0u64);
        let mut connected = false;
        for (name, n) in self.networks.iter() {
            let lname = name.to_string_lossy().to_lowercase();
            if lname.contains("loopback") {
                continue;
            }
            rx += n.received();
            tx += n.transmitted();
            if matches!(n.operational_state(), sysinfo::InterfaceOperationalState::Up)
                && n.ip_networks().iter().any(|ip| !ip.addr.is_loopback() && !ip.addr.is_unspecified())
            {
                connected = true;
            }
        }
        let per_sec = |v: u64, s: f64| if s > 0.05 { (v as f64 / s) as u64 } else { 0 };

        self.disks.refresh_specifics(false, DiskRefreshKind::nothing().with_io_usage());
        let disk_secs = self.last_disk.map(|t| now.duration_since(t).as_secs_f64()).unwrap_or(0.0);
        self.last_disk = Some(now);
        let (mut dr, mut dw) = (0u64, 0u64);
        for d in self.disks.iter() {
            let u = d.usage();
            dr += u.read_bytes;
            dw += u.written_bytes;
        }

        let sample = Sample {
            ts: chrono::Utc::now().timestamp_millis(),
            cpu: self.sys.global_cpu_usage(),
            mem_used: self.sys.used_memory(),
            net_rx: per_sec(rx, net_secs),
            net_tx: per_sec(tx, net_secs),
            disk_read: per_sec(dr, disk_secs),
            disk_write: per_sec(dw, disk_secs),
        };
        if self.history.len() >= HISTORY_LEN {
            self.history.pop_front();
        }
        self.history.push_back(sample.clone());

        LiveStats {
            ts: sample.ts,
            cpu: sample.cpu,
            per_core: self.sys.cpus().iter().map(|c| c.usage()).collect(),
            mem_total: self.sys.total_memory(),
            mem_used: self.sys.used_memory(),
            mem_available: self.sys.available_memory(),
            swap_total: self.sys.total_swap(),
            swap_used: self.sys.used_swap(),
            uptime: System::uptime().unwrap_or(0),
            net_rx_per_sec: sample.net_rx,
            net_tx_per_sec: sample.net_tx,
            net_connected: connected,
            disk_read_per_sec: sample.disk_read,
            disk_write_per_sec: sample.disk_write,
            process_count: self.sys.processes().len(),
            history: self.history.iter().cloned().collect(),
        }
    }

    // -----------------------------------------------------------------------
    // Süreçler
    // -----------------------------------------------------------------------

    pub fn processes(&mut self) -> Vec<ProcessInfo> {
        let now = Instant::now();
        let secs = self.procs_refreshed.map(|t| now.duration_since(t).as_secs_f64()).unwrap_or(0.0);
        self.procs_refreshed = Some(now);
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_disk_usage()
                .with_exe(UpdateKind::OnlyIfNotSet)
                .with_user(UpdateKind::OnlyIfNotSet),
        );
        let ncpu = self.sys.cpus().len().max(1) as f32;
        let self_pid = std::process::id();
        let conns = crate::sysmgmt::procinfo::connection_counts();
        let ctx = crate::sysmgmt::indicators::PathContext::current();
        self.sys
            .processes()
            .values()
            .map(|p| {
                let name = p.name().to_string_lossy().into_owned();
                let exe = p.exe().map(|e| e.to_string_lossy().into_owned());
                let du = p.disk_usage();
                let protection = protection_of(&name, exe.as_deref(), p.pid().as_u32(), self_pid);
                let indicators = crate::sysmgmt::indicators::process_indicators(&ctx, &name, exe.as_deref(), |e| {
                    std::path::Path::new(e).exists()
                });
                ProcessInfo {
                    pid: p.pid().as_u32(),
                    parent: p.parent().map(|x| x.as_u32()),
                    name,
                    exe,
                    // sysinfo, CPU kullanımını çekirdek başına verir; Görev
                    // Yöneticisi ile tutarlı olması için toplam CPU'ya
                    // oranlıyoruz.
                    cpu: p.cpu_usage() / ncpu,
                    memory: p.memory(),
                    io_read_per_sec: if secs > 0.05 { (du.read_bytes as f64 / secs) as u64 } else { 0 },
                    io_write_per_sec: if secs > 0.05 { (du.written_bytes as f64 / secs) as u64 } else { 0 },
                    start_time: p.start_time(),
                    status: format!("{:?}", p.status()),
                    protection: protection.0.to_string(),
                    protection_reason: protection.1.map(str::to_string),
                    is_self: p.pid().as_u32() == self_pid,
                    connections: conns.get(&p.pid().as_u32()).copied().unwrap_or(0),
                    indicators,
                }
            })
            .collect()
    }

    pub fn kill(&mut self, pid: u32, expected_name: &str, force_sensitive: bool) -> AppResult<()> {
        self.sys.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), true);
        let p = self
            .sys
            .process(Pid::from_u32(pid))
            .ok_or_else(|| AppError::NotFound(format!("PID {pid} artık çalışmıyor")))?;
        let name = p.name().to_string_lossy().into_owned();
        // PID yeniden kullanılmış olabilir: adı doğrula.
        if !name.eq_ignore_ascii_case(expected_name) {
            return Err(AppError::Conflict(format!(
                "PID {pid} artık başka bir sürece ({name}) ait; liste yenilendi"
            )));
        }
        let exe = p.exe().map(|e| e.to_string_lossy().into_owned());
        match protection_of(&name, exe.as_deref(), pid, std::process::id()) {
            ("critical", reason) => {
                return Err(AppError::Protected(reason.unwrap_or("kritik sistem süreci").to_string()))
            }
            ("sensitive", _) if !force_sensitive => {
                return Err(AppError::Protected("bu süreç için ek onay gerekiyor".into()))
            }
            _ => {}
        }
        if p.kill() {
            Ok(())
        } else {
            Err(AppError::PermissionDenied(format!(
                "{name} sonlandırılamadı. Süreç yönetici hakları gerektiriyor veya korumalı olabilir."
            )))
        }
    }
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: u32,
    pub parent: Option<u32>,
    pub name: String,
    pub exe: Option<String>,
    pub cpu: f32,
    pub memory: u64,
    pub io_read_per_sec: u64,
    pub io_write_per_sec: u64,
    pub start_time: u64,
    pub status: String,
    /// "critical" | "sensitive" | "normal"
    pub protection: String,
    pub protection_reason: Option<String>,
    pub is_self: bool,
    /// Bu sürece ait TCP + UDP uç noktası sayısı.
    pub connections: u32,
    /// Ucuz, yol tabanlı "dikkat gerektirebilir" göstergeleri.
    pub indicators: Vec<crate::sysmgmt::indicators::Indicator>,
}

/// Sonlandırılması sistemi çökertebilecek veya oturumu kapatabilecek süreçler.
const CRITICAL: &[&str] = &[
    "system", "registry", "smss.exe", "csrss.exe", "wininit.exe", "winlogon.exe", "services.exe", "lsass.exe",
    "lsaiso.exe", "memory compression", "secure system", "system idle process", "fontdrvhost.exe", "svchost.exe",
    "dwm.exe", "sihost.exe", "ctfmon.exe", "msmpeng.exe", "mpdefendercoreservice.exe", "nissrv.exe",
    "securityhealthservice.exe", "wudfhost.exe", "spoolsv.exe", "audiodg.exe",
];

/// Sonlandırılabilir ama beklenmedik sonuçları olabilecek süreçler.
const SENSITIVE: &[&str] = &[
    "explorer.exe", "searchhost.exe", "searchindexer.exe", "startmenuexperiencehost.exe", "shellexperiencehost.exe",
    "runtimebroker.exe", "taskhostw.exe", "conhost.exe", "dllhost.exe", "textinputhost.exe", "wmiprvse.exe",
    "msiexec.exe", "trustedinstaller.exe", "tiworker.exe", "securityhealthsystray.exe", "smartscreen.exe",
];

pub fn protection_of(name: &str, exe: Option<&str>, pid: u32, self_pid: u32) -> (&'static str, Option<&'static str>) {
    let lname = name.to_lowercase();
    if pid <= 4 {
        return ("critical", Some("Windows çekirdek süreci"));
    }
    if pid == self_pid {
        return ("sensitive", Some("Ivily Optimizer'ın kendisi"));
    }
    if CRITICAL.contains(&lname.as_str()) {
        return ("critical", Some("Windows'un çalışması için gerekli kritik bir sistem süreci"));
    }
    if SENSITIVE.contains(&lname.as_str()) {
        return ("sensitive", Some("Windows kabuğu veya sistem bileşeni; sonlandırmak masaüstünü ya da bazı özellikleri geçici olarak bozabilir"));
    }
    if let Some(e) = exe {
        let windir = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        if crate::paths::is_within(e, &windir) {
            return ("sensitive", Some("Windows klasöründen çalışan bir sistem bileşeni"));
        }
    }
    ("normal", None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protection_rules() {
        assert_eq!(protection_of("csrss.exe", None, 700, 1).0, "critical");
        assert_eq!(protection_of("LSASS.EXE", None, 700, 1).0, "critical");
        assert_eq!(protection_of("System", None, 4, 1).0, "critical");
        assert_eq!(protection_of("explorer.exe", None, 900, 1).0, "sensitive");
        assert_eq!(protection_of("x.exe", Some("C:\\Windows\\System32\\x.exe"), 900, 1).0, "sensitive");
        assert_eq!(protection_of("notepad++.exe", Some("C:\\Program Files\\N\\n.exe"), 900, 1).0, "normal");
        assert_eq!(protection_of("ivily.exe", None, 77, 77).0, "sensitive");
    }

    #[test]
    fn live_stats_work() {
        let mut m = Monitor::new();
        let a = m.live();
        assert!(a.mem_total > 0);
        let b = m.live();
        assert_eq!(b.history.len(), 2);
        assert!(!m.disks().is_empty());
    }
}
