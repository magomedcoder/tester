# Tester

LLM agent for generating and fixing tests: unit, integration, UI, and contract.

Supported languages: Go, C++, Rust, JS/TS, Dart/Flutter, Kotlin, Python, PHP.

### Build from source

```bash
cargo build --release -p tester
# binary: target/release/tester
```

## Quick start

```bash
tester
tester /path/to/project
tester test run /path/to/project
tester agent /path/to/project "cover multiply with unit tests"
tester chat /path/to/project
```

### Common scenarios

```bash
# Write unit tests
tester test run /project --kind unit

# Fix a bug from a traceback
tester debug /project --bug-file /tmp/panic.txt --reproduce-only

# Check test usefulness (mutations)
tester test eval /project --max-symbols 5
tester test eval /dir-with-projects --suite

# UI e2e
tester ui init /web-app
tester ui doctor /web-app
tester test run /web-app --kind ui

# Environment health
tester doctor /project

# Defensive security scan + harden patch
tester security scan /project
tester security patch /project --finding F-0001

# Multi-chat sessions
tester sessions list
tester /project --session <id>
```

Config: `~/.tester/tester.toml`.
Default `--kind all`; `test run` writes files immediately (disable with `--dry-run`).

## Docs

- [CLI commands & global options](docs/cli.md)
- [TUI (slash / keybinds)](docs/ui.md)
- [Config `tester.toml`](docs/config.md)
- [Exit codes & threats](docs/reference.md)
- [Security (`tester security`)](docs/cli.md#security)

[Русский](README-ru.md)
