//! Süreç ayrıntıları: ağ bağlantıları, üst/alt süreçler, imza ve güvenli
//! yeniden başlatma.

use super::indicators::{process_indicators, Indicator, PathContext};
use super::signature::{self, SignatureStatus};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::oplog::{self, Outcome};
use serde::Serialize;
use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::Path;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

// ---------------------------------------------------------------------------
// Ağ bağlantıları
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    /// "TCP" | "UDP"
    pub proto: String,
    pub local_address: String,
    pub local_port: u16,
    pub remote_address: Option<String>,
    pub remote_port: Option<u16>,
    pub state: Option<String>,
    #[serde(skip)]
    pub pid: u32,
}

/// MIB_TCP_STATE değerinin Türkçe karşılığı.
pub fn tcp_state_label(state: u32) -> &'static str {
    match state {
        1 => "Kapalı",
        2 => "Dinliyor",
        3 | 4 => "Bağlantı kuruluyor",
        5 => "Bağlı",
        6 | 7 | 9 | 10 => "Kapanıyor",
        8 => "Karşı taraf kapattı",
        11 => "Kapanış bekleniyor",
        12 => "Siliniyor",
        _ => "Bilinmiyor",
    }
}

fn u32_at(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// DWORD içindeki ağ bayt sıralı port.
fn port_at(b: &[u8], off: usize) -> u16 {
    u16::from_be_bytes([b[off], b[off + 1]])
}

fn v4_at(b: &[u8], off: usize) -> String {
    Ipv4Addr::new(b[off], b[off + 1], b[off + 2], b[off + 3]).to_string()
}

fn v6_at(b: &[u8], off: usize) -> String {
    let mut a = [0u8; 16];
    a.copy_from_slice(&b[off..off + 16]);
    Ipv6Addr::from(a).to_string()
}

/// Ham tablo arabelleklerini ayrıştırır. `kind`: 0=TCP4, 1=TCP6, 2=UDP4, 3=UDP6.
pub fn parse_table(kind: u8, buf: &[u8]) -> Vec<Connection> {
    if buf.len() < 4 {
        return Vec::new();
    }
    let n = u32_at(buf, 0) as usize;
    let row = match kind {
        0 => 24,
        1 => 56,
        2 => 12,
        _ => 28,
    };
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let o = 4 + i * row;
        if o + row > buf.len() {
            break;
        }
        let c = match kind {
            0 => {
                let st = u32_at(buf, o);
                let listen = st == 2;
                Connection {
                    proto: "TCP".into(),
                    local_address: v4_at(buf, o + 4),
                    local_port: port_at(buf, o + 8),
                    remote_address: (!listen).then(|| v4_at(buf, o + 12)),
                    remote_port: (!listen).then(|| port_at(buf, o + 16)),
                    state: Some(tcp_state_label(st).into()),
                    pid: u32_at(buf, o + 20),
                }
            }
            1 => {
                let st = u32_at(buf, o + 48);
                let listen = st == 2;
                Connection {
                    proto: "TCP".into(),
                    local_address: v6_at(buf, o),
                    local_port: port_at(buf, o + 20),
                    remote_address: (!listen).then(|| v6_at(buf, o + 24)),
                    remote_port: (!listen).then(|| port_at(buf, o + 44)),
                    state: Some(tcp_state_label(st).into()),
                    pid: u32_at(buf, o + 52),
                }
            }
            2 => Connection {
                proto: "UDP".into(),
                local_address: v4_at(buf, o),
                local_port: port_at(buf, o + 4),
                remote_address: None,
                remote_port: None,
                state: None,
                pid: u32_at(buf, o + 8),
            },
            _ => Connection {
                proto: "UDP".into(),
                local_address: v6_at(buf, o),
                local_port: port_at(buf, o + 20),
                remote_address: None,
                remote_port: None,
                state: None,
                pid: u32_at(buf, o + 24),
            },
        };
        out.push(c);
    }
    out
}

#[cfg(windows)]
fn fetch_table(kind: u8) -> Vec<u8> {
    use windows::Win32::NetworkManagement::IpHelper::*;
    const AF_INET: u32 = 2;
    const AF_INET6: u32 = 23;
    let af = if kind == 0 || kind == 2 { AF_INET } else { AF_INET6 };
    let mut size = 0u32;
    for _ in 0..4 {
        let mut buf = vec![0u64; (size as usize).div_ceil(8).max(1)];
        let cap = (buf.len() * 8) as u32;
        size = cap;
        let r = unsafe {
            if kind < 2 {
                GetExtendedTcpTable(Some(buf.as_mut_ptr() as *mut _), &mut size, false, af, TCP_TABLE_OWNER_PID_ALL, 0)
            } else {
                GetExtendedUdpTable(Some(buf.as_mut_ptr() as *mut _), &mut size, false, af, UDP_TABLE_OWNER_PID, 0)
            }
        };
        if r == 0 {
            let bytes = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, cap as usize) };
            return bytes.to_vec();
        }
        if r != 122 {
            // ERROR_INSUFFICIENT_BUFFER dışında bir hata.
            return Vec::new();
        }
        size += 4096;
    }
    Vec::new()
}
#[cfg(not(windows))]
fn fetch_table(_kind: u8) -> Vec<u8> {
    Vec::new()
}

/// Tüm TCP (IPv4+IPv6) ve UDP uç noktalarının anlık görüntüsü.
pub fn connections_snapshot() -> Vec<Connection> {
    (0u8..4).flat_map(|k| parse_table(k, &fetch_table(k))).collect()
}

/// PID başına uç nokta sayısı.
pub fn connection_counts() -> HashMap<u32, u32> {
    let mut m = HashMap::new();
    for c in connections_snapshot() {
        *m.entry(c.pid).or_insert(0) += 1;
    }
    m
}

// ---------------------------------------------------------------------------
// Süreç ayrıntıları
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcRef {
    pub pid: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDetails {
    pub pid: u32,
    pub name: String,
    pub exe: Option<String>,
    pub cmd: Vec<String>,
    pub cwd: Option<String>,
    pub start_time: u64,
    pub parent_chain: Vec<ProcRef>,
    pub children: Vec<ProcRef>,
    pub signature: Option<SignatureStatus>,
    pub connections: Vec<Connection>,
    pub indicators: Vec<Indicator>,
    pub protection: String,
    pub protection_reason: Option<String>,
    pub can_restart: bool,
    pub restart_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessRestartResult {
    pub old_pid: u32,
    pub new_pid: u32,
    pub message: String,
}

/// Yeniden başlatılması desteklenmeyen tek örnekli kabuk süreçleri.
const NO_RESTART: &[&str] = &[
    "explorer.exe",
    "sihost.exe",
    "startmenuexperiencehost.exe",
    "shellexperiencehost.exe",
    "searchhost.exe",
    "textinputhost.exe",
];

fn new_system() -> AppResult<System> {
    System::new().map_err(|e| AppError::System(format!("süreç bilgisi alınamadı ({e:?})")))
}

fn refresh_all(sys: &mut System) {
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet).with_user(UpdateKind::OnlyIfNotSet),
    );
}

fn refresh_one(sys: &mut System, pid: u32) {
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
        true,
        ProcessRefreshKind::nothing()
            .with_exe(UpdateKind::Always)
            .with_user(UpdateKind::Always)
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always),
    );
}

/// Yeniden başlatma uygunluğu: `Ok(())` veya Türkçe gerekçe.
fn restart_check(sys: &System, pid: u32) -> Result<(), String> {
    let self_pid = std::process::id();
    let p = sys.process(Pid::from_u32(pid)).ok_or("Süreç artık çalışmıyor.")?;
    let name = p.name().to_string_lossy().into_owned();
    let exe = p.exe().map(|e| e.to_string_lossy().into_owned());
    if pid == self_pid {
        return Err("Ivily Optimizer kendini bu yolla yeniden başlatamaz.".into());
    }
    if NO_RESTART.contains(&name.to_lowercase().as_str()) {
        return Err("Windows kabuğunun bir parçası olduğu için yeniden başlatma desteklenmiyor.".into());
    }
    let (prot, _) = crate::monitor::protection_of(&name, exe.as_deref(), pid, self_pid);
    if prot != "normal" {
        return Err("Sistem bileşenleri ve korunan süreçler yeniden başlatılamaz.".into());
    }
    let Some(exe) = exe.filter(|e| !e.is_empty()) else {
        return Err("Programın dosya yolu okunamadı (süreç yönetici hakları gerektiriyor olabilir).".into());
    };
    if !Path::new(&exe).is_file() {
        return Err("Programın dosyası artık diskte bulunamıyor.".into());
    }
    let me = sysinfo::get_current_pid().ok().and_then(|sp| sys.process(sp)).and_then(|s| s.user_id().cloned());
    match (me, p.user_id()) {
        (Some(a), Some(b)) if &a == b => {}
        (Some(_), Some(_)) => return Err("Süreç başka bir kullanıcı hesabına ait.".into()),
        _ => return Err("Sürecin sahibi belirlenemedi; güvenlik nedeniyle yeniden başlatılmıyor.".into()),
    }
    if p.cmd().is_empty() {
        return Err("Sürecin komut satırı okunamadı.".into());
    }
    // Komut satırı hedef sürecin kendi belleğinden okunur ve o süreç
    // tarafından değiştirilebilir. Ivily yönetici olarak çalışıyorsa bu komut
    // satırı yükseltilmiş haklarla yeniden çalıştırılmış olurdu.
    if crate::platform::is_elevated() {
        return Err(
            "Ivily Optimizer yönetici olarak çalışırken süreçler yeniden başlatılamaz; uygulamayı normal kullanıcı olarak açın."
                .into(),
        );
    }
    Ok(())
}

pub fn details(pid: u32) -> AppResult<ProcessDetails> {
    let mut sys = new_system()?;
    refresh_all(&mut sys);
    refresh_one(&mut sys, pid);
    let p = sys.process(Pid::from_u32(pid)).ok_or_else(|| AppError::NotFound(format!("PID {pid} artık çalışmıyor")))?;
    let name = p.name().to_string_lossy().into_owned();
    let exe = p.exe().map(|e| e.to_string_lossy().into_owned()).filter(|e| !e.is_empty());
    let cmd: Vec<String> = p.cmd().iter().map(|s| s.to_string_lossy().into_owned()).collect();
    let cwd = p.cwd().map(|c| c.to_string_lossy().into_owned()).filter(|c| !c.is_empty());

    let mut parent_chain = Vec::new();
    let mut cur = p.parent();
    let mut seen = vec![pid];
    while let Some(pp) = cur {
        let ppid = pp.as_u32();
        if seen.contains(&ppid) || parent_chain.len() >= 64 {
            break;
        }
        seen.push(ppid);
        match sys.process(pp) {
            // Üst süreç bu süreçten sonra başlamışsa PID yeniden kullanılmıştır.
            Some(q) if q.start_time() <= p.start_time() || p.start_time() == 0 => {
                parent_chain.push(ProcRef { pid: ppid, name: q.name().to_string_lossy().into_owned() });
                cur = q.parent();
            }
            _ => break,
        }
    }
    let mut children: Vec<ProcRef> = sys
        .processes()
        .values()
        .filter(|c| c.parent().map(|x| x.as_u32()) == Some(pid) && c.start_time() >= p.start_time())
        .map(|c| ProcRef { pid: c.pid().as_u32(), name: c.name().to_string_lossy().into_owned() })
        .collect();
    children.sort_by_key(|c| c.pid);

    let signature = exe.as_deref().filter(|e| Path::new(e).is_file()).map(|e| signature::verify(Path::new(e)));
    let ctx = PathContext::current();
    let mut indicators = process_indicators(&ctx, &name, exe.as_deref(), |e| Path::new(e).exists());
    if let Some(s) = &signature {
        match s.state.as_str() {
            "unsigned" => indicators.push(Indicator::new("unsigned")),
            "invalid" => indicators.push(Indicator::new("invalid_signature")),
            _ => {}
        }
    }
    let mut connections: Vec<Connection> = connections_snapshot().into_iter().filter(|c| c.pid == pid).collect();
    connections.sort_by(|a, b| (a.proto.as_str(), a.local_port).cmp(&(b.proto.as_str(), b.local_port)));
    let (prot, reason) = crate::monitor::protection_of(&name, exe.as_deref(), pid, std::process::id());
    let check = restart_check(&sys, pid);
    let restart_note = Some(match &check {
        Ok(()) => "Uygulama kapatılıp aynı komut satırı ve çalışma klasörüyle yeniden başlatılır. Kaydedilmemiş veriler kaybolabilir.".to_string(),
        Err(r) => r.clone(),
    });
    Ok(ProcessDetails {
        pid,
        name,
        exe,
        cmd,
        cwd,
        start_time: p.start_time(),
        parent_chain,
        children,
        signature,
        connections,
        indicators,
        protection: prot.to_string(),
        protection_reason: reason.map(str::to_string),
        can_restart: check.is_ok(),
        restart_note,
    })
}

/// Süreci sonlandırıp aynı komut satırıyla yeniden başlatır.
pub fn restart(db: &Db, pid: u32, expected_name: &str) -> AppResult<ProcessRestartResult> {
    let mut sys = new_system()?;
    refresh_all(&mut sys);
    refresh_one(&mut sys, pid);
    let p = sys.process(Pid::from_u32(pid)).ok_or_else(|| AppError::NotFound(format!("PID {pid} artık çalışmıyor")))?;
    let name = p.name().to_string_lossy().into_owned();
    if !name.eq_ignore_ascii_case(expected_name) {
        return Err(AppError::Conflict(format!("PID {pid} artık başka bir sürece ({name}) ait; liste yenilendi")));
    }
    restart_check(&sys, pid).map_err(AppError::Protected)?;
    let exe = p.exe().map(|e| e.to_path_buf()).ok_or_else(|| AppError::System("program yolu okunamadı".into()))?;
    let args: Vec<std::ffi::OsString> = p.cmd().iter().skip(1).cloned().collect();
    let cwd = p.cwd().map(|c| c.to_path_buf()).filter(|c| c.is_dir());
    let start_time = p.start_time();
    let detail_base = serde_json::json!({
        "pid": pid,
        "name": name,
        "exe": exe.to_string_lossy(),
        // Komut satırı bağımsız değişkenleri parola veya belirteç içerebilir;
        // geçmişe yalnızca sayıları yazılır.
        "argCount": args.len(),
        "cwd": cwd.as_ref().map(|c| c.to_string_lossy().into_owned()),
    });

    if !p.kill() {
        let e = AppError::PermissionDenied(format!(
            "{name} sonlandırılamadı; süreç yönetici hakları gerektiriyor olabilir."
        ));
        let mut d = detail_base.clone();
        d["error"] = serde_json::Value::String(e.to_string());
        let _ =
            oplog::record(db, "process", &format!("Süreç yeniden başlatılamadı: {name}"), Outcome::Failed, false, d);
        return Err(e);
    }
    // Çıkışı en fazla 5 saniye bekle.
    let mut exited = false;
    for _ in 0..50 {
        sys.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), true);
        match sys.process(Pid::from_u32(pid)) {
            Some(q) if q.start_time() == start_time => std::thread::sleep(std::time::Duration::from_millis(100)),
            _ => {
                exited = true;
                break;
            }
        }
    }
    if !exited {
        let e = AppError::System(format!("{name} 5 saniye içinde kapanmadı; yeniden başlatılmadı."));
        let mut d = detail_base.clone();
        d["error"] = serde_json::Value::String(e.to_string());
        let _ =
            oplog::record(db, "process", &format!("Süreç yeniden başlatılamadı: {name}"), Outcome::Partial, false, d);
        return Err(e);
    }
    let mut cmd = std::process::Command::new(&exe);
    cmd.args(&args);
    if let Some(c) = &cwd {
        cmd.current_dir(c);
    }
    match cmd.spawn() {
        Ok(child) => {
            let new_pid = child.id();
            drop(child);
            let mut d = detail_base;
            d["newPid"] = serde_json::json!(new_pid);
            let _ =
                oplog::record(db, "process", &format!("Süreç yeniden başlatıldı: {name}"), Outcome::Success, false, d);
            Ok(ProcessRestartResult {
                old_pid: pid,
                new_pid,
                message: format!("{name} kapatıldı ve yeniden başlatıldı (yeni PID {new_pid})."),
            })
        }
        Err(err) => {
            let e = AppError::System(format!("{name} kapatıldı ancak yeniden başlatılamadı: {err}"));
            let mut d = detail_base;
            d["error"] = serde_json::Value::String(e.to_string());
            let _ = oplog::record(
                db,
                "process",
                &format!("Süreç yeniden başlatılamadı: {name}"),
                Outcome::Partial,
                false,
                d,
            );
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_labels() {
        assert_eq!(tcp_state_label(5), "Bağlı");
        assert_eq!(tcp_state_label(2), "Dinliyor");
        assert_eq!(tcp_state_label(6), "Kapanıyor");
        assert_eq!(tcp_state_label(11), "Kapanış bekleniyor");
        assert_eq!(tcp_state_label(99), "Bilinmiyor");
    }

    #[test]
    fn parse_tcp4_and_udp6() {
        // 1 satır: ESTABLISHED 192.168.1.5:50000 → 93.184.216.34:443, PID 1234
        let mut b = vec![];
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&5u32.to_le_bytes());
        b.extend_from_slice(&[192, 168, 1, 5]);
        b.extend_from_slice(&[0xC3, 0x50, 0, 0]);
        b.extend_from_slice(&[93, 184, 216, 34]);
        b.extend_from_slice(&[0x01, 0xBB, 0, 0]);
        b.extend_from_slice(&1234u32.to_le_bytes());
        let v = parse_table(0, &b);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].local_address, "192.168.1.5");
        assert_eq!(v[0].local_port, 50000);
        assert_eq!(v[0].remote_address.as_deref(), Some("93.184.216.34"));
        assert_eq!(v[0].remote_port, Some(443));
        assert_eq!(v[0].state.as_deref(), Some("Bağlı"));
        assert_eq!(v[0].pid, 1234);

        let mut u = vec![];
        u.extend_from_slice(&1u32.to_le_bytes());
        let mut addr = [0u8; 16];
        addr[15] = 1;
        u.extend_from_slice(&addr);
        u.extend_from_slice(&0u32.to_le_bytes());
        u.extend_from_slice(&[0x14, 0xE9, 0, 0]);
        u.extend_from_slice(&77u32.to_le_bytes());
        let v = parse_table(3, &u);
        assert_eq!(v[0].local_address, "::1");
        assert_eq!(v[0].local_port, 5353);
        assert_eq!(v[0].pid, 77);
        assert!(v[0].state.is_none());
        // Kısa arabellek güvenli.
        assert!(parse_table(1, &[5, 0, 0, 0, 1]).is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn snapshot_works() {
        let counts = connection_counts();
        println!("bağlantısı olan süreç sayısı: {}", counts.len());
    }

    #[cfg(windows)]
    #[test]
    fn restart_self_spawned_dummy() {
        let db = Db::open_in_memory().unwrap();
        let ping = crate::platform::system_tool("PING.EXE");
        let mut child = crate::platform::hidden_command(&ping)
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id();
        std::thread::sleep(std::time::Duration::from_millis(500));
        let d = details(pid).unwrap();
        assert_eq!(d.name.to_lowercase(), "ping.exe");
        assert!(d.parent_chain.iter().any(|p| p.pid == std::process::id()));
        // PING.EXE System32 altında olduğundan "sensitive" sayılır ve
        // yeniden başlatılamaz — korumanın çalıştığını doğrular.
        assert!(!d.can_restart);
        assert!(matches!(restart(&db, pid, "PING.EXE"), Err(AppError::Protected(_))));
        // Ad uyuşmazlığı.
        assert!(matches!(restart(&db, pid, "baska.exe"), Err(AppError::Conflict(_))));

        // System32 dışında bir kopya ile gerçek yeniden başlatma.
        let t = tempfile::tempdir().unwrap();
        let copy = t.path().join("ivilytestping.exe");
        std::fs::copy(&ping, &copy).unwrap();
        let mut c2 = std::process::Command::new(&copy)
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let pid2 = c2.id();
        std::thread::sleep(std::time::Duration::from_millis(500));
        let d2 = details(pid2).unwrap();
        println!("dummy ayrıntı: canRestart={} note={:?} sig={:?}", d2.can_restart, d2.restart_note, d2.signature);
        if crate::platform::is_elevated() {
            // Yönetici olarak çalışırken (ör. GitHub Actions koşucuları)
            // yeniden başlatma, komut satırı güvenilmez olduğu için reddedilir.
            assert!(!d2.can_restart);
            assert!(matches!(restart(&db, pid2, "ivilytestping.exe"), Err(AppError::Protected(_))));
            let _ = c2.kill();
            let _ = c2.wait();
            let _ = child.kill();
            let _ = child.wait();
            return;
        }
        assert!(d2.can_restart, "{:?}", d2.restart_note);
        let r = restart(&db, pid2, "ivilytestping.exe").unwrap();
        assert_eq!(r.old_pid, pid2);
        assert_ne!(r.new_pid, pid2);
        let _ = c2.wait();
        // Yeni süreci temizle.
        let mut sys = new_system().unwrap();
        refresh_one(&mut sys, r.new_pid);
        let np = sys.process(Pid::from_u32(r.new_pid)).expect("yeni süreç çalışıyor olmalı");
        assert_eq!(np.name().to_string_lossy().to_lowercase(), "ivilytestping.exe");
        assert!(np.cmd().iter().any(|a| a == "127.0.0.1"));
        assert!(np.kill());
        let log = oplog::list(&db, Some("process"), 10).unwrap();
        assert!(log.iter().any(|e| e.outcome == "success"));

        let _ = child.kill();
        let _ = child.wait();
        // Yeni süreç bitene kadar dosya kilitli olabilir.
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
