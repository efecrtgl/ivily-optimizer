//! Ağ bağdaştırıcıları, kullanıcı tarafından başlatılan bağlantı denetimi ve
//! DNS önbelleği.
//!
//! Bu modül başka ana bilgisayarları taramaz, trafik yakalamaz ve ağ
//! ayarlarını sıfırlamaz.

use super::validate_hostname;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::oplog::{self, Outcome};
use serde::Serialize;
use serde_json::Value;
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpAddrInfo {
    pub address: String,
    pub prefix_length: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetAdapter {
    pub name: String,
    pub description: String,
    /// ethernet, wifi, loopback, other
    pub kind: String,
    pub kind_label: String,
    /// up, down, testing, unknown, dormant, not_present, lower_layer_down
    pub status: String,
    pub status_label: String,
    pub mac: Option<String>,
    pub ipv4: Vec<IpAddrInfo>,
    pub ipv6: Vec<IpAddrInfo>,
    pub gateways: Vec<String>,
    pub dns_servers: Vec<String>,
    /// Bağlantı hızı (bit/sn); bilinmiyorsa null.
    pub link_speed_bps: Option<u64>,
    pub dhcp_enabled: bool,
}

pub fn kind_of(if_type: u32) -> (&'static str, &'static str) {
    match if_type {
        6 => ("ethernet", "Ethernet"),
        71 => ("wifi", "Wi-Fi"),
        24 => ("loopback", "Geri döngü"),
        _ => ("other", "Diğer"),
    }
}

pub fn oper_status(v: i32) -> (&'static str, &'static str) {
    match v {
        1 => ("up", "Bağlı"),
        2 => ("down", "Bağlı değil"),
        3 => ("testing", "Sınanıyor"),
        5 => ("dormant", "Beklemede"),
        6 => ("not_present", "Mevcut değil"),
        7 => ("lower_layer_down", "Alt katman bağlı değil"),
        _ => ("unknown", "Bilinmiyor"),
    }
}

pub fn format_mac(b: &[u8]) -> Option<String> {
    if b.is_empty() || b.iter().all(|x| *x == 0) {
        return None;
    }
    Some(b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join("-"))
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::Win32::NetworkManagement::IpHelper::*;
    use windows::Win32::Networking::WinSock::SOCKET_ADDRESS;

    /// SOCKADDR yapısını ham baytlardan okur (AF_INET=2, AF_INET6=23).
    unsafe fn sockaddr(sa: &SOCKET_ADDRESS) -> Option<IpAddr> {
        if sa.lpSockaddr.is_null() || sa.iSockaddrLength < 8 {
            return None;
        }
        let p = sa.lpSockaddr as *const u8;
        let len = sa.iSockaddrLength as usize;
        let bytes = unsafe { std::slice::from_raw_parts(p, len) };
        let family = u16::from_le_bytes([bytes[0], bytes[1]]);
        match family {
            2 if len >= 8 => Some(IpAddr::from([bytes[4], bytes[5], bytes[6], bytes[7]])),
            23 if len >= 24 => {
                let mut a = [0u8; 16];
                a.copy_from_slice(&bytes[8..24]);
                Some(IpAddr::from(a))
            }
            _ => None,
        }
    }

    pub fn adapters(include_loopback: bool) -> AppResult<Vec<NetAdapter>> {
        let flags = GAA_FLAG_INCLUDE_GATEWAYS | GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST;
        let mut size: u32 = 32 * 1024;
        let mut buf: Vec<u64>;
        let mut tries = 0;
        loop {
            buf = vec![0u64; (size as usize).div_ceil(8)];
            let r = unsafe {
                GetAdaptersAddresses(0, flags, None, Some(buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH), &mut size)
            };
            match r {
                0 => break,
                111 if tries < 4 => {
                    tries += 1;
                    continue;
                }
                232 => return Ok(Vec::new()), // ERROR_NO_DATA
                code => return Err(AppError::System(format!("ağ bağdaştırıcıları okunamadı (kod {code})"))),
            }
        }
        let mut out = Vec::new();
        let mut cur = buf.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
        unsafe {
            while !cur.is_null() {
                let a = &*cur;
                cur = a.Next;
                let (kind, kind_label) = kind_of(a.IfType);
                if kind == "loopback" && !include_loopback {
                    continue;
                }
                let (status, status_label) = oper_status(a.OperStatus.0);
                let mac_len = (a.PhysicalAddressLength as usize).min(8);
                let mut ipv4 = Vec::new();
                let mut ipv6 = Vec::new();
                let mut u = a.FirstUnicastAddress;
                while !u.is_null() {
                    let ua = &*u;
                    if let Some(ip) = sockaddr(&ua.Address) {
                        let info = IpAddrInfo { address: ip.to_string(), prefix_length: ua.OnLinkPrefixLength };
                        if ip.is_ipv4() {
                            ipv4.push(info)
                        } else {
                            ipv6.push(info)
                        }
                    }
                    u = ua.Next;
                }
                let mut gateways = Vec::new();
                let mut g = a.FirstGatewayAddress;
                while !g.is_null() {
                    if let Some(ip) = sockaddr(&(*g).Address) {
                        let s = ip.to_string();
                        if !gateways.contains(&s) {
                            gateways.push(s);
                        }
                    }
                    g = (*g).Next;
                }
                let mut dns_servers = Vec::new();
                let mut d = a.FirstDnsServerAddress;
                while !d.is_null() {
                    if let Some(ip) = sockaddr(&(*d).Address) {
                        let s = ip.to_string();
                        if !dns_servers.contains(&s) {
                            dns_servers.push(s);
                        }
                    }
                    d = (*d).Next;
                }
                let speed = a.ReceiveLinkSpeed.max(a.TransmitLinkSpeed);
                let flags = a.Anonymous2.Flags;
                out.push(NetAdapter {
                    name: crate::sysmgmt::pwstr_to_string(a.FriendlyName),
                    description: crate::sysmgmt::pwstr_to_string(a.Description),
                    kind: kind.into(),
                    kind_label: kind_label.into(),
                    status: status.into(),
                    status_label: status_label.into(),
                    mac: format_mac(&a.PhysicalAddress[..mac_len]),
                    ipv4,
                    ipv6,
                    gateways,
                    dns_servers,
                    link_speed_bps: (status == "up" && speed != 0 && speed != u64::MAX).then_some(speed),
                    dhcp_enabled: flags & 0x4 != 0, // IP_ADAPTER_DHCP_ENABLED
                });
            }
        }
        Ok(out)
    }
}

#[cfg(windows)]
pub fn adapters(include_loopback: bool) -> AppResult<Vec<NetAdapter>> {
    let mut v = imp::adapters(include_loopback)?;
    // Bağlı olanlar önce.
    v.sort_by_key(|a| (a.status != "up", a.name.to_lowercase()));
    Ok(v)
}
#[cfg(not(windows))]
pub fn adapters(_include_loopback: bool) -> AppResult<Vec<NetAdapter>> {
    Err(AppError::Unsupported("yalnızca Windows".into()))
}

// ---------------------------------------------------------------------------
// Bağlantı denetimi
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TcpCheck {
    pub target: String,
    pub label: String,
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsCheck {
    pub host: String,
    pub ok: bool,
    pub address_count: usize,
    pub addresses: Vec<String>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityReport {
    pub tcp: Vec<TcpCheck>,
    pub dns: Vec<DnsCheck>,
    /// ok, partial, dns_failed, offline
    pub status: String,
    pub summary: String,
    pub checked_at: i64,
}

fn tcp_check(addr: SocketAddr, label: &str) -> TcpCheck {
    let start = Instant::now();
    match TcpStream::connect_timeout(&addr, Duration::from_secs(3)) {
        Ok(s) => {
            let _ = s.shutdown(std::net::Shutdown::Both);
            TcpCheck { target: addr.to_string(), label: label.into(), ok: true, latency_ms: Some(start.elapsed().as_millis() as u64), error: None }
        }
        Err(e) => {
            let msg = match e.kind() {
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => "3 saniye içinde yanıt alınamadı".to_string(),
                std::io::ErrorKind::ConnectionRefused => "bağlantı reddedildi".to_string(),
                _ => match e.raw_os_error() {
                    Some(10051) | Some(10065) => "ağa ulaşılamıyor".to_string(),
                    _ => "bağlantı kurulamadı".to_string(),
                },
            };
            TcpCheck { target: addr.to_string(), label: label.into(), ok: false, latency_ms: None, error: Some(msg) }
        }
    }
}

/// Sistem çözümleyicisiyle ad çözümler (yinelenen adresler ayıklanır).
pub fn resolve(host: &str) -> DnsCheck {
    let start = Instant::now();
    let res = (host, 0u16).to_socket_addrs();
    let duration_ms = start.elapsed().as_millis() as u64;
    match res {
        Ok(it) => {
            let mut addrs: Vec<String> = Vec::new();
            for a in it {
                let s = a.ip().to_string();
                if !addrs.contains(&s) {
                    addrs.push(s);
                }
            }
            let ok = !addrs.is_empty();
            DnsCheck {
                host: host.into(),
                ok,
                address_count: addrs.len(),
                addresses: addrs,
                duration_ms,
                error: (!ok).then(|| "adres bulunamadı".to_string()),
            }
        }
        Err(_) => DnsCheck {
            host: host.into(),
            ok: false,
            address_count: 0,
            addresses: Vec::new(),
            duration_ms,
            error: Some("ad çözümlenemedi".into()),
        },
    }
}

pub fn summarize(tcp: &[TcpCheck], dns: &[DnsCheck]) -> (&'static str, &'static str) {
    let tcp_ok = tcp.iter().filter(|t| t.ok).count();
    let dns_ok = dns.iter().filter(|d| d.ok).count();
    if tcp_ok == tcp.len() && dns_ok == dns.len() {
        ("ok", "İnternet bağlantısı ve DNS çözümlemesi çalışıyor.")
    } else if tcp_ok == 0 && dns_ok == 0 {
        ("offline", "İnternet bağlantısı kurulamadı. Ağ kablonuzu veya Wi-Fi bağlantınızı ve modeminizi kontrol edin.")
    } else if tcp_ok > 0 && dns_ok == 0 {
        ("dns_failed", "İnternete erişim var ancak alan adları çözümlenemiyor. DNS sunucusu ayarlarınızı kontrol edin.")
    } else {
        ("partial", "Bağlantı kısmen çalışıyor; bazı denetimler başarısız oldu. Bir güvenlik duvarı veya ağ kısıtlaması bazı bağlantıları engelliyor olabilir.")
    }
}

/// Kullanıcının başlattığı bağlantı denetimi. Denetimler paralel çalışır.
pub fn connectivity_check() -> ConnectivityReport {
    let targets: [(SocketAddr, &str); 2] = [
        (SocketAddr::from(([1, 1, 1, 1], 443)), "Cloudflare (HTTPS, 443)"),
        (SocketAddr::from(([8, 8, 8, 8], 53)), "Google DNS (TCP 53)"),
    ];
    let hosts = ["github.com", "www.microsoft.com"];
    let tcp_handles: Vec<_> = targets.iter().map(|(a, l)| {
        let (a, l) = (*a, l.to_string());
        std::thread::spawn(move || tcp_check(a, &l))
    }).collect();
    let dns_handles: Vec<_> = hosts.iter().map(|h| {
        let h = h.to_string();
        std::thread::spawn(move || resolve(&h))
    }).collect();
    let tcp: Vec<TcpCheck> = tcp_handles
        .into_iter()
        .zip(targets.iter())
        .map(|(h, (a, l))| h.join().unwrap_or_else(|_| TcpCheck { target: a.to_string(), label: l.to_string(), ok: false, latency_ms: None, error: Some("denetim tamamlanamadı".into()) }))
        .collect();
    let dns: Vec<DnsCheck> = dns_handles
        .into_iter()
        .zip(hosts.iter())
        .map(|(h, host)| h.join().unwrap_or_else(|_| DnsCheck { host: host.to_string(), ok: false, address_count: 0, addresses: vec![], duration_ms: 0, error: Some("denetim tamamlanamadı".into()) }))
        .collect();
    let (status, summary) = summarize(&tcp, &dns);
    ConnectivityReport { tcp, dns, status: status.into(), summary: summary.into(), checked_at: crate::db::now_ts() }
}

/// Kullanıcının girdiği ana bilgisayar adını çözümler.
pub fn dns_lookup(host: &str) -> AppResult<DnsCheck> {
    let h = validate_hostname(host)?;
    Ok(resolve(&h))
}

// ---------------------------------------------------------------------------
// DNS önbelleği
// ---------------------------------------------------------------------------

const DNS_CACHE_SCRIPT: &str = r#"[Console]::OutputEncoding=[System.Text.Encoding]::UTF8;
$ErrorActionPreference='SilentlyContinue';
$all=@(Get-DnsClientCache);
$items=@($all | Select-Object -First 2000 | ForEach-Object { [pscustomobject]@{ Entry=$_.Entry; RecordName=$_.RecordName; Type=[int]$_.Type; Status=[int]$_.Status; TimeToLive=[int64]$_.TimeToLive; Data=[string]$_.Data } });
ConvertTo-Json -InputObject ([pscustomobject]@{ Total=$all.Count; Items=$items }) -Depth 3 -Compress"#;

pub const DNS_CACHE_CAP: usize = 2000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsCacheEntry {
    pub entry: String,
    pub record_name: String,
    pub record_type: String,
    pub status: String,
    pub ttl_seconds: Option<i64>,
    pub data: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsCacheList {
    pub entries: Vec<DnsCacheEntry>,
    pub total: usize,
    pub truncated: bool,
}

pub fn record_type_name(v: &Value) -> String {
    match v {
        Value::Number(n) => match n.as_u64().unwrap_or(0) {
            1 => "A".into(),
            2 => "NS".into(),
            5 => "CNAME".into(),
            6 => "SOA".into(),
            12 => "PTR".into(),
            15 => "MX".into(),
            16 => "TXT".into(),
            28 => "AAAA".into(),
            33 => "SRV".into(),
            64 => "SVCB".into(),
            65 => "HTTPS".into(),
            other => format!("Tür {other}"),
        },
        Value::String(s) => s.clone(),
        _ => "Bilinmiyor".into(),
    }
}

pub fn status_name(v: &Value) -> String {
    match v {
        Value::Number(n) => match n.as_u64().unwrap_or(u64::MAX) {
            0 => "Başarılı".into(),
            9003 => "Ad mevcut değil".into(),
            9501 => "Kayıt yok".into(),
            9701 => "Bu türde kayıt yok".into(),
            other => format!("Kod {other}"),
        },
        Value::String(s) if s == "Success" => "Başarılı".into(),
        Value::String(s) => s.clone(),
        _ => "Bilinmiyor".into(),
    }
}

pub fn parse_dns_cache(json: &str) -> AppResult<DnsCacheList> {
    let t = json.trim().trim_start_matches('\u{feff}');
    if t.is_empty() {
        return Ok(DnsCacheList { entries: vec![], total: 0, truncated: false });
    }
    let bad = || AppError::System("DNS önbelleği okunamadı (beklenmeyen çıktı)".into());
    let root: Value = serde_json::from_str(t).map_err(|_| bad())?;
    let items = match root.get("Items") {
        Some(Value::Array(a)) => a.clone(),
        Some(Value::Null) | None => Vec::new(),
        Some(o) => vec![o.clone()],
    };
    let str_of = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let entries: Vec<DnsCacheEntry> = items
        .iter()
        .take(DNS_CACHE_CAP)
        .map(|v| DnsCacheEntry {
            entry: str_of(v, "Entry"),
            record_name: str_of(v, "RecordName"),
            record_type: record_type_name(v.get("Type").unwrap_or(&Value::Null)),
            status: status_name(v.get("Status").unwrap_or(&Value::Null)),
            ttl_seconds: v.get("TimeToLive").and_then(Value::as_i64),
            data: str_of(v, "Data"),
        })
        .collect();
    let total = root.get("Total").and_then(Value::as_u64).map(|n| n as usize).unwrap_or(entries.len()).max(entries.len());
    Ok(DnsCacheList { truncated: total > entries.len(), total, entries })
}

pub fn dns_cache_list() -> AppResult<DnsCacheList> {
    let out = super::run_powershell(DNS_CACHE_SCRIPT, &[], "DNS önbelleği")?;
    parse_dns_cache(&out)
}

/// DNS çözümleyici önbelleğini temizler (`ipconfig /flushdns`). Geri
/// alınamaz; kayıtlar ihtiyaç duyuldukça yeniden oluşturulur.
pub fn dns_cache_flush(db: &Db) -> AppResult<String> {
    let before = dns_cache_list().map(|l| l.total).ok();
    let out = crate::platform::hidden_command(&crate::platform::system_tool("ipconfig.exe"))
        .arg("/flushdns")
        .output()
        .map_err(|e| AppError::io(e, "ipconfig.exe çalıştırılamadı"))?;
    let ok = out.status.success();
    let _ = oplog::record(
        db,
        "network",
        if ok { "DNS önbelleği temizlendi" } else { "DNS önbelleği temizlenemedi" },
        if ok { Outcome::Success } else { Outcome::Failed },
        false,
        serde_json::json!({ "entriesBefore": before, "exitCode": out.status.code() }),
    );
    if !ok {
        return Err(AppError::System("DNS önbelleği temizlenemedi".into()));
    }
    Ok("DNS önbelleği temizlendi. Alan adları bir sonraki erişimde yeniden çözümlenecek.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers() {
        assert_eq!(kind_of(71).1, "Wi-Fi");
        assert_eq!(kind_of(6).0, "ethernet");
        assert_eq!(kind_of(131).0, "other");
        assert_eq!(oper_status(1).1, "Bağlı");
        assert_eq!(format_mac(&[0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E]).as_deref(), Some("00-1A-2B-3C-4D-5E"));
        assert_eq!(format_mac(&[]), None);
        assert_eq!(format_mac(&[0, 0, 0]), None);
    }

    #[test]
    fn summaries() {
        let t = |ok| TcpCheck { target: "x".into(), label: "x".into(), ok, latency_ms: None, error: None };
        let d = |ok| DnsCheck { host: "x".into(), ok, address_count: 0, addresses: vec![], duration_ms: 0, error: None };
        assert_eq!(summarize(&[t(true), t(true)], &[d(true), d(true)]).0, "ok");
        assert_eq!(summarize(&[t(false), t(false)], &[d(false), d(false)]).0, "offline");
        assert_eq!(summarize(&[t(true), t(false)], &[d(false), d(false)]).0, "dns_failed");
        assert_eq!(summarize(&[t(true), t(false)], &[d(true), d(true)]).0, "partial");
    }

    #[test]
    fn dns_cache_parsing() {
        let j = r#"{"Total":3,"Items":[{"Entry":"github.com","RecordName":"github.com","Type":1,"Status":0,"TimeToLive":42,"Data":"140.82.121.4"},{"Entry":"x.test","RecordName":"x.test","Type":28,"Status":9003,"TimeToLive":0,"Data":""}]}"#;
        let l = parse_dns_cache(j).unwrap();
        assert_eq!(l.entries.len(), 2);
        assert_eq!(l.total, 3);
        assert!(l.truncated);
        assert_eq!(l.entries[0].record_type, "A");
        assert_eq!(l.entries[0].status, "Başarılı");
        assert_eq!(l.entries[0].ttl_seconds, Some(42));
        assert_eq!(l.entries[1].record_type, "AAAA");
        assert_eq!(l.entries[1].status, "Ad mevcut değil");
        let single = r#"{"Total":1,"Items":{"Entry":"a","RecordName":"a","Type":5,"Status":0,"TimeToLive":1,"Data":"b"}}"#;
        assert_eq!(parse_dns_cache(single).unwrap().entries[0].record_type, "CNAME");
        let empty = r#"{"Total":0,"Items":[]}"#;
        assert_eq!(parse_dns_cache(empty).unwrap().entries.len(), 0);
        assert_eq!(parse_dns_cache("").unwrap().total, 0);
        assert!(parse_dns_cache("bozuk").is_err());
    }

    #[test]
    fn lookup_validation() {
        assert!(matches!(dns_lookup("a b"), Err(AppError::InvalidInput(_))));
        assert!(matches!(dns_lookup("evil.com & calc"), Err(AppError::InvalidInput(_))));
    }

    #[cfg(windows)]
    #[test]
    fn adapters_readonly() {
        let v = adapters(false).unwrap();
        assert!(v.iter().all(|a| a.kind != "loopback"));
        let with_lo = adapters(true).unwrap();
        assert!(with_lo.len() >= v.len());
    }

    #[cfg(windows)]
    #[test]
    fn dns_cache_readonly() {
        let l = dns_cache_list().unwrap();
        assert!(l.entries.len() <= DNS_CACHE_CAP);
    }
}
