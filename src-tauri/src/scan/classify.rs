//! Uzantıya göre dosya kategorisi. Uzantı her zaman gerçek dosya türünü
//! göstermez; bu sınıflandırma yalnızca yaklaşık bir dağılım içindir.

pub const CATEGORIES: &[(&str, &str)] = &[
    ("image", "Görseller"),
    ("video", "Videolar"),
    ("audio", "Ses"),
    ("document", "Belgeler"),
    ("archive", "Arşivler ve disk görüntüleri"),
    ("executable", "Programlar ve kitaplıklar"),
    ("code", "Geliştirme dosyaları"),
    ("temp", "Geçici dosyalar ve günlükler"),
    ("other", "Diğer"),
];

pub fn category_of_ext(ext: &str) -> &'static str {
    match ext {
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "tif" | "tiff" | "heic" | "heif" | "svg" | "ico"
        | "raw" | "cr2" | "nef" | "arw" | "dng" | "psd" | "avif" => "image",
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg" | "ts" | "m2ts" | "3gp" => {
            "video"
        }
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus" | "aiff" | "mid" | "midi" => "audio",
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp" | "rtf" | "txt" | "md"
        | "csv" | "epub" | "pages" | "numbers" | "key" | "one" => "document",
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "zst" | "cab" | "iso" | "img" | "vhd" | "vhdx" | "vmdk"
        | "wim" | "esd" => "archive",
        "exe" | "dll" | "msi" | "msix" | "appx" | "sys" | "com" | "scr" | "ocx" | "drv" | "efi" | "jar" | "apk" => {
            "executable"
        }
        "rs" | "c" | "h" | "cpp" | "hpp" | "cc" | "cs" | "java" | "kt" | "py" | "pyc" | "js" | "mjs" | "cjs"
        | "tsx" | "jsx" | "go" | "rb" | "php" | "swift" | "lua" | "json" | "toml" | "yaml" | "yml"
        | "xml" | "html" | "css" | "scss" | "vue" | "svelte" | "sql" | "sh" | "ps1" | "bat" | "cmd" | "obj"
        | "o" | "a" | "lib" | "pdb" | "ilk" | "rlib" | "rmeta" | "d" | "class" | "map" | "wasm" | "node"
        | "lock" | "gradle" | "sln" | "csproj" | "vcxproj" | "ipynb" => "code",
        "tmp" | "temp" | "log" | "etl" | "dmp" | "old" | "bak" | "chk" | "~" => "temp",
        _ => "other",
    }
}

/// Boyutu da dikkate alan sınıflandırma. `.ts` uzantısı hem TypeScript kaynak
/// dosyası hem MPEG-TS videosu olabilir; büyük olanları video sayarız.
pub fn category_of(ext: &str, size: u64) -> &'static str {
    if ext == "ts" {
        return if size > 1024 * 1024 { "video" } else { "code" };
    }
    category_of_ext(ext)
}

pub fn category_label(id: &str) -> &'static str {
    CATEGORIES.iter().find(|(k, _)| *k == id).map(|(_, v)| *v).unwrap_or("Diğer")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classifies() {
        assert_eq!(category_of_ext("jpg"), "image");
        assert_eq!(category_of_ext("mkv"), "video");
        assert_eq!(category_of_ext("dll"), "executable");
        assert_eq!(category_of_ext("rs"), "code");
        assert_eq!(category_of_ext("tmp"), "temp");
        assert_eq!(category_of_ext(""), "other");
        assert_eq!(category_of_ext("xyz123"), "other");
        assert_eq!(category_label("image"), "Görseller");
    }
}
