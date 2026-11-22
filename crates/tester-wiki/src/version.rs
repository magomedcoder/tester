use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub fn infer_version_from_title(title: &str) -> Option<String> {
    let re = version_re();
    if let Some(c) = re.captures(title) {
        return Some(normalize_version(c.get(1)?.as_str()));
    }
    None
}

pub fn infer_version_from_path(path: &Path) -> Option<String> {
    for comp in path.components() {
        let s = comp.as_os_str().to_string_lossy();
        if let Some(v) = infer_version_from_title(&s) {
            return Some(v);
        }
        let lower = s.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix('v')
            && looks_like_version(rest)
        {
            return Some(normalize_version(rest));
        }
    }
    None
}

pub fn infer_version_from_url(url: &str) -> Option<String> {
    infer_version_from_title(url)
}

pub fn version_or_default(v: Option<String>) -> String {
    v.filter(|s| !s.is_empty()).unwrap_or_else(|| "default".into())
}

fn version_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)(?:^|[/\s_-])v?(\d+\.\d+(?:\.\d+)?)(?:$|[/\s_-])").expect("regex")
    })
}

fn looks_like_version(s: &str) -> bool {
    let mut parts = s.split('.');
    let a = parts.next().unwrap_or("");
    let b = parts.next().unwrap_or("");
    !a.is_empty()
        && a.chars().all(|c| c.is_ascii_digit())
        && !b.is_empty()
        && b.chars().all(|c| c.is_ascii_digit())
}

fn normalize_version(s: &str) -> String {
    s.trim().trim_start_matches('v').trim_start_matches('V').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_path_version() {
        let p = PathBuf::from("docs/wiki/v1.2/login.md");
        assert_eq!(infer_version_from_path(&p).as_deref(), Some("1.2"));
    }

    #[test]
    fn detects_title_version() {
        assert_eq!(
            infer_version_from_title("Version 2.0 / Login").as_deref(),
            Some("2.0")
        );
    }
}
