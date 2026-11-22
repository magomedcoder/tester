use anyhow::Result;
use std::time::Duration;

pub fn wiki_client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("tester-wiki/0.1")
        .build()?)
}

pub fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            b'|' => out.push_str("%7C"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
