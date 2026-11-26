//! Authenticode dijital imza doğrulaması.
//!
//! `WinVerifyTrust` ile çevrimdışı (yalnızca önbellek; ağ erişimi yok,
//! iptal denetimi yok) doğrulama yapılır. Gömülü imzası olmayan dosyalar için
//! Windows katalog imzaları (`CryptCATAdmin*`) da denetlenir; Windows
//! bileşenlerinin çoğu bu şekilde imzalıdır.
//!
//! Sonuçlar dosya yolu, değişiklik zamanı ve boyutuna göre bellekte
//! önbelleğe alınır.

use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureStatus {
    /// "signed" | "unsigned" | "invalid" | "unknown"
    pub state: String,
    pub state_label: String,
    pub signer: Option<String>,
}

impl SignatureStatus {
    pub fn new(state: &str, signer: Option<String>) -> Self {
        let label = match state {
            "signed" => "İmzalı ve geçerli",
            "unsigned" => "İmzasız",
            "invalid" => "İmza geçersiz",
            _ => "Doğrulanamadı",
        };
        Self { state: state.to_string(), state_label: label.to_string(), signer }
    }

    pub fn unknown() -> Self {
        Self::new("unknown", None)
    }

    pub fn catalog_unchecked() -> Self {
        Self { state: "unknown".into(), state_label: "Katalog imzası denetlenmedi".into(), signer: None }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureResult {
    pub path: String,
    pub status: SignatureStatus,
}

// WinVerifyTrust dönüş kodları.
const TRUST_E_NOSIGNATURE: u32 = 0x800B_0100;
const TRUST_E_SUBJECT_FORM_UNKNOWN: u32 = 0x800B_0003;
const TRUST_E_PROVIDER_UNKNOWN: u32 = 0x800B_0001;
const TRUST_E_BAD_DIGEST: u32 = 0x8009_6010;
const TRUST_E_CERT_SIGNATURE: u32 = 0x8009_6004;
const TRUST_E_EXPLICIT_DISTRUST: u32 = 0x800B_0111;
const CERT_E_UNTRUSTEDROOT: u32 = 0x800B_0109;
const CERT_E_REVOKED: u32 = 0x800B_010C;
const CERT_E_EXPIRED: u32 = 0x800B_0101;
const CERT_E_UNTRUSTEDTESTROOT: u32 = 0x800B_010D;
const CERT_E_WRONG_USAGE: u32 = 0x800B_0110;
const TRUST_E_TIME_STAMP: u32 = 0x8009_6005;

/// Ham `WinVerifyTrust` sonucunu duruma çevirir.
/// `None` → gömülü imza yok (katalog denetimi yapılabilir).
pub fn classify(code: u32) -> Option<&'static str> {
    match code {
        0 => Some("signed"),
        TRUST_E_NOSIGNATURE | TRUST_E_SUBJECT_FORM_UNKNOWN | TRUST_E_PROVIDER_UNKNOWN => None,
        TRUST_E_BAD_DIGEST
        | TRUST_E_CERT_SIGNATURE
        | TRUST_E_EXPLICIT_DISTRUST
        | CERT_E_UNTRUSTEDROOT
        | CERT_E_REVOKED
        | CERT_E_EXPIRED
        | CERT_E_UNTRUSTEDTESTROOT
        | CERT_E_WRONG_USAGE
        | TRUST_E_TIME_STAMP => Some("invalid"),
        // Zincir çevrimdışı kurulamadı, ilke engeli vb.: dürüstçe "doğrulanamadı".
        _ => Some("unknown"),
    }
}

type CacheKey = (String, u64, u64);

fn cache() -> &'static Mutex<HashMap<CacheKey, SignatureStatus>> {
    static C: OnceLock<Mutex<HashMap<CacheKey, SignatureStatus>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

fn under_system_root(path: &Path) -> bool {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    crate::paths::is_within(&path.to_string_lossy(), &root)
}

/// Dosyanın dijital imzasını doğrular (ağ erişimi olmadan).
pub fn verify(path: &Path) -> SignatureStatus {
    let Ok(meta) = std::fs::metadata(path) else { return SignatureStatus::unknown() };
    if !meta.is_file() {
        return SignatureStatus::unknown();
    }
    let mtime = meta
        .modified()
        .ok()
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let key = (crate::paths::normalize(&path.to_string_lossy()), mtime, meta.len());
    if let Some(s) = cache().lock().get(&key) {
        return s.clone();
    }
    let status = imp::verify_uncached(path);
    let mut c = cache().lock();
    if c.len() > 4096 {
        c.clear();
    }
    c.insert(key, status.clone());
    status
}

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::sysmgmt::wide;
    use std::os::windows::io::AsRawHandle;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HANDLE, HWND};
    use windows::Win32::Security::Cryptography::Catalog::*;
    use windows::Win32::Security::Cryptography::*;
    use windows::Win32::Security::WinTrust::*;

    pub fn verify_uncached(path: &Path) -> SignatureStatus {
        let wpath = wide(&path.to_string_lossy());
        let code = verify_embedded(&wpath);
        match classify(code) {
            Some(state) => {
                let signer = if state == "unknown" { None } else { signer_name(&wpath) };
                SignatureStatus::new(state, signer)
            }
            None => match verify_catalog(path, &wpath) {
                CatalogOutcome::Verified(code, cat) => {
                    let state = classify(code).unwrap_or("unknown");
                    let signer = if state == "unknown" { None } else { signer_name(&wide(&cat)) };
                    SignatureStatus::new(state, signer)
                }
                CatalogOutcome::NotFound => SignatureStatus::new("unsigned", None),
                CatalogOutcome::Error => {
                    if under_system_root(path) {
                        SignatureStatus::catalog_unchecked()
                    } else {
                        SignatureStatus::new("unsigned", None)
                    }
                }
            },
        }
    }

    fn verify_embedded(wpath: &[u16]) -> u32 {
        let mut file = WINTRUST_FILE_INFO {
            cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
            pcwszFilePath: PCWSTR(wpath.as_ptr()),
            ..Default::default()
        };
        let mut data = WINTRUST_DATA {
            cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
            dwUIChoice: WTD_UI_NONE,
            fdwRevocationChecks: WTD_REVOKE_NONE,
            dwUnionChoice: WTD_CHOICE_FILE,
            dwStateAction: WTD_STATEACTION_VERIFY,
            dwProvFlags: WTD_CACHE_ONLY_URL_RETRIEVAL,
            ..Default::default()
        };
        data.Anonymous.pFile = &mut file;
        run_trust(&mut data)
    }

    fn run_trust(data: &mut WINTRUST_DATA) -> u32 {
        let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        unsafe {
            let r = WinVerifyTrust(HWND::default(), &mut action, data as *mut _ as *mut _);
            data.dwStateAction = WTD_STATEACTION_CLOSE;
            let _ = WinVerifyTrust(HWND::default(), &mut action, data as *mut _ as *mut _);
            r as u32
        }
    }

    enum CatalogOutcome {
        Verified(u32, String),
        NotFound,
        Error,
    }

    fn verify_catalog(path: &Path, wpath: &[u16]) -> CatalogOutcome {
        let Ok(file) = std::fs::File::open(path) else { return CatalogOutcome::Error };
        let handle = HANDLE(file.as_raw_handle());
        let mut any_ok = false;
        // Önce SHA-256, ardından eski kataloglar için SHA-1.
        for algo in [Some("SHA256"), None] {
            match try_catalog(handle, wpath, algo) {
                Some(Some(v)) => return v,
                Some(None) => any_ok = true,
                None => {}
            }
        }
        if any_ok {
            CatalogOutcome::NotFound
        } else {
            CatalogOutcome::Error
        }
    }

    /// `None` → API hatası; `Some(None)` → katalogda bulunamadı.
    fn try_catalog(file: HANDLE, wpath: &[u16], algo: Option<&str>) -> Option<Option<CatalogOutcome>> {
        unsafe {
            let mut admin: isize = 0;
            let walgo = algo.map(wide);
            let algo_ptr = walgo.as_ref().map(|w| PCWSTR(w.as_ptr())).unwrap_or(PCWSTR::null());
            CryptCATAdminAcquireContext2(&mut admin, None, algo_ptr, None, None).ok()?;
            struct Admin(isize);
            impl Drop for Admin {
                fn drop(&mut self) {
                    unsafe {
                        let _ = CryptCATAdminReleaseContext(self.0, 0);
                    }
                }
            }
            let admin = Admin(admin);
            let mut size = 0u32;
            let _ = CryptCATAdminCalcHashFromFileHandle2(admin.0, file, &mut size, None, None);
            if size == 0 || size > 256 {
                return None;
            }
            let mut hash = vec![0u8; size as usize];
            CryptCATAdminCalcHashFromFileHandle2(admin.0, file, &mut size, Some(hash.as_mut_ptr()), None).ok()?;
            hash.truncate(size as usize);
            let cat = CryptCATAdminEnumCatalogFromHash(admin.0, &hash, None, None);
            if cat == 0 {
                return Some(None);
            }
            let mut info = CATALOG_INFO { cbStruct: std::mem::size_of::<CATALOG_INFO>() as u32, ..Default::default() };
            let got = CryptCATCatalogInfoFromContext(cat, &mut info, 0).is_ok();
            let outcome = if got {
                let end = info.wszCatalogFile.iter().position(|c| *c == 0).unwrap_or(info.wszCatalogFile.len());
                let cat_path = String::from_utf16_lossy(&info.wszCatalogFile[..end]);
                let tag: String = hash.iter().map(|b| format!("{b:02X}")).collect();
                let wtag = wide(&tag);
                let wcat = wide(&cat_path);
                let mut ci = WINTRUST_CATALOG_INFO {
                    cbStruct: std::mem::size_of::<WINTRUST_CATALOG_INFO>() as u32,
                    pcwszCatalogFilePath: PCWSTR(wcat.as_ptr()),
                    pcwszMemberTag: PCWSTR(wtag.as_ptr()),
                    pcwszMemberFilePath: PCWSTR(wpath.as_ptr()),
                    hMemberFile: file,
                    pbCalculatedFileHash: hash.as_mut_ptr(),
                    cbCalculatedFileHash: hash.len() as u32,
                    hCatAdmin: admin.0,
                    ..Default::default()
                };
                let mut data = WINTRUST_DATA {
                    cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
                    dwUIChoice: WTD_UI_NONE,
                    fdwRevocationChecks: WTD_REVOKE_NONE,
                    dwUnionChoice: WTD_CHOICE_CATALOG,
                    dwStateAction: WTD_STATEACTION_VERIFY,
                    dwProvFlags: WTD_CACHE_ONLY_URL_RETRIEVAL,
                    ..Default::default()
                };
                data.Anonymous.pCatalog = &mut ci;
                let code = run_trust(&mut data);
                Some(Some(CatalogOutcome::Verified(code, cat_path)))
            } else {
                None
            };
            let _ = CryptCATAdminReleaseCatalogContext(admin.0, cat, 0);
            outcome
        }
    }

    /// İmzalayanın görünen adı (gömülü imza veya katalog dosyası için).
    fn signer_name(wpath: &[u16]) -> Option<String> {
        unsafe {
            let mut store = HCERTSTORE::default();
            let mut msg: *mut core::ffi::c_void = std::ptr::null_mut();
            CryptQueryObject(
                CERT_QUERY_OBJECT_FILE,
                wpath.as_ptr() as *const _,
                // Katalog dosyaları da PKCS#7 imzalı ileti olarak açılır.
                CERT_QUERY_CONTENT_TYPE_FLAGS(
                    CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED.0 | CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED.0,
                ),
                CERT_QUERY_FORMAT_FLAG_BINARY,
                0,
                None,
                None,
                None,
                Some(&mut store),
                Some(&mut msg),
                None,
            )
            .ok()?;
            let result = (|| {
                if msg.is_null() || store.is_invalid() {
                    return None;
                }
                let mut size = 0u32;
                CryptMsgGetParam(msg, CMSG_SIGNER_INFO_PARAM, 0, None, &mut size).ok()?;
                if size == 0 {
                    return None;
                }
                let mut buf = vec![0u64; (size as usize).div_ceil(8)];
                CryptMsgGetParam(msg, CMSG_SIGNER_INFO_PARAM, 0, Some(buf.as_mut_ptr() as *mut _), &mut size).ok()?;
                let si = &*(buf.as_ptr() as *const CMSG_SIGNER_INFO);
                let ci = CERT_INFO { Issuer: si.Issuer, SerialNumber: si.SerialNumber, ..Default::default() };
                let cert = CertFindCertificateInStore(
                    store,
                    X509_ASN_ENCODING | PKCS_7_ASN_ENCODING,
                    0,
                    CERT_FIND_SUBJECT_CERT,
                    Some(&ci as *const _ as *const _),
                    None,
                );
                if cert.is_null() {
                    return None;
                }
                let n = CertGetNameStringW(cert, CERT_NAME_SIMPLE_DISPLAY_TYPE, 0, None, None);
                let name = if n > 1 {
                    let mut w = vec![0u16; n as usize];
                    let n2 = CertGetNameStringW(cert, CERT_NAME_SIMPLE_DISPLAY_TYPE, 0, None, Some(&mut w));
                    let s = String::from_utf16_lossy(&w[..(n2.saturating_sub(1)) as usize]).trim().to_string();
                    (!s.is_empty()).then_some(s)
                } else {
                    None
                };
                let _ = CertFreeCertificateContext(Some(cert));
                name
            })();
            if !msg.is_null() {
                let _ = CryptMsgClose(Some(msg));
            }
            if !store.is_invalid() {
                let _ = CertCloseStore(Some(store), 0);
            }
            result
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;
    pub fn verify_uncached(_p: &Path) -> SignatureStatus {
        SignatureStatus::unknown()
    }
}

/// Komut için yol listesini doğrular ve imzaları denetler.
pub fn check_many(paths: Vec<String>) -> crate::error::AppResult<Vec<SignatureResult>> {
    use crate::error::AppError;
    if paths.len() > 200 {
        return Err(AppError::InvalidInput("tek seferde en fazla 200 dosya denetlenebilir".into()));
    }
    for p in &paths {
        crate::paths::check_local(p)?;
    }
    Ok(paths
        .into_iter()
        .map(|p| {
            let status = verify(Path::new(&p));
            SignatureResult { path: p, status }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification() {
        assert_eq!(classify(0), Some("signed"));
        assert_eq!(classify(TRUST_E_NOSIGNATURE), None);
        assert_eq!(classify(TRUST_E_BAD_DIGEST), Some("invalid"));
        assert_eq!(classify(0x800B_010A), Some("unknown"));
        assert_eq!(SignatureStatus::new("unsigned", None).state_label, "İmzasız");
    }

    #[cfg(windows)]
    #[test]
    fn verify_real_files() {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        let explorer = Path::new(&root).join("explorer.exe");
        let s = verify(&explorer);
        println!("explorer.exe → {s:?}");
        assert_ne!(s.state, "invalid");
        assert_ne!(s.state, "unsigned");
        // Önbellekten aynı sonuç.
        assert_eq!(verify(&explorer), s);

        let notepad = Path::new(&root).join("System32\\notepad.exe");
        if notepad.exists() {
            println!("notepad.exe → {:?}", verify(&notepad));
        }
        let kernel32 = Path::new(&root).join("System32\\kernel32.dll");
        let k = verify(&kernel32);
        println!("kernel32.dll → {k:?}");
        assert_ne!(k.state, "invalid");

        let t = tempfile::tempdir().unwrap();
        let f = t.path().join("ornek.exe");
        std::fs::write(&f, b"MZ bu bir deneme dosyasidir").unwrap();
        let u = verify(&f);
        println!("temp → {u:?}");
        assert_eq!(u.state, "unsigned");

        let missing = t.path().join("yok.exe");
        assert_eq!(verify(&missing).state, "unknown");

        assert!(check_many(vec!["göreli\\yol.exe".into()]).is_err());
        assert!(check_many(vec!["x".into(); 201]).is_err());
        let r = check_many(vec![explorer.to_string_lossy().into_owned()]).unwrap();
        assert_eq!(r.len(), 1);
    }
}
