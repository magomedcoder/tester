#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    Compile,
    Assert,
    Timeout,
    EnvMissing,
    Other,
}

impl ErrorClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compile => "compile",
            Self::Assert => "assert",
            Self::Timeout => "timeout",
            Self::EnvMissing => "env_missing",
            Self::Other => "other",
        }
    }
}

pub fn classify_error(blob: &str) -> ErrorClass {
    let b = blob.to_ascii_lowercase();
    if b.contains("timeout") || b.contains("превысила таймаут") || b.contains("timed out")
    {
        return ErrorClass::Timeout;
    }
    if b.contains("not found")
        || b.contains("no such file")
        || b.contains("command not found")
        || b.contains("не найден")
        || b.contains("cannot find binary")
        || b.contains("no module named")
        || b.contains("modulenotfound")
        || b.contains("error: toolchain")
        || b.contains("cargo: command")
    {
        return ErrorClass::EnvMissing;
    }
    if b.contains("error[e")
        || b.contains("cannot find value")
        || b.contains("unresolved import")
        || b.contains("syntaxerror")
        || b.contains("compile error")
        || b.contains("compilation failed")
        || b.contains("tsc:")
        || b.contains("cannot open source file")
    {
        return ErrorClass::Compile;
    }
    if b.contains("assertion")
        || b.contains("assert_eq")
        || b.contains("asserted")
        || (b.contains("expected") && (b.contains("assert") || b.contains("expect(")))
        || b.contains("panic")
    {
        return ErrorClass::Assert;
    }
    ErrorClass::Other
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_compile() {
        assert_eq!(
            classify_error("error[E0308]: mismatched types"),
            ErrorClass::Compile
        );
    }
}
