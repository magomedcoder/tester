# Config `tester.toml`

Two files (deep merge; project overrides global):

1. **`~/.tester/tester.toml`** (or `$TESTER_HOME/tester.toml`) - `base_url`, `api_key`, locale, shared defaults. Created automatically on first run if missing.
2. **`<project-root>/tester.toml`** - per-repo overlay

Choose with TUI `/model`, CLI `--model`, or `tester models` (auto-pick when the endpoint lists exactly one). Last choice is stored in `~/.tester/model`.

```toml
locale = "en"

[llm]
base_url = "http://127.0.0.1:11434/v1"
# api_key = ""

[run]
agent_auto_approve = true
allow_docker = false

[analysis]
embeddings = false

[security]
enabled = true
deep = false
fetch_advisories = false
```

| Section / key               | Default                     | Description                                                                                       |
| --------------------------- | --------------------------- | ------------------------------------------------------------------------------------------------- |
| `locale`                    | `en`                        | UI language: `en` or `ru`                                                                         |
| `llm.base_url`              | `http://127.0.0.1:11434/v1` | OpenAI-compatible API base                                                                        |
| `llm.api_key`               | none                        | API key (file only, not env)                                                                      |
| `run.agent_auto_approve`    | `true`                      | Auto-approve write/shell in project root (`false` -> y/N); path escape / write policy still block |
| `run.allow_docker`          | `false`                     | Enable agent `docker_info` tool                                                                   |
| `analysis.embeddings`       | `false`                     | Opt-in semantic `/search` (calls `/embeddings`)                                                   |
| `security.enabled`          | `true`                      | Enable `tester security scan`                                                                     |
| `security.deep`             | `false`                     | Intra-file taint pass                                                                             |
| `security.fetch_advisories` | `false`                     | Opt-in OSV HTTPS queries                                                                          |

Optional (UI tests): `[tests.ui]` - `framework`, `base_url`, `start`.  
Advanced overrides (`max_tokens`, repair budget, agent limits, ...) still parse if set, but are not required.

Typical setup: put `[llm]` in the global file; keep only project-specific keys in the workspace file.

Needed for LLM work: TUI chat / `test generate`, `test run`, `test repair`, `agent`, `chat` (plus a selected model).  
Not needed for: `test scan`, `test plan`, `test eval`, `security scan`, `ui init`, `sessions`, `models`.
