use std::process::{Command, Stdio};

pub fn read() -> Option<String> {
    pipe_read(&[
        ("wl-paste", &["-n"]),
        ("xclip", &["-selection", "clipboard", "-o"]),
        ("pbpaste", &[]),
    ])
}

fn read_primary() -> Option<String> {
    pipe_read(&[
        ("wl-paste", &["-n", "--primary"]),
        ("xclip", &["-selection", "primary", "-o"]),
    ])
}

pub fn read_paste() -> Option<String> {
    read_primary().or_else(read)
}

fn pipe_read(cmds: &[(&str, &[&str])]) -> Option<String> {
    for &(bin, args) in cmds {
        if let Ok(out) = Command::new(bin)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).into_owned();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }
    None
}
