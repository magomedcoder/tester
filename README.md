# Tester

LLM agent for generating and fixing tests: unit, integration, UI, and contract.

Supported languages: Go, C++, Rust, JS/TS, Dart/Flutter, Kotlin, Python, PHP.

### Build from source

Cargo workspace (`crates/*`). From the repo root:

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
tester test eval ./tests/fixtures --suite

# UI e2e
tester ui init /web-app
tester ui doctor /web-app
tester test run /web-app --kind ui

# Environment health
tester doctor /project

# Multi-chat sessions
tester sessions list
tester /project --session <id>
```

Config: `<root>/tester.toml`. Default `--kind all`; `test run` writes files immediately (disable with `--dry-run`).

## Docs

| Topic                          | Link                                  |
| ------------------------------ | ------------------------------------- |
| CLI commands & global options  | [docs/en/commands.md](docs/en/commands.md) |
| Interactive TUI                | [docs/en/tui.md](docs/en/tui.md)           |
| Config `tester.toml`           | [docs/en/config.md](docs/en/config.md)     |
| Exit codes & threats           | [docs/en/reference.md](docs/en/reference.md) |


[Русский](README-ru.md)
