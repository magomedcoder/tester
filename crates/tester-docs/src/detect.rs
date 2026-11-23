use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    Text,
    Pdf,
    Office,
    Image,
}

impl DocKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Pdf => "pdf",
            Self::Office => "office",
            Self::Image => "image",
        }
    }
}

pub fn detect_kind(path: &Path) -> DocKind {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "pdf" => DocKind::Pdf,
        "doc" | "docx" | "odt" | "rtf" | "xls" | "xlsx" | "ods" | "ppt" | "pptx" | "odp"
        | "csv" => DocKind::Office,
        "png" | "jpg" | "jpeg" | "tif" | "tiff" | "webp" | "bmp" | "gif" => DocKind::Image,
        "md" | "mdx" | "txt" | "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "java"
        | "kt" | "c" | "h" | "cpp" | "hpp" | "cs" | "rb" | "php" | "swift" | "toml" | "yaml"
        | "yml" | "json" | "html" | "css" | "xml" | "sh" | "bash" | "zsh" | "sql" | "graphql"
        | "vue" | "svelte" | "zig" | "rs.in" | "txt.in" => DocKind::Text,
        _ => {
            DocKind::Text
        }
    }
}

pub fn prefers_outline(path: &Path) -> bool {
    matches!(detect_kind(path), DocKind::Text)
        && path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| {
                let e = e.to_ascii_lowercase();
                !matches!(e.as_str(), "md" | "mdx" | "txt" | "rst" | "adoc")
            })
            .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_pdf_office_image() {
        assert_eq!(detect_kind(Path::new("a.PDF")), DocKind::Pdf);
        assert_eq!(detect_kind(Path::new("x.docx")), DocKind::Office);
        assert_eq!(detect_kind(Path::new("y.png")), DocKind::Image);
        assert_eq!(detect_kind(Path::new("main.rs")), DocKind::Text);
    }

    #[test]
    fn outline_for_source_not_md() {
        assert!(prefers_outline(&PathBuf::from("src/lib.rs")));
        assert!(!prefers_outline(&PathBuf::from("notes.md")));
        assert!(!prefers_outline(&PathBuf::from("scan.pdf")));
    }
}
