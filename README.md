# Tester

LLM agent for generating and fixing tests: unit, integration, UI, and contract.

Supported languages: Go, C++, Rust, JS/TS, Dart/Flutter, Kotlin, Python, PHP.

### Build from source

```bash
cargo build --release
# binary: target/release/tester
```

## Quick start

```bash
tester run /path/to/project
tester agent /path/to/project "cover multiply with unit tests"
tester chat /path/to/project
tester tui
tester tui /path/to/project
```

### Common scenarios

```bash
# Write unit tests
tester run /project --kind unit

# Fix a bug from a traceback
tester debug /project --bug-file /tmp/panic.txt --reproduce-only

# Check test usefulness (mutations)
tester eval /project --max-symbols 5
tester eval ./tests/fixtures --suite

# UI e2e
tester ui init /web-app
tester ui doctor /web-app
tester run /web-app --kind ui

# Environment health
tester doctor /project
```

Config: `<root>/tester.toml`. Default `--kind all`; `run` writes files immediately (disable with `--dry-run`).

## Global options

| Option                | Description                              |
| --------------------- | ---------------------------------------- |
| `--format text\|json` | Output format (`text` by default)        |
| `--debug`             | Verbose log (prompts, HTTP to the model) |
| `-h`, `--help`        | Help                                     |
| `-V`, `--version`     | Version                                  |

Model, `base_url`, `api_key`, and `agent_auto_approve` are set only in `tester.toml` (`[llm]` / `[run]`).

## Commands

Most commands take `<PATH>` - path to the project or any file inside it (root is detected via the manifest).

### `scan`

Project profile without calling the model: language, package, test command, frameworks, capabilities, notes.

```bash
tester scan /path/to/project
tester scan /path/to/project --format json
```

### `plan`

List of coverage targets. Does not call the model.

```bash
tester plan /path/to/project --kind unit
tester plan /path/to/project --kind all --base main
tester plan /path/to/project --kind unit --bug "panic in multiply"
```

| Flag                | Description                                              |
| ------------------- | -------------------------------------------------------- |
| `--kind <KIND>`     | `all` (default), `unit`, `integration`, `ui`, `contract` |
| `--base <REF>`      | Only symbols from `git diff REF...HEAD`                  |
| `--bug <TEXT>`      | Prioritize symbols from traceback / bug text             |
| `--bug-file <PATH>` | Same, but text from a file                               |
| `--append`          | Plan appending to existing test files                    |
| `--package <NAME>`  | Package filter (Cargo workspace and similar)             |

### `generate`

Generate drafts via LLM. By default files are not written - a diff is printed. Use `--write` to persist.

```bash
tester generate /path/to/project --kind unit
tester generate /path/to/project --kind unit --write
tester generate /path/to/project --kind unit --write --update
tester generate /path/to/project --kind unit --append --write
```

| Flag                                                               | Description                                                                      |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| `--kind`, `--base`, `--bug`, `--bug-file`, `--append`, `--package` | Same as `plan`                                                                   |
| `--write`                                                          | Write files to disk                                                              |
| `--update`                                                         | Allow overwriting existing test files (without the flag - reject, exit code `5`) |

### `run`

Full cycle: plan -> generate -> write -> run -> repair on failure (up to `repair_attempts`).

```bash
tester run /path/to/project
tester run /path/to/project --kind ui
tester run /path/to/project --dry-run
```

Flags match `generate`, but writing is on by default; `--dry-run` - diff only, no write and no test run.

### `repair`

Continue fixing the last failed draft (state in the project's `.tester/`).

```bash
tester repair /path/to/project
```

### `eval`

Mutations of covered symbols (arithmetic / boolean / off-by-one / branch / null). No model required.

```bash
tester eval /path/to/project --max-symbols 5
tester eval /path/to/fixtures --suite
tester eval /path/to/project --json
```

### `debug`

From traceback -> minimal reproducing test -> validate/repair.

```bash
tester debug /project --bug "panic in multiply at src/lib.rs:10"
tester debug /project --bug-file ./fail.log --reproduce-only
echo "$TRACE" | tester debug /project --stdin --reproduce-only
```

### `doctor` / `ui doctor`

Check LLM, toolchain, UI runner, android/docker.

```bash
tester doctor /project
tester ui doctor /web-app
```cargo build
### `agent` / `chat`

Console agent with a tool-calling loop (scan/plan/write_test/run_tests/repair + fs/git).

```bash
tester agent /path/to/project "add unit tests for multiply"
tester agent /path/to/project "fix failing tests" --resume --dry-run
tester chat /path/to/project
```

| Flag          | Description                                      |
| ------------- | ------------------------------------------------ |
| `--dry-run`   | Do not write files / do not run write shell cmds |
| `--resume`    | Continue `.tester/session.json`                  |
| `--stream`    | Token streaming (on by default)                  |
| `--no-stream` | Disable streaming                                |

In `chat`: `/scan`, `/plan [kind]`, `/run <prompt>`, `/config`, `/clear`, `/audit`, `/exit`.

### `tui`

Interactive terminal UI. Requires a TTY. Without `PATH`, opens the project/folder picker. Fallback: `--plain` -> `chat` (PATH required).

```bash
tester tui
tester tui /path/to/project --dry-run
tester tui /path/to/project --plain
```

| Flag        | Description                                     |
| ----------- | ----------------------------------------------- |
| `--dry-run` | Passed through to future session / agent wiring |
| `--resume`  | Reserved for session resume                     |
| `--plain`   | Use classic `chat` REPL instead of TUI          |

Keybinds: `Ctrl+O` open project picker  `↑↓` navigate  `Enter` open folder  `Ctrl+Enter` / `Ctrl+S` select  type to filter  `Esc` back  `q` quit. Recents: `~/.local/share/.tester/recents.json`.

### `ui init`

Minimal Playwright setup: `playwright.config.ts`, `e2e/` directory, UI section in `tester.toml` (if the file does not exist yet), `npm install` + chromium when possible.

```bash
tester ui init /path/to/web-app
tester run /path/to/web-app --kind ui
```

## Config `tester.toml`

```toml
[llm]
base_url = "http://127.0.0.1:8080/v1"
model = ""
api_key = ""
max_tokens = 4096
timeout_secs = 120

[tests.ui]
framework = "playwright"
base_url = "http://127.0.0.1:3000"
start = "python3 -m http.server 3000"

[run]
repair_attempts = 3
timeout_secs = 120
allow_docker = false
max_context_chars = 24000
android_opt_in = false
max_agent_turns = 20
max_tool_calls = 40
agent_deadline_secs = 600
agent_auto_approve = false
```

### `[llm]`

| Key            | Default                    | Description                            |
| -------------- | -------------------------- | -------------------------------------- |
| `base_url`     | `http://127.0.0.1:8080/v1` | OpenAI-compatible API base URL         |
| `model`        | - (required)               | Model name                             |
| `api_key`      | none                       | API key (from file only, not from env) |
| `max_tokens`   | `4096`                     | Response token limit                   |
| `timeout_secs` | `120`                      | Model request timeout                  |

### `[tests.ui]`

| Key                  | Default                 | Description                                             |
| -------------------- | ----------------------- | ------------------------------------------------------- |
| `tests.ui.framework` | `playwright`            | UI framework                                            |
| `tests.ui.base_url`  | `http://127.0.0.1:3000` | Playwright baseURL                                      |
| `tests.ui.start`     | none                    | Shell command to start a local server before the UI run |

All kinds (unit, integration, UI, contract) are always enabled; filter with `--kind`.

### `[run]`

| Key                   | Default | Description                                  |
| --------------------- | ------- | -------------------------------------------- |
| `repair_attempts`     | `3`     | Repair attempt budget                        |
| `timeout_secs`        | `120`   | Validate timeout (test run)                  |
| `allow_docker`        | `false` | Enables the agent's `docker_info` tool       |
| `max_context_chars`   | `24000` | Max context / compaction characters          |
| `android_opt_in`      | `false` | Kotlin: `connectedAndroidTest` / androidTest |
| `max_agent_turns`     | `20`    | Agent loop turn limit                        |
| `max_tool_calls`      | `40`    | Tool call limit per session                  |
| `agent_deadline_secs` | `600`   | Agent deadline                               |
| `agent_auto_approve`  | `false` | Auto-approve write/shell in `agent`/`chat`   |

Required for: `generate`, `run`, `repair`, `agent`, `chat`.  
Not required for: `scan`, `plan`, `eval`, `ui init`.

## Exit codes

| Code | Meaning                                                  |
| ---- | -------------------------------------------------------- |
| `0`  | success                                                  |
| `2`  | no goals in the plan                                     |
| `3`  | model unavailable                                        |
| `4`  | test not fixed within repair budget (`unfixed`)          |
| `5`  | write refused due to path / policy / `--update` required |

## Threats and mitigations

| Threat               | Mitigation                                    |
| -------------------- | --------------------------------------------- |
| Writes outside tests | Language adapter write policy; exit code `5`  |
| Path with `..`       | Normalization relative to project root        |
| Infinite repair      | `run.repair_attempts`                         |
| Arbitrary commands   | Adapter argv only + optional `tests.ui.start` |
