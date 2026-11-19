use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StackFrame {
    pub file: String,
    pub line: Option<usize>,
    pub symbol: Option<String>,
}

pub fn parse_stacktrace(text: &str) -> Vec<StackFrame> {
    let mut frames = Vec::new();
    let patterns: Vec<Regex> = [
        // Rust
        r"(?m)(?:at\s+)?(?P<file>[^\s:]+?\.(?:rs|go|py|js|jsx|ts|tsx|kt|kts|java|php|dart|cpp|cc|c|h|hpp)):(?P<line>\d+)(?::\d+)?",
        // Python
        r#"(?m)File\s+\"(?P<file>[^\"]+\.py)\",\s+line\s+(?P<line>\d+)(?:,\s+in\s+(?P<symbol>\S+))?"#,
        // JS
        r"(?m)at\s+(?:(?P<symbol>[\w.]+)\s+)?\(?(?P<file>[^()\s]+\.(?:js|jsx|ts|tsx)):(?P<line>\d+):\d+\)?",
        // JVM
        r"(?m)at\s+(?P<symbol>[\w.$]+)\((?P<file>[\w./]+\.(?:kt|kts|java)):(?P<line>\d+)\)",
        // PHP
        r"(?m)(?P<file>[^\s()]+\.php)\((?P<line>\d+)\)|(?P<file2>[^\s]+\.php):(?P<line2>\d+)",
        // Go
        r"(?m)(?P<file>[^\s]+\.go):(?P<line>\d+)",
        // Dart
        r"(?m)(?P<file>(?:package:)?[^\s]+\.dart):(?P<line>\d+)(?::\d+)?",
    ]
    .into_iter()
    .filter_map(|p| Regex::new(p).ok())
    .collect();

    let sym_re = Regex::new(r"(?m)^\s*\d+:\s+(?P<symbol>[\w:]+(?:::[\w]+)+)").ok();

    for line in text.lines() {
        for re in &patterns {
            if let Some(c) = re.captures(line) {
                let file = c
                    .name("file")
                    .or_else(|| c.name("file2"))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                if file.is_empty() {
                    continue;
                }
                let line_no = c
                    .name("line")
                    .or_else(|| c.name("line2"))
                    .and_then(|m| m.as_str().parse().ok());
                let symbol = c.name("symbol").map(|m| m.as_str().to_string());
                let frame = StackFrame {
                    file: normalize_file(&file),
                    line: line_no,
                    symbol: symbol.or_else(|| extract_symbol_nearby(line)),
                };
                if !frames.iter().any(|f| f == &frame) {
                    frames.push(frame);
                }
            }
        }
        if let Some(re) = &sym_re {
            if let Some(c) = re.captures(line) {
                let sym = c.name("symbol").unwrap().as_str();
                let short = sym.rsplit("::").next().unwrap_or(sym).to_string();
                if !frames
                    .iter()
                    .any(|f| f.symbol.as_deref() == Some(short.as_str()))
                {
                    frames.push(StackFrame {
                        file: String::new(),
                        line: None,
                        symbol: Some(short),
                    });
                }
            }
        }
    }
    frames
}

fn normalize_file(file: &str) -> String {
    file.trim_start_matches("./")
        .trim_start_matches("package:")
        .replace('\\', "/")
}

fn extract_symbol_nearby(line: &str) -> Option<String> {
    let re = Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]{2,})\b").ok()?;
    re.find_iter(line)
        .map(|m| m.as_str().to_string())
        .find(|s| {
            !matches!(
                s.as_str(),
                "at" | "in" | "File" | "line" | "thread" | "panic" | "Error" | "Exception"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_traceback() {
        let t = r#"
Traceback (most recent call last):
  File "/app/src/mathutil.py", line 12, in multiply
    return a * b
"#;
        let f = parse_stacktrace(t);
        assert!(f.iter().any(|x| x.file.ends_with("mathutil.py")));
        assert!(f.iter().any(|x| x.symbol.as_deref() == Some("multiply")));
    }

    #[test]
    fn js_traceback() {
        let t = "at multiply (/proj/src/math.ts:10:5)";
        let f = parse_stacktrace(t);
        assert!(f.iter().any(|x| x.file.ends_with("math.ts")));
    }
}
