use anyhow::{Context, Result, bail};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tester_core::config::Config;
use tester_core::i18n::{self, Msg};
use tester_core::model::{Draft, ProjectProfile, TestKind};
use tester_lang::adapter;
use tracing::{info, warn};

#[derive(Debug)]
pub struct ValidationResult {
    pub ok: bool,
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub trace_path: Option<PathBuf>,
}

pub fn validate_draft(
    profile: &ProjectProfile,
    config: &Config,
    draft: &Draft,
) -> Result<ValidationResult> {
    if draft.kind == TestKind::Ui
        && (profile.ui_framework.as_deref() == Some("playwright")
            || profile.root.join("playwright.config.ts").exists())
    {
        return validate_ui(profile, config, draft);
    }

    let adapter = adapter::adapter_for(profile.language)?;
    let argv = adapter.validate_command(profile, draft);
    if argv.is_empty() {
        bail!("{}", i18n::t(Msg::ErrEmptyValidateCommand));
    }

    let command = argv.join(" ");
    info!(%command, "проверка");
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..])
        .current_dir(&profile.root)
        .env("CI", "1")
        .env("CARGO_TERM_COLOR", "never");
    let output = run_timed(&mut cmd, Duration::from_secs(config.run.timeout_secs))?;
    let ok = output.status_ok
        || vitest_reported_pass(&output.stdout, &output.stderr)
        || go_reported_pass(&output.stdout, &output.stderr)
        || pytest_reported_pass(&output.stdout, &output.stderr);
    Ok(ValidationResult {
        ok,
        command,
        stdout: output.stdout,
        stderr: output.stderr,
        trace_path: None,
    })
}

fn validate_ui(
    profile: &ProjectProfile,
    config: &Config,
    draft: &Draft,
) -> Result<ValidationResult> {
    if profile.language == tester_core::model::Language::Flutter
        || profile.ui_framework.as_deref() == Some("flutter")
    {
        let file = draft.path.display().to_string();
        let mut cmd = Command::new("flutter");
        cmd.args(["test", &file])
            .current_dir(&profile.root)
            .env("CI", "1");
        let output = run_timed(&mut cmd, Duration::from_secs(config.run.timeout_secs))?;
        return Ok(ValidationResult {
            ok: output.status_ok,
            command: format!("flutter test {file}"),
            stdout: output.stdout,
            stderr: output.stderr,
            trace_path: None,
        });
    }

    struct KillOnDrop(Option<Child>);
    impl Drop for KillOnDrop {
        fn drop(&mut self) {
            if let Some(mut child) = self.0.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    let mut guard = KillOnDrop(None);
    let base = config.tests.ui.base_url.clone();
    if !url_up(&base) {
        if let Some(start) = &config.tests.ui.start {
            info!(%start, "запуск UI-сервера");
            guard.0 = Some(spawn_shell(start, &profile.root)?);
            wait_for_url(&base, Duration::from_secs(45))?;
        } else {
            bail!("{}", i18n::tr(Msg::ErrUiBaseUnavailable, &[("base", &base)]));
        }
    }

    let file = draft.path.display().to_string();
    let command = format!("npx playwright test {file}");
    let mut cmd = Command::new("npx");
    cmd.args(["playwright", "test", &file, "--trace", "on"])
        .current_dir(&profile.root)
        .env("CI", "1");
    let output = run_timed(&mut cmd, Duration::from_secs(config.run.timeout_secs))?;

    drop(guard);

    let trace = archive_trace(&profile.root).or_else(|| find_trace(&profile.root));
    Ok(ValidationResult {
        ok: output.status_ok,
        command,
        stdout: output.stdout,
        stderr: output.stderr,
        trace_path: trace,
    })
}

fn archive_trace(root: &Path) -> Option<PathBuf> {
    let src = find_trace(root)?;
    let dest_dir = root.join(".tester").join("traces");
    let _ = std::fs::create_dir_all(&dest_dir);
    let name = src
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "trace.zip".into());
    let dest = dest_dir.join(name);
    std::fs::copy(&src, &dest).ok()?;
    if let Some(parent) = src.parent() {
        for entry in std::fs::read_dir(parent).ok()?.flatten() {
            let p = entry.path();
            if p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| matches!(e, "png" | "webm" | "jpeg" | "jpg"))
            {
                let _ = std::fs::copy(&p, dest_dir.join(entry.file_name()));
            }
        }
    }
    Some(dest)
}

fn spawn_shell(start: &str, cwd: &Path) -> Result<Child> {
    let mut child = if start.starts_with("python3 ") {
        let args: Vec<&str> = start.split_whitespace().collect();
        Command::new(args[0])
            .args(&args[1..])
            .current_dir(cwd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("запуск `{start}`"))?
    } else {
        Command::new("bash")
            .arg("-lc")
            .arg(start)
            .current_dir(cwd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("запуск `{start}`"))?
    };
    std::thread::sleep(Duration::from_millis(300));
    if let Ok(Some(status)) = child.try_wait() {
        bail!(
            "{}",
            i18n::tr(
                Msg::ErrUiStartEarly,
                &[("status", &format!("{status}")), ("start", start)],
            )
        );
    }
    Ok(child)
}

fn url_up(url: &str) -> bool {
    let Some(hostport) = host_port(url) else {
        return false;
    };
    TcpStream::connect_timeout(&hostport, Duration::from_millis(400)).is_ok()
}

fn host_port(url: &str) -> Option<std::net::SocketAddr> {
    let u = url
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let host = u.split('/').next()?;
    let (h, p) = if let Some((h, p)) = host.split_once(':') {
        (h, p.parse().ok()?)
    } else {
        (host, 80u16)
    };
    format!("{h}:{p}").parse().ok()
}

fn wait_for_url(url: &str, budget: Duration) -> Result<()> {
    let start = Instant::now();
    while start.elapsed() < budget {
        if url_up(url) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    bail!("{}", i18n::tr(Msg::ErrWaitUrlTimeout, &[("url", url)]));
}

fn find_trace(root: &Path) -> Option<PathBuf> {
    let dir = root.join("test-results");
    if !dir.is_dir() {
        return None;
    }
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().contains("trace"))
        .map(|e| e.path().to_path_buf())
}

struct TimedOutput {
    status_ok: bool,
    stdout: String,
    stderr: String,
}

fn run_timed(cmd: &mut Command, timeout: Duration) -> Result<TimedOutput> {
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("запуск команды проверки")?;

    let start = Instant::now();
    loop {
        match child.try_wait()? {
            Some(status) => {
                let mut stdout = String::new();
                let mut stderr = String::new();
                if let Some(mut out) = child.stdout.take() {
                    let mut buf = Vec::new();
                    let _ = out.read_to_end(&mut buf);
                    stdout = String::from_utf8_lossy(&buf).into_owned();
                }
                if let Some(mut err) = child.stderr.take() {
                    let mut buf = Vec::new();
                    let _ = err.read_to_end(&mut buf);
                    stderr = String::from_utf8_lossy(&buf).into_owned();
                }
                return Ok(TimedOutput {
                    status_ok: status.success(),
                    stdout,
                    stderr,
                });
            }
            None => {
                if start.elapsed() > timeout {
                    warn!("проверка превысила таймаут; завершаем процесс");
                    let _ = child.kill();
                    let _ = child.wait();
                    bail!(
                        "{}",
                        i18n::tr(
                            Msg::ErrValidateTimeout,
                            &[("secs", &timeout.as_secs().to_string())],
                        )
                    );
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn vitest_reported_pass(stdout: &str, stderr: &str) -> bool {
    let blob = format!("{stdout}\n{stderr}");
    if blob.contains("AssertionError") || blob.contains("\nFAIL ") {
        return false;
    }
    let files_ok = regex::Regex::new(r"Test Files\s+([1-9]\d*)\s+passed")
        .ok()
        .and_then(|re| re.captures(&blob))
        .is_some();
    let checkmark_ok = blob.contains('✓')
        && regex::Regex::new(r"✓[^\n]*\(\s*[1-9]\d*\s+tests?\s*\)")
            .ok()
            .and_then(|re| re.find(&blob))
            .is_some();
    let tests_failed = regex::Regex::new(r"Tests\s+[1-9]\d*\s+failed")
        .ok()
        .and_then(|re| re.captures(&blob))
        .is_some();
    (files_ok || checkmark_ok) && !tests_failed
}

fn go_reported_pass(stdout: &str, stderr: &str) -> bool {
    let blob = format!("{stdout}\n{stderr}");
    blob.contains("PASS") && !blob.contains("FAIL")
}

fn pytest_reported_pass(stdout: &str, stderr: &str) -> bool {
    let blob = format!("{stdout}\n{stderr}");
    (blob.contains(" passed") || blob.contains("passed in")) && !blob.contains(" failed")
}

pub fn write_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::File::create(path)?;
    f.write_all(bytes)?;
    Ok(())
}
