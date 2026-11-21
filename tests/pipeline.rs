use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tester"))
}

fn write_rust_demo(dir: &std::path::Path) {
    std::fs::write(
        dir.join("Cargo.toml"),
        r#"[package]
name = "rust_demo"
version = "0.1.0"
edition = "2021"
[lib]
path = "src/lib.rs"
"#,
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("testdata")).unwrap();
    std::fs::write(
        dir.join("src/lib.rs"),
        r#"pub fn add(a: i32, b: i32) -> i32 { a + b }
pub fn multiply(a: i32, b: i32) -> i32 { a * b }
pub fn classify(n: i32) -> &'static str {
    if n < 0 { "negative" } else if n == 0 { "zero" } else { "positive" }
}
pub fn read_config(path: &std::path::Path) -> std::io::Result<String> {
    Ok(std::fs::read_to_string(path)?.trim().to_string())
}
pub fn lookup_port(name: &str) -> Option<u16> {
    match name { "http" => Some(8080), "https" => Some(8443), _ => None }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn add_works() { assert_eq!(add(2, 3), 5); }
}
"#,
    )
    .unwrap();
    std::fs::write(dir.join("testdata/app.conf"), "mode=demo\n").unwrap();
}

fn write_go_demo(dir: &std::path::Path) {
    std::fs::write(
        dir.join("go.mod"),
        "module example.com/go_demo\n\ngo 1.22\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("math.go"),
        r#"package godemo
import "os"
func Add(a, b int) int { return a + b }
func Multiply(a, b int) int { return a * b }
func ReadLabel(path string) (string, error) {
    b, err := os.ReadFile(path)
    if err != nil { return "", err }
    return string(b), nil
}
"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("math_test.go"),
        "package godemo\nimport \"testing\"\nfunc TestAdd(t *testing.T) { if Add(2,3)!=5 { t.Fatal(\"add\") } }\n",
    )
    .unwrap();
}

fn write_python_demo(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::write(
        dir.join("pyproject.toml"),
        "[project]\nname=\"python_demo\"\nversion=\"0.1.0\"\n[tool.pytest.ini_options]\npythonpath=[\"src\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/mathutil.py"),
        "def add(a,b): return a+b\ndef multiply(a,b): return a*b\ndef read_label(path):\n    with open(path) as f: return f.read().strip()\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("tests/test_add.py"),
        "from mathutil import add\ndef test_add():\n    assert add(2,3)==5\n",
    )
    .unwrap();
}

#[test]
fn scan_rust_project() {
    let dir = tempfile::tempdir().unwrap();
    write_rust_demo(dir.path());
    let out = Command::new(bin())
        .args(["scan", dir.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("язык: rust"));
    assert!(stdout.contains("команда_тестов: cargo test"));
    assert!(stdout.contains("возможности:"));
}

#[test]
fn scan_detects_other_languages() {
    type DemoWriter = Box<dyn Fn(&std::path::Path)>;
    let cases: &[(&str, DemoWriter)] = &[
        ("go", Box::new(write_go_demo)),
        ("python", Box::new(write_python_demo)),
    ];
    for (lang, write) in cases {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path());
        let out = Command::new(bin())
            .args(["scan", dir.path().to_str().unwrap()])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{lang}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains(&format!("язык: {lang}")), "{lang} {stdout}");
        assert!(stdout.contains("поддерживается: true"), "{lang} {stdout}");
    }
}

#[test]
fn plan_unit_skips_covered_add() {
    let dir = tempfile::tempdir().unwrap();
    write_rust_demo(dir.path());
    let out = Command::new(bin())
        .args(["plan", dir.path().to_str().unwrap(), "--kind", "unit"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("multiply"));
    assert!(!stdout.lines().any(|l| l.contains("] add ")));
}

#[test]
fn scan_fixtures_dir() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for (lang, rel) in [
        ("rust", "rust"),
        ("go", "go"),
        ("python", "python"),
        ("php", "php"),
        ("cpp", "cpp"),
    ] {
        let path = root.join(rel);
        let out = Command::new(bin())
            .args(["scan", path.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{lang}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains(&format!("язык: {lang}")), "{lang} {stdout}");
    }
}

#[test]
fn help_lists_agent_and_chat() {
    let out = Command::new(bin()).args(["--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("agent"));
    assert!(stdout.contains("chat"));
    assert!(stdout.contains("debug"));
    assert!(stdout.contains("doctor"));
}

#[test]
fn doctor_runs_on_rust_fixture() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rust");
    let out = Command::new(bin())
        .args(["doctor", root.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("tester doctor") || stdout.contains("toolchain"),
        "{stdout}"
    );
}

#[test]
fn plan_show_after_plan() {
    let dir = tempfile::tempdir().unwrap();
    write_rust_demo(dir.path());
    let _ = Command::new(bin())
        .args(["plan", dir.path().to_str().unwrap(), "--kind", "unit"])
        .output()
        .unwrap();
    let out = Command::new(bin())
        .args(["plan", dir.path().to_str().unwrap(), "--show"])
        .output()
        .unwrap();
    assert!(
        out.status.success() || out.status.code() == Some(2),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
