# Commands

Most commands take `<PATH>` - path to the project or any file inside it (root is detected via the manifest).

## Global options

| Option                | Description                                                             |
| --------------------- | ----------------------------------------------------------------------- |
| `--format text\|json` | Output format (`text` by default)                                       |
| `--debug`             | Verbose log (prompts, HTTP to the model)                                |
| `--locale en\|ru`     | UI language (overrides `TESTER_LOCALE` / `LANG` / `tester.toml` locale) |
| `-h`, `--help`        | Help                                                                    |
| `-V`, `--version`     | Version                                                                 |

Model, `base_url`, `api_key`, and `agent_auto_approve` are set only in `tester.toml` (`[llm]` / `[run]`). Locale: `--locale` -> `TESTER_LOCALE` -> `LANG` -> `locale` in `tester.toml` -> `en`.

## Test pipeline (`tester test ...`)

Subcommands: `scan` / `plan` / `generate` / `run` / `repair` / `eval`.

### `test scan`

Project profile without calling the model: language, package, test command, frameworks, capabilities, notes.

```bash
tester test scan /path/to/project
tester test scan /path/to/project --format json
```

### `test plan`

List of coverage targets. Does not call the model.

```bash
tester test plan /path/to/project --kind unit
tester test plan /path/to/project --kind all --base main
tester test plan /path/to/project --kind unit --bug "panic in multiply"
tester test plan /path/to/project --kind unit --max-items 10 --coverage lcov.info
tester test plan /path/to/project --show
tester test plan /path/to/project --apply
```

| Flag                | Description                                              |
| ------------------- | -------------------------------------------------------- |
| `--kind <KIND>`     | `all` (default), `unit`, `integration`, `ui`, `contract` |
| `--base <REF>`      | Only symbols from `git diff REF...HEAD`                  |
| `--bug <TEXT>`      | Prioritize symbols from traceback / bug text             |
| `--bug-file <PATH>` | Same, but text from a file                               |
| `--append`          | Plan appending to existing test files                    |
| `--package <NAME>`  | Package filter (Cargo workspace and similar)             |
| `--show`            | Print saved plan from `.tester/plan.json`                |
| `--apply`           | Generate/write from the saved plan                       |

### `test generate`

Generate drafts via LLM. By default files are not written - a diff is printed. Use `--write` to persist.

```bash
tester test generate /path/to/project --kind unit
tester test generate /path/to/project --kind unit --write
tester test generate /path/to/project --kind unit --write --update
tester test generate /path/to/project --kind unit --append --write
```

| Flag                                                               | Description                                                                      |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| `--kind`, `--base`, `--bug`, `--bug-file`, `--append`, `--package` | Same as `test plan`                                                              |
| `--write`                                                          | Write files to disk                                                              |
| `--update`                                                         | Allow overwriting existing test files (without the flag - reject, exit code `5`) |

### `test run`

Full cycle: plan -> generate -> write -> run -> repair on failure (up to `repair_attempts`).

```bash
tester test run /path/to/project
tester test run /path/to/project --kind ui
tester test run /path/to/project --dry-run
```

Flags match `test generate`, but writing is on by default; `--dry-run` - diff only, no write and no test run.

### `test repair`

Continue fixing the last failed draft (state in the project's `.tester/`).

```bash
tester test repair /path/to/project
```

### `test eval`

Mutations of covered symbols (arithmetic / boolean / off-by-one / branch / null). No model required.

```bash
tester test eval /path/to/project --max-symbols 5
tester test eval /path/to/projects --suite
tester test eval /path/to/project --json
```

## `debug`

From traceback -> minimal reproducing test -> validate/repair.

```bash
tester debug /project --bug "panic in multiply at src/lib.rs:10"
tester debug /project --bug-file ./fail.log --reproduce-only
echo "$TRACE" | tester debug /project --stdin --reproduce-only
```

## `doctor` / `ui doctor`

Check LLM, toolchain, UI runner, android/docker.

```bash
tester doctor /project
tester ui doctor /web-app
```

## `agent` / `chat`

Console agent with a tool-calling loop (scan/plan/write_test/run_tests/repair + fs/git/analysis).

```bash
tester agent /path/to/project "add unit tests for multiply"
tester agent /path/to/project "fix failing tests" --resume --dry-run
tester chat /path/to/project
```

| Flag          | Description                                          |
| ------------- | ---------------------------------------------------- |
| `--dry-run`   | Do not write files / do not run write shell cmds     |
| `--resume`    | Continue the active chat under `~/.tester/sessions/` |
| `--stream`    | Token streaming (on by default)                      |
| `--no-stream` | Disable streaming                                    |

In `chat`: `/scan`, `/plan [kind]`, `/run <prompt>`, `/clear`, `/audit`, `/grep`, `/symbol`, `/refs`, `/search`, `/index`, `/map`, `/list_files`, `/show_file`, `/exit`.

For the full-screen UI, see [tui.md](tui.md).

## `sessions`

List / prune multi-chat sessions under `~/.tester/sessions/<id>/` (meta + transcript; workspace pointer `.tester/session_id`).

```bash
tester sessions list
tester sessions list --workspace /path/to/project
tester sessions prune <id>
tester sessions prune --workspace /path/to/project
tester sessions prune --older-than 30
tester sessions prune --all
```

In TUI: `/session` picker`/session new``/session <id>``/prune ...`. `/recent` is **projects**, not chat sessions.

## `ui init`

Minimal Playwright setup: `playwright.config.ts`, `e2e/` directory, UI section in `tester.toml` (if the file does not exist yet), `npm install` + chromium when possible.

```bash
tester ui init /path/to/web-app
tester test run /path/to/web-app --kind ui
```
