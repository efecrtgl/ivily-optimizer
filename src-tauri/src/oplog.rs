//! İşlem geçmişi: temizlik, karantina, başlangıç/servis değişiklikleri vb.
//!
//! Her kayıt için ayrıntı (`detail`) JSON'undan aşağıdaki alanlar türetilir:
//! hedef, önceki/yeni durum, kullanıcı onayı, hata ve geri alma durumu. Bir
//! kayıt yalnızca önceki durumu gerçekten geri getirilebiliyorsa
//! "available" (geri alınabilir) olarak işaretlenir.

use crate::db::{now_ts, Db};
use crate::error::AppResult;
use rusqlite::{params, Connection};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Partial,
    Failed,
    Cancelled,
}

impl Outcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Outcome::Success => "success",
            Outcome::Partial => "partial",
            Outcome::Failed => "failed",
            Outcome::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpEntry {
    pub id: i64,
    pub ts: i64,
    pub kind: String,
    pub title: String,
    pub outcome: String,
    pub reversible: bool,
    pub detail: Value,
    pub target: Option<String>,
    pub previous_state: Option<Value>,
    pub new_state: Option<Value>,
    pub confirmed: bool,
    /// "none" | "available" | "restored" | "failed"
    pub restore_status: String,
    pub restored_at: Option<i64>,
    pub restore_op: Option<i64>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Türetilmiş alanlar
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Derived {
    pub target: Option<String>,
    pub prev_state: Option<Value>,
    pub new_state: Option<Value>,
    pub confirmed: bool,
    pub error: Option<String>,
    pub restore_status: &'static str,
}

fn s<'a>(d: &'a Value, k: &str) -> Option<&'a str> {
    d.get(k).and_then(Value::as_str).filter(|v| !v.is_empty())
}

fn non_empty_array(d: &Value, k: &str) -> bool {
    d.get(k).and_then(Value::as_array).map(|a| !a.is_empty()).unwrap_or(false)
}

fn derive_target(kind: &str, d: &Value) -> Option<String> {
    if kind == "task" {
        if let (Some(p), Some(n)) = (s(d, "path"), s(d, "name")) {
            return Some(format!("{p}{n}"));
        }
    }
    for k in ["displayName", "name", "original", "path", "id"] {
        if let Some(v) = s(d, k) {
            return Some(v.to_string());
        }
    }
    // Tek öğeli dosya / klasör işlemleri.
    for k in ["items", "paths"] {
        if let Some(a) = d.get(k).and_then(Value::as_array) {
            if a.len() == 1 {
                if let Some(v) = a[0].as_str() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

fn pick_prefixed(d: &Value, prefixes: &[&str]) -> Option<Value> {
    let obj = d.as_object()?;
    let m: serde_json::Map<String, Value> = obj
        .iter()
        .filter(|(k, _)| {
            prefixes.iter().any(|p| {
                k.starts_with(p) && k[p.len()..].chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false)
            })
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if m.is_empty() {
        None
    } else {
        Some(Value::Object(m))
    }
}

/// Arayüzün her zaman onay istediği işlem türleri.
fn is_confirmed(kind: &str, title: &str) -> bool {
    match kind {
        "cleanup" | "files" | "startup" | "service" | "task" | "power" | "network" | "process" | "folders" => true,
        "quarantine" => title.starts_with("Karantinadan kalıcı silme"),
        _ => false,
    }
}

/// Kaydın önceki durumu gerçekten geri getirilebilir mi?
pub fn can_restore(kind: &str, d: &Value) -> bool {
    match kind {
        "startup" => s(d, "id").is_some() && d.get("previousEnabled").map(Value::is_boolean).unwrap_or(false),
        "task" => {
            s(d, "path").is_some()
                && s(d, "name").is_some()
                && d.get("previousEnabled").map(Value::is_boolean).unwrap_or(false)
        }
        "power" => s(d, "previousGuid").is_some(),
        "service" => {
            s(d, "action") == Some("start_type")
                && s(d, "name").is_some()
                && matches!(s(d, "previousStartType"), Some("auto" | "auto_delayed" | "manual" | "disabled"))
        }
        "files" => s(d, "mode") == Some("quarantine") && non_empty_array(d, "quarantineIds"),
        "cleanup" => {
            d.get("usedQuarantine").and_then(Value::as_bool).unwrap_or(false) && non_empty_array(d, "quarantineIds")
        }
        "folders" => non_empty_array(d, "paths"),
        _ => false,
    }
}

pub fn derive(kind: &str, title: &str, outcome: &str, reversible: bool, d: &Value) -> Derived {
    Derived {
        target: derive_target(kind, d),
        prev_state: pick_prefixed(d, &["previous"]),
        new_state: pick_prefixed(d, &["new", "requested"]),
        confirmed: is_confirmed(kind, title),
        error: s(d, "error").map(str::to_string),
        restore_status: if reversible && outcome == "success" && can_restore(kind, d) { "available" } else { "none" },
    }
}

fn opt_json(v: &Option<Value>) -> Option<String> {
    v.as_ref().map(Value::to_string)
}

/// Göç sırasında eski kayıtların türetilmiş alanlarını doldurur.
pub fn backfill_derived(c: &Connection) -> rusqlite::Result<()> {
    let rows: Vec<(i64, String, String, String, i64, String)> = {
        let mut st = c.prepare("SELECT id, kind, title, outcome, reversible, detail FROM oplog")?;
        let r = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?;
        r.collect::<rusqlite::Result<_>>()?
    };
    let mut up = c.prepare(
        "UPDATE oplog SET target=?2, prev_state=?3, new_state=?4, confirmed=?5, restore_status=?6, error=?7 WHERE id=?1",
    )?;
    for (id, kind, title, outcome, reversible, detail) in rows {
        let d: Value = serde_json::from_str(&detail).unwrap_or(Value::Null);
        let x = derive(&kind, &title, &outcome, reversible != 0, &d);
        up.execute(params![
            id,
            x.target,
            opt_json(&x.prev_state),
            opt_json(&x.new_state),
            x.confirmed as i64,
            x.restore_status,
            x.error
        ])?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Yazma
// ---------------------------------------------------------------------------

pub fn record(db: &Db, kind: &str, title: &str, outcome: Outcome, reversible: bool, detail: Value) -> AppResult<i64> {
    tracing::info!(kind, title, outcome = outcome.as_str(), "işlem kaydedildi");
    let x = derive(kind, title, outcome.as_str(), reversible, &detail);
    db.with(|c| {
        c.execute(
            "INSERT INTO oplog(ts, kind, title, outcome, reversible, detail, target, prev_state, new_state, confirmed, restore_status, error)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                now_ts(),
                kind,
                title,
                outcome.as_str(),
                reversible as i64,
                detail.to_string(),
                x.target,
                opt_json(&x.prev_state),
                opt_json(&x.new_state),
                x.confirmed as i64,
                x.restore_status,
                x.error
            ],
        )?;
        Ok(c.last_insert_rowid())
    })
}

/// Geri alma sonucunu özgün kayda işler.
pub fn mark_restore(db: &Db, id: i64, status: &str, restore_op: Option<i64>, error: Option<&str>) -> AppResult<()> {
    db.with(|c| {
        c.execute(
            "UPDATE oplog SET restore_status=?2, restored_at=?3, restore_op=?4, error=COALESCE(?5, error) WHERE id=?1",
            params![id, status, now_ts(), restore_op, error],
        )
        .map(|_| ())
    })
}

/// Bir kaydın başlığını değiştirir ve ayrıntısına ek alanlar ekler.
pub fn annotate(db: &Db, id: i64, title: &str, extra: &[(&str, Value)]) -> AppResult<()> {
    let Some(e) = get(db, id)? else { return Ok(()) };
    let mut d = e.detail;
    if let Some(obj) = d.as_object_mut() {
        for (k, v) in extra {
            obj.insert((*k).to_string(), v.clone());
        }
    }
    db.with(|c| {
        c.execute("UPDATE oplog SET title=?2, detail=?3 WHERE id=?1", params![id, title, d.to_string()]).map(|_| ())
    })
}

// ---------------------------------------------------------------------------
// Okuma
// ---------------------------------------------------------------------------

const COLS: &str = "id, ts, kind, title, outcome, reversible, detail, target, prev_state, new_state, confirmed, restore_status, restored_at, restore_op, error";

fn parse_json(s: Option<String>) -> Option<Value> {
    s.and_then(|s| serde_json::from_str(&s).ok())
}

fn row(r: &rusqlite::Row) -> rusqlite::Result<OpEntry> {
    let detail: String = r.get(6)?;
    Ok(OpEntry {
        id: r.get(0)?,
        ts: r.get(1)?,
        kind: r.get(2)?,
        title: r.get(3)?,
        outcome: r.get(4)?,
        reversible: r.get::<_, i64>(5)? != 0,
        detail: serde_json::from_str(&detail).unwrap_or(Value::Null),
        target: r.get(7)?,
        previous_state: parse_json(r.get(8)?),
        new_state: parse_json(r.get(9)?),
        confirmed: r.get::<_, i64>(10)? != 0,
        restore_status: r.get(11)?,
        restored_at: r.get(12)?,
        restore_op: r.get(13)?,
        error: r.get(14)?,
    })
}

#[derive(Debug, Default, Clone)]
pub struct OpFilter {
    pub kind: Option<String>,
    pub query: Option<String>,
    pub outcome: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

/// LIKE desenindeki özel karakterleri `\` ile kaçırır.
pub fn like_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for ch in s.chars() {
        if matches!(ch, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Filtreli ve sayfalı liste; toplam eşleşen kayıt sayısıyla birlikte.
pub fn query(db: &Db, f: &OpFilter) -> AppResult<(Vec<OpEntry>, i64)> {
    let kind = f.kind.as_deref().filter(|s| !s.is_empty());
    let outcome = f.outcome.as_deref().filter(|s| !s.is_empty());
    let pattern = f.query.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(|q| format!("%{}%", like_escape(q)));
    let filter = "(?1 IS NULL OR kind = ?1) AND (?2 IS NULL OR outcome = ?2)
         AND (?3 IS NULL OR title LIKE ?3 ESCAPE '\\' OR COALESCE(target,'') LIKE ?3 ESCAPE '\\')";
    db.with(|c| {
        let total: i64 =
            c.query_row(&format!("SELECT COUNT(*) FROM oplog WHERE {filter}"), params![kind, outcome, pattern], |r| {
                r.get(0)
            })?;
        let mut st = c.prepare(&format!(
            "SELECT {COLS} FROM oplog WHERE {filter} ORDER BY ts DESC, id DESC LIMIT ?4 OFFSET ?5"
        ))?;
        let items = st
            .query_map(params![kind, outcome, pattern, f.limit, f.offset.max(0)], row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok((items, total))
    })
}

pub fn list(db: &Db, kind: Option<&str>, limit: i64) -> AppResult<Vec<OpEntry>> {
    let f = OpFilter { kind: kind.map(str::to_string), limit, ..Default::default() };
    Ok(query(db, &f)?.0)
}

pub fn get(db: &Db, id: i64) -> AppResult<Option<OpEntry>> {
    use rusqlite::OptionalExtension;
    db.with(|c| c.query_row(&format!("SELECT {COLS} FROM oplog WHERE id = ?1"), [id], row).optional())
}

pub fn prune(db: &Db, older_than_days: i64) -> AppResult<usize> {
    let cutoff = now_ts() - older_than_days * 86_400;
    db.with(|c| c.execute("DELETE FROM oplog WHERE ts < ?1", [cutoff]))
}

// ---------------------------------------------------------------------------
// Dışa aktarma yardımcıları
// ---------------------------------------------------------------------------

pub fn kind_label(kind: &str) -> &'static str {
    match kind {
        "cleanup" => "Temizlik",
        "files" => "Dosya işlemi",
        "folders" => "Boş klasörler",
        "quarantine" => "Karantina",
        "startup" => "Başlangıç öğesi",
        "service" => "Hizmet",
        "task" => "Zamanlanmış görev",
        "power" => "Güç planı",
        "network" => "Ağ",
        "process" => "Süreç",
        "update" => "Güncelleme",
        _ => "Diğer",
    }
}

pub fn outcome_label(outcome: &str) -> &'static str {
    match outcome {
        "success" => "Başarılı",
        "partial" => "Kısmen başarılı",
        "failed" => "Başarısız",
        "cancelled" => "İptal edildi",
        _ => "Bilinmiyor",
    }
}

pub fn restore_label(status: &str) -> &'static str {
    match status {
        "available" => "Geri alınabilir",
        "restored" => "Geri alındı",
        "failed" => "Geri alma başarısız",
        _ => "Geri alınamaz",
    }
}

/// Kayıtları `;` ayraçlı, UTF-8 BOM'lu CSV metnine dönüştürür.
pub fn to_csv(entries: &[OpEntry]) -> String {
    use crate::files::csv_field as f;
    let mut out = String::from("\u{feff}");
    out.push_str("Kimlik;Tarih;Tür;Başlık;Hedef;Sonuç;Onaylandı;Geri alma durumu;Geri alma tarihi;Hata;Önceki durum;Yeni durum;Ayrıntı\r\n");
    for e in entries {
        let cols = [
            e.id.to_string(),
            crate::files::fmt_local(e.ts),
            kind_label(&e.kind).to_string(),
            e.title.clone(),
            e.target.clone().unwrap_or_default(),
            outcome_label(&e.outcome).to_string(),
            if e.confirmed { "Evet" } else { "Hayır" }.to_string(),
            restore_label(&e.restore_status).to_string(),
            e.restored_at.map(crate::files::fmt_local).unwrap_or_default(),
            e.error.clone().unwrap_or_default(),
            e.previous_state.as_ref().map(Value::to_string).unwrap_or_default(),
            e.new_state.as_ref().map(Value::to_string).unwrap_or_default(),
            e.detail.to_string(),
        ];
        let line: Vec<String> = cols.iter().map(|c| f(c)).collect();
        out.push_str(&line.join(";"));
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn record_and_list() {
        let db = Db::open_in_memory().unwrap();
        record(&db, "cleanup", "Temizlik", Outcome::Success, false, json!({"n":1})).unwrap();
        record(&db, "startup", "Başlangıç", Outcome::Failed, true, json!({})).unwrap();
        assert_eq!(list(&db, None, 10).unwrap().len(), 2);
        let s = list(&db, Some("startup"), 10).unwrap();
        assert_eq!(s.len(), 1);
        assert!(s[0].reversible);
        assert_eq!(s[0].outcome, "failed");
    }

    #[test]
    fn derives_columns_and_restore_status() {
        let db = Db::open_in_memory().unwrap();
        let id = record(
            &db,
            "service",
            "Hizmet başlangıç türü değiştirildi",
            Outcome::Success,
            true,
            json!({"action":"start_type","name":"Spooler","displayName":"Yazdırma Biriktiricisi","previousStartType":"auto","previousStatus":"running","newStartType":"manual"}),
        )
        .unwrap();
        let e = get(&db, id).unwrap().unwrap();
        assert_eq!(e.target.as_deref(), Some("Yazdırma Biriktiricisi"));
        assert_eq!(e.previous_state, Some(json!({"previousStartType":"auto","previousStatus":"running"})));
        assert_eq!(e.new_state, Some(json!({"newStartType":"manual"})));
        assert!(e.confirmed);
        assert_eq!(e.restore_status, "available");

        // "boot" önceki türü geri yüklenemez.
        let id = record(
            &db,
            "service",
            "x",
            Outcome::Success,
            true,
            json!({"action":"start_type","name":"a","previousStartType":"boot"}),
        )
        .unwrap();
        assert_eq!(get(&db, id).unwrap().unwrap().restore_status, "none");
        // Hizmet durdurma/başlatma geri alınabilir sayılmaz.
        let id = record(
            &db,
            "service",
            "x",
            Outcome::Success,
            true,
            json!({"action":"stop","name":"a","previousStartType":"auto"}),
        )
        .unwrap();
        assert_eq!(get(&db, id).unwrap().unwrap().restore_status, "none");
        // Başarısız işlem geri alınamaz; hata alanı doldurulur.
        let id = record(
            &db,
            "startup",
            "x",
            Outcome::Failed,
            true,
            json!({"id":"a","previousEnabled":true,"error":"erişim yok"}),
        )
        .unwrap();
        let e = get(&db, id).unwrap().unwrap();
        assert_eq!(e.restore_status, "none");
        assert_eq!(e.error.as_deref(), Some("erişim yok"));
        // Görev hedefi yol + ad.
        let id = record(
            &db,
            "task",
            "x",
            Outcome::Success,
            true,
            json!({"path":"\\Ivily\\","name":"T","previousEnabled":false}),
        )
        .unwrap();
        let e = get(&db, id).unwrap().unwrap();
        assert_eq!(e.target.as_deref(), Some("\\Ivily\\T"));
        assert_eq!(e.restore_status, "available");
        // Karantina kimlikleri olmayan dosya işlemi geri alınamaz.
        let id = record(&db, "files", "x", Outcome::Success, true, json!({"mode":"quarantine","items":["C:\\a.txt"]}))
            .unwrap();
        let e = get(&db, id).unwrap().unwrap();
        assert_eq!(e.restore_status, "none");
        assert_eq!(e.target.as_deref(), Some("C:\\a.txt"));
        let id =
            record(&db, "files", "x", Outcome::Success, true, json!({"mode":"recycle","quarantineIds":["q"]})).unwrap();
        assert_eq!(get(&db, id).unwrap().unwrap().restore_status, "none");
        let id = record(&db, "files", "x", Outcome::Success, true, json!({"mode":"quarantine","quarantineIds":["q"]}))
            .unwrap();
        assert_eq!(get(&db, id).unwrap().unwrap().restore_status, "available");
        // Güncelleme onaylı sayılmaz.
        let id = record(&db, "update", "x", Outcome::Success, false, json!({})).unwrap();
        assert!(!get(&db, id).unwrap().unwrap().confirmed);
        let id =
            record(&db, "quarantine", "Karantinadan kalıcı silme: 2 öğe", Outcome::Success, false, json!({})).unwrap();
        assert!(get(&db, id).unwrap().unwrap().confirmed);
    }

    #[test]
    fn search_and_pagination() {
        let db = Db::open_in_memory().unwrap();
        for i in 0..25 {
            record(&db, "cleanup", &format!("Temizlik {i}"), Outcome::Success, false, json!({})).unwrap();
        }
        record(&db, "files", "100% indirim_dosyası", Outcome::Partial, true, json!({"items":["C:\\x\\rapor.pdf"]}))
            .unwrap();
        let f = |kind: Option<&str>, q: Option<&str>, o: Option<&str>, limit, offset| OpFilter {
            kind: kind.map(Into::into),
            query: q.map(Into::into),
            outcome: o.map(Into::into),
            limit,
            offset,
        };
        let (items, total) = query(&db, &f(None, None, None, 10, 0)).unwrap();
        assert_eq!((items.len(), total), (10, 26));
        let (items, total) = query(&db, &f(Some("cleanup"), None, None, 10, 20)).unwrap();
        assert_eq!((items.len(), total), (5, 25));
        // % ve _ birer joker değil, düz karakter olarak aranır.
        let (items, total) = query(&db, &f(None, Some("100%"), None, 10, 0)).unwrap();
        assert_eq!((items.len(), total), (1, 1));
        let (_, total) = query(&db, &f(None, Some("0%"), None, 10, 0)).unwrap();
        assert_eq!(total, 1);
        let (_, total) = query(&db, &f(None, Some("k_1"), None, 10, 0)).unwrap();
        assert_eq!(total, 0);
        // Hedefte arama.
        let (items, _) = query(&db, &f(None, Some("rapor.pdf"), None, 10, 0)).unwrap();
        assert_eq!(items.len(), 1);
        let (_, total) = query(&db, &f(None, None, Some("partial"), 10, 0)).unwrap();
        assert_eq!(total, 1);
    }

    #[test]
    fn csv_export_contents() {
        let db = Db::open_in_memory().unwrap();
        record(
            &db,
            "files",
            "2 dosya; \"karantina\"",
            Outcome::Success,
            true,
            json!({"mode":"quarantine","quarantineIds":["a"]}),
        )
        .unwrap();
        let csv = to_csv(&list(&db, None, 10).unwrap());
        assert!(csv.starts_with('\u{feff}'));
        let mut lines = csv.trim_start_matches('\u{feff}').lines();
        assert!(lines.next().unwrap().starts_with("Kimlik;Tarih;Tür;Başlık;Hedef;Sonuç"));
        let row = lines.next().unwrap();
        assert!(row.contains(";Dosya işlemi;\"2 dosya; \"\"karantina\"\"\";"));
        assert!(row.contains(";Başarılı;Evet;Geri alınabilir;"));
    }
}
