//! Güç planları (`powercfg.exe`) ve pil durumu.

use super::{is_guid, validate_guid, ChangeResult};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::oplog::{self, Outcome};
use crate::platform::{hidden_command, system_tool};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerPlan {
    pub guid: String,
    pub name: String,
    pub active: bool,
    /// Windows ile gelen bilinen bir plan mı?
    pub builtin: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryStatus {
    /// Şebeke bağlı mı? (bilinmiyorsa null)
    pub ac_online: Option<bool>,
    pub has_battery: bool,
    pub percent: Option<u8>,
    pub charging: bool,
    pub seconds_remaining: Option<u32>,
    pub battery_saver_on: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerOverview {
    pub plans: Vec<PowerPlan>,
    pub active_guid: Option<String>,
    pub battery: Option<BatteryStatus>,
    pub note: String,
}

pub const BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";
pub const HIGH_PERFORMANCE: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
/// winnt.h: GUID_MAX_POWER_SAVINGS
pub const POWER_SAVER: &str = "a1841308-3541-4fab-bc81-f71556f20b4a";
pub const ULTIMATE: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";

/// Bilinen planlar için dürüst, abartısız açıklama.
pub fn describe(guid: &str) -> (bool, &'static str) {
    match guid {
        BALANCED => (
            true,
            "Windows'un varsayılan planı. İşlemci hızını o anki ihtiyaca göre artırıp azaltır; çoğu kullanım için performans ile enerji tüketimi arasında iyi bir denge sağlar.",
        ),
        HIGH_PERFORMANCE => (
            true,
            "İşlemcinin düşük güç durumlarına inmesini sınırlar ve uyku/ekran kapatma sürelerini uzatır. Enerji tüketimini ve ısınmayı artırır, pil ömrünü kısaltır. Oyunlarda daha yüksek FPS garanti etmez; fark çoğu sistemde küçüktür veya hiç yoktur.",
        ),
        POWER_SAVER => (
            true,
            "İşlemci performansını ve bazı donanım özelliklerini sınırlayarak enerji tasarrufu sağlar. Pil ömrünü uzatabilir; buna karşılık sistem daha yavaş tepki verebilir.",
        ),
        ULTIMATE => (
            true,
            "Yüksek performans planına benzer; güç tasarrufu özelliklerini daha da azaltır. İş istasyonları için tasarlanmıştır. Enerji tüketimini artırır, dizüstü bilgisayarlarda pil ömrünü belirgin biçimde kısaltır ve FPS artışı garanti etmez.",
        ),
        _ => (
            false,
            "Bilgisayar üreticisi veya kullanıcı tarafından oluşturulmuş bir güç planı. Ayarlarının ayrıntılarını Denetim Masası'ndaki Güç Seçenekleri bölümünde görebilirsiniz.",
        ),
    }
}

/// Bir satırdaki ilk GUID'in konumunu bulur.
fn find_guid(line: &str) -> Option<(usize, String)> {
    let b = line.as_bytes();
    if b.len() < 36 {
        return None;
    }
    (0..=b.len() - 36).find_map(|i| {
        let cand = line.get(i..i + 36)?;
        is_guid(cand).then(|| (i, cand.to_ascii_lowercase()))
    })
}

/// Tek bir `powercfg` satırını ayrıştırır: GUID, parantez içindeki ad ve
/// sondaki `*` (etkin) işareti. Dil bağımsızdır.
pub fn parse_scheme_line(line: &str) -> Option<(String, String, bool)> {
    let (pos, guid) = find_guid(line)?;
    let rest = &line[pos + 36..];
    let name = match (rest.find('('), rest.rfind(')')) {
        (Some(a), Some(b)) if b > a => rest[a + 1..b].trim().to_string(),
        _ => String::new(),
    };
    let active = rest.trim_end().ends_with('*');
    Some((guid, name, active))
}

/// `powercfg /list` çıktısını ayrıştırır.
pub fn parse_list(text: &str) -> Vec<(String, String, bool)> {
    let mut v: Vec<(String, String, bool)> = Vec::new();
    for line in text.lines() {
        if let Some(entry) = parse_scheme_line(line) {
            if !v.iter().any(|(g, _, _)| *g == entry.0) {
                v.push(entry);
            }
        }
    }
    v
}

fn powercfg(args: &[&str]) -> AppResult<(bool, String)> {
    let out = hidden_command(&system_tool("powercfg.exe"))
        .args(args)
        .output()
        .map_err(|e| AppError::io(e, "powercfg.exe çalıştırılamadı"))?;
    let text = super::decode_console(&out.stdout);
    Ok((out.status.success(), text))
}

fn active_guid() -> Option<String> {
    let (ok, text) = powercfg(&["/getactivescheme"]).ok()?;
    if !ok {
        return None;
    }
    text.lines().find_map(parse_scheme_line).map(|(g, _, _)| g)
}

#[cfg(windows)]
pub fn battery() -> Option<BatteryStatus> {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut s = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut s).ok()? };
    let has_battery = s.BatteryFlag != 128 && s.BatteryFlag != 255;
    Some(BatteryStatus {
        ac_online: match s.ACLineStatus {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        },
        has_battery,
        percent: (has_battery && s.BatteryLifePercent <= 100).then_some(s.BatteryLifePercent),
        charging: has_battery && s.BatteryFlag & 8 != 0,
        seconds_remaining: (has_battery && s.BatteryLifeTime != u32::MAX).then_some(s.BatteryLifeTime),
        battery_saver_on: s.SystemStatusFlag == 1,
    })
}
#[cfg(not(windows))]
pub fn battery() -> Option<BatteryStatus> {
    None
}

pub fn list() -> AppResult<PowerOverview> {
    let (ok, text) = powercfg(&["/list"])?;
    if !ok {
        return Err(AppError::System("güç planları okunamadı (powercfg.exe hatası)".into()));
    }
    let active = active_guid();
    let plans: Vec<PowerPlan> = parse_list(&text)
        .into_iter()
        .map(|(guid, name, star)| {
            let (builtin, d) = describe(&guid);
            PowerPlan {
                active: active.as_deref().map(|a| a == guid).unwrap_or(star),
                name: if name.is_empty() { "Adsız plan".into() } else { name },
                builtin,
                description: d.to_string(),
                guid,
            }
        })
        .collect();
    let note = if plans.len() <= 1 {
        "Bazı yeni bilgisayarlarda (Modern Bekleme kullanan sistemler) yalnızca Dengeli plan bulunur; performans ayarı Windows Ayarlar > Sistem > Güç bölümündeki güç modu ile yapılır.".into()
    } else {
        "Güç planı değişikliği yönetici izni gerektirmez ve istediğiniz zaman geri alınabilir. Yüksek performanslı planlar enerji tüketimini artırır; oyun performansında belirgin bir artış garanti etmez.".into()
    };
    Ok(PowerOverview { active_guid: active.or_else(|| plans.iter().find(|p| p.active).map(|p| p.guid.clone())), plans, battery: battery(), note })
}

/// Etkin güç planını değiştirir (yönetici izni gerekmez).
pub fn set_active(db: &Db, guid: &str) -> AppResult<ChangeResult<PowerPlan>> {
    let guid = validate_guid(guid)?;
    let before = list()?;
    let Some(target) = before.plans.iter().find(|p| p.guid == guid).cloned() else {
        return Err(AppError::NotFound("bu güç planı sistemde bulunamadı".into()));
    };
    if target.active {
        return Ok(ChangeResult { message: format!("\"{}\" zaten etkin.", target.name), item: target, elevated: false, oplog_id: None });
    }
    let previous = before.active_guid.clone();
    let previous_name = before.plans.iter().find(|p| Some(&p.guid) == previous.as_ref()).map(|p| p.name.clone());
    let (ok, _) = powercfg(&["/setactive", &guid])?;
    let now = active_guid();
    if !ok || now.as_deref() != Some(guid.as_str()) {
        let e = AppError::System("güç planı değiştirilemedi".into());
        let _ = oplog::record(
            db,
            "power",
            &format!("Güç planı değiştirilemedi: {}", target.name),
            Outcome::Failed,
            false,
            serde_json::json!({ "previousGuid": previous, "requestedGuid": guid, "error": e.to_string() }),
        );
        return Err(e);
    }
    let oplog_id = oplog::record(
        db,
        "power",
        &format!("Güç planı değiştirildi: {} → {}", previous_name.as_deref().unwrap_or("bilinmiyor"), target.name),
        Outcome::Success,
        previous.is_some(),
        serde_json::json!({ "previousGuid": previous, "previousName": previous_name, "newGuid": guid, "newName": target.name }),
    )
    .ok();
    let item = PowerPlan { active: true, ..target };
    Ok(ChangeResult { message: format!("\"{}\" güç planı etkinleştirildi.", item.name), item, elevated: false, oplog_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TR: &str = "Varolan Güç Düzenleri (* Etkin)\r\n-----------------------------------\r\nGüç Düzeni GUID'si: 381b4222-f694-41f0-9685-ff5bb260df2e  (Dengeli) *\r\nGüç Düzeni GUID'si: 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c  (Yüksek performans)\r\nGüç Düzeni GUID'si: a1841308-3541-4fab-bc81-f71556f20b4a  (Güç tasarrufu)\r\n";
    const EN: &str = "Existing Power Schemes (* Active)\n-----------------------------------\nPower Scheme GUID: 27fa6203-3987-4dcc-918d-748559d549ec  (Performance)\nPower Scheme GUID: 381b4222-f694-41f0-9685-ff5bb260df2e  (Balanced)\nPower Scheme GUID: 6fecc5ae-f350-48a5-b669-b472cb895ccf  (Turbo (OEM)) *\n";

    #[test]
    fn parse_turkish() {
        let v = parse_list(TR);
        assert_eq!(v.len(), 3);
        assert_eq!(v[0], (BALANCED.to_string(), "Dengeli".to_string(), true));
        assert_eq!(v[1].1, "Yüksek performans");
        assert!(!v[1].2);
        assert_eq!(v[2].0, POWER_SAVER);
        assert_eq!(v[2].1, "Güç tasarrufu");
    }

    #[test]
    fn parse_english_nested_parens_and_active() {
        let v = parse_list(EN);
        assert_eq!(v.len(), 3);
        assert_eq!(v[2].1, "Turbo (OEM)");
        assert!(v[2].2);
        assert!(!v[0].2);
        // /getactivescheme çıktısı (sonunda satır sonu olmadan)
        let a = parse_scheme_line("Power Scheme GUID: 6FECC5AE-F350-48A5-B669-B472CB895CCF  (Turbo)").unwrap();
        assert_eq!(a.0, "6fecc5ae-f350-48a5-b669-b472cb895ccf");
        assert!(parse_scheme_line("-----------").is_none());
        assert!(parse_list("").is_empty());
    }

    #[test]
    fn descriptions_are_honest() {
        assert!(describe(BALANCED).0);
        assert!(describe(HIGH_PERFORMANCE).1.contains("garanti etmez"));
        assert!(describe(ULTIMATE).1.contains("garanti etmez"));
        let (b, d) = describe("27fa6203-3987-4dcc-918d-748559d549ec");
        assert!(!b);
        assert!(!d.is_empty());
    }

    #[test]
    fn invalid_guid_rejected() {
        let db = Db::open_in_memory().unwrap();
        assert!(matches!(set_active(&db, "381b4222; shutdown"), Err(AppError::InvalidInput(_))));
        assert!(matches!(set_active(&db, "{381b4222-f694-41f0-9685-ff5bb260df2e}"), Err(AppError::InvalidInput(_))));
    }

    #[cfg(windows)]
    #[test]
    fn list_readonly() {
        let o = list().unwrap();
        assert!(!o.plans.is_empty());
        assert!(o.active_guid.is_some());
        assert_eq!(o.plans.iter().filter(|p| p.active).count(), 1);
    }
}
