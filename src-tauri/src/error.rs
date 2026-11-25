//! Uygulama genelinde kullanılan hata türü.
//!
//! Ön yüze yalnızca Türkçe, kullanıcıya uygun bir mesaj ve makine tarafından
//! okunabilir bir kod gönderilir. Ayrıntılı teknik bilgi yerel günlüğe yazılır.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Bu işlem için erişim izni yok: {0}")]
    PermissionDenied(String),
    #[error("Öğe bulunamadı: {0}")]
    NotFound(String),
    #[error("Geçersiz giriş: {0}")]
    InvalidInput(String),
    #[error("İşlem güvenlik nedeniyle engellendi: {0}")]
    Protected(String),
    #[error("Çakışma: {0}")]
    Conflict(String),
    #[error("İşlem iptal edildi.")]
    Cancelled,
    #[error("Başka bir işlem zaten çalışıyor: {0}")]
    Busy(String),
    #[error("Ağ hatası: {0}")]
    Network(String),
    #[error("Yerel veri hatası: {0}")]
    Storage(String),
    #[error("Sistem hatası: {0}")]
    System(String),
    #[error("Bu özellik bu sistemde desteklenmiyor: {0}")]
    Unsupported(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::PermissionDenied(_) => "permission_denied",
            AppError::NotFound(_) => "not_found",
            AppError::InvalidInput(_) => "invalid_input",
            AppError::Protected(_) => "protected",
            AppError::Conflict(_) => "conflict",
            AppError::Cancelled => "cancelled",
            AppError::Busy(_) => "busy",
            AppError::Network(_) => "network",
            AppError::Storage(_) => "storage",
            AppError::System(_) => "system",
            AppError::Unsupported(_) => "unsupported",
        }
    }

    /// `std::io::Error` değerini bağlam bilgisiyle birlikte dönüştürür.
    pub fn io(err: std::io::Error, context: &str) -> Self {
        use std::io::ErrorKind::*;
        match err.kind() {
            PermissionDenied => AppError::PermissionDenied(context.to_string()),
            NotFound => AppError::NotFound(context.to_string()),
            _ => match err.raw_os_error() {
                Some(32) | Some(33) => {
                    AppError::System(format!("{context} (dosya başka bir uygulama tarafından kullanılıyor)"))
                }
                _ => AppError::System(format!("{context}: {err}")),
            },
        }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        tracing::error!(error = %e, "veritabanı hatası");
        AppError::Storage("yerel veritabanına erişilemedi".into())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        tracing::error!(error = %e, "JSON hatası");
        AppError::Storage("yerel veri okunamadı".into())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;
