use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub type Uncovered = HashMap<String, HashSet<usize>>;

pub fn load_coverage(path: &Path) -> Result<Uncovered> {
    let text = std::fs::read_to_string(path)?;
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("xml"))
        || text.contains("<coverage")
        || text.contains("<class ")
    {
        return parse_cobertura(&text);
    }
    parse_lcov(&text)
}

pub fn discover_coverage_file(root: &Path) -> Option<PathBuf> {
    const CANDIDATES: &[&str] = &[
        "lcov.info",
        "coverage/lcov.info",
        "coverage/lcov-report/../lcov.info",
        "target/llvm-cov/lcov.info",
        "coverage.xml",
        "cobertura.xml",
        "coverage/cobertura-coverage.xml",
    ];
    for c in CANDIDATES {
        let p = root.join(c);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn parse_lcov(text: &str) -> Result<Uncovered> {
    let mut out = Uncovered::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("SF:") {
            current = Some(normalize(rest.trim()));
        } else if let Some(rest) = line.strip_prefix("DA:") {
            let mut parts = rest.split(',');
            let ln: usize = parts.next().unwrap_or("0").parse().unwrap_or(0);
            let hits: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
            if hits == 0 && ln > 0 {
                if let Some(file) = &current {
                    out.entry(file.clone()).or_default().insert(ln);
                }
            }
        } else if line.trim() == "end_of_record" {
            current = None;
        }
    }
    Ok(out)
}

fn parse_cobertura(text: &str) -> Result<Uncovered> {
    let mut out = Uncovered::new();
    let file_re = regex::Regex::new(r#"filename="([^"]+)""#)?;
    let line_re = regex::Regex::new(r#"<line[^>]*number="(\d+)"[^>]*hits="(\d+)""#)?;
    let mut current = String::new();
    for line in text.lines() {
        if let Some(c) = file_re.captures(line) {
            current = normalize(c.get(1).unwrap().as_str());
        }
        for c in line_re.captures_iter(line) {
            let ln: usize = c[1].parse().unwrap_or(0);
            let hits: u64 = c[2].parse().unwrap_or(0);
            if hits == 0 && ln > 0 && !current.is_empty() {
                out.entry(current.clone()).or_default().insert(ln);
            }
        }
    }
    Ok(out)
}

fn normalize(p: &str) -> String {
    p.replace('\\', "/")
}

pub fn symbol_uncovered(uncovered: &Uncovered, file: &Path, start: usize, end: usize) -> usize {
    let key = normalize(&file.to_string_lossy());
    let Some(lines) = uncovered.get(&key).or_else(|| {
        uncovered
            .iter()
            .find(|(k, _)| k.ends_with(&key) || key.ends_with(k.as_str()))
            .map(|(_, v)| v)
    }) else {
        return 0;
    };
    (start..=end).filter(|l| lines.contains(l)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lcov() {
        let sample = "SF:src/lib.rs\nDA:10,0\nDA:11,1\nend_of_record\n";
        let u = parse_lcov(sample).unwrap();
        assert!(u.get("src/lib.rs").unwrap().contains(&10));
        assert!(!u.get("src/lib.rs").unwrap().contains(&11));
    }
}
