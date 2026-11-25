//! İşlem geçmişi: temizlik, karantina, başlangıç/servis değişiklikleri vb.

use crate::db::{now_ts, Db};
use crate::error::AppResult;
use rusqlite::params;
use serde::Serialize;

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

#[derive(Debug, Serialize)]
pub struct OpEntry {
    pub id: i64,
    pub ts: i64,
    pub kind: String,
    pub title: String,
    pub outcome: String,
    pub reversible: bool,
    pub detail: serde_json::Value,
}

pub fn record(
    db: &Db,
    kind: &str,
    title: &str,
    outcome: Outcome,
    reversible: bool,
    detail: serde_json::Value,
) -> AppResult<i64> {
    tracing::info!(kind, title, outcome = outcome.as_str(), "işlem kaydedildi");
    db.with(|c| {
        c.execute(
            "INSERT INTO oplog(ts, kind, title, outcome, reversible, detail) VALUES(?1,?2,?3,?4,?5,?6)",
            params![now_ts(), kind, title, outcome.as_str(), reversible as i64, detail.to_string()],
        )?;
        Ok(c.last_insert_rowid())
    })
}

pub fn list(db: &Db, kind: Option<&str>, limit: i64) -> AppResult<Vec<OpEntry>> {
    db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT id, ts, kind, title, outcome, reversible, detail FROM oplog
             WHERE (?1 IS NULL OR kind = ?1) ORDER BY ts DESC, id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![kind, limit], |r| {
            let detail: String = r.get(6)?;
            Ok(OpEntry {
                id: r.get(0)?,
                ts: r.get(1)?,
                kind: r.get(2)?,
                title: r.get(3)?,
                outcome: r.get(4)?,
                reversible: r.get::<_, i64>(5)? != 0,
                detail: serde_json::from_str(&detail).unwrap_or(serde_json::Value::Null),
            })
        })?;
        rows.collect()
    })
}

pub fn get(db: &Db, id: i64) -> AppResult<Option<OpEntry>> {
    use rusqlite::OptionalExtension;
    db.with(|c| {
        c.query_row(
            "SELECT id, ts, kind, title, outcome, reversible, detail FROM oplog WHERE id = ?1",
            [id],
            |r| {
                let detail: String = r.get(6)?;
                Ok(OpEntry {
                    id: r.get(0)?,
                    ts: r.get(1)?,
                    kind: r.get(2)?,
                    title: r.get(3)?,
                    outcome: r.get(4)?,
                    reversible: r.get::<_, i64>(5)? != 0,
                    detail: serde_json::from_str(&detail).unwrap_or(serde_json::Value::Null),
                })
            },
        )
        .optional()
    })
}

pub fn prune(db: &Db, older_than_days: i64) -> AppResult<usize> {
    let cutoff = now_ts() - older_than_days * 86_400;
    db.with(|c| c.execute("DELETE FROM oplog WHERE ts < ?1", [cutoff]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn record_and_list() {
        let db = Db::open_in_memory().unwrap();
        record(&db, "cleanup", "Temizlik", Outcome::Success, false, serde_json::json!({"n":1})).unwrap();
        record(&db, "startup", "Başlangıç", Outcome::Failed, true, serde_json::json!({})).unwrap();
        assert_eq!(list(&db, None, 10).unwrap().len(), 2);
        let s = list(&db, Some("startup"), 10).unwrap();
        assert_eq!(s.len(), 1);
        assert!(s[0].reversible);
        assert_eq!(s[0].outcome, "failed");
    }
}
