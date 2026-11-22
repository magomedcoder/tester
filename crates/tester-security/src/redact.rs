pub fn redact_secret(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 8 {
        return "*".repeat(chars.len().max(4));
    }
    let head: String = chars.iter().take(4).collect();
    let tail: String = chars.iter().rev().take(2).rev().collect();
    format!("{head}...{tail} (redacted, {} chars)", chars.len())
}

pub fn redact_snippet(line: &str) -> String {
    let mut out = line.to_string();
    let pat = r#"(?i)((?:api[_-]?key|token|secret|password|authorization)\s*[:=]\s*)(['"]?)([^'"\s]+)(['"]?)"#;
    if let Ok(re) = regex::Regex::new(pat) {
        out = re
            .replace_all(&out, |caps: &regex::Captures| {
                format!(
                    "{}{}{}{}",
                    &caps[1],
                    &caps[2],
                    redact_secret(&caps[3]),
                    &caps[4]
                )
            })
            .into_owned();
    }
    if let Ok(re) = regex::Regex::new(r"AKIA[0-9A-Z]{16}") {
        out = re.replace_all(&out, "AKIA********").into_owned();
    }
    if let Ok(re) = regex::Regex::new(r"ghp_[A-Za-z0-9]{20,}") {
        out = re.replace_all(&out, "ghp_********").into_owned();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_key_assignment() {
        let s = redact_snippet(r#"api_key = "sk-secret-value-here""#);
        assert!(!s.contains("sk-secret-value-here"));
        assert!(s.contains("redacted") || s.contains('…'));
    }
}
