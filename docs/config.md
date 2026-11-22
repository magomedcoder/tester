# Config `tester.toml`

Single file: **`~/.tester/tester.toml`** (or `$TESTER_HOME/tester.toml`).  
Created automatically on first run if missing. There is no per-project overlay.

Choose with `/model`, `--model`, `/provider` (planned), `--provider`.
Persist: `~/.tester/model`, `~/.tester/provider`.

```toml
locale = "en"

[[providers]]
id = "llamacpp"
kind = "openai_compat"
base_url = "http://127.0.0.1:8080/v1"
# api_key = "" # only if the server was started with --api-key

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

| Section / key               | Default        | Description                                                                                       |
| --------------------------- | -------------- | ------------------------------------------------------------------------------------------------- |
| `locale`                    | `en`           | UI language: `en` or `ru`                                                                         |
| `[[providers]]`             | one `llamacpp` | Provider entries: `id`, `kind` (`openai_compat`), `base_url`, optional `api_key`                  |
| `run.agent_auto_approve`    | `true`         | Auto-approve write/shell in project root (`false` -> y/N); path escape / write policy still block |
| `run.allow_docker`          | `false`        | Enable agent `docker_info` tool                                                                   |
| `analysis.embeddings`       | `false`        | Opt-in semantic `/search` (calls `/embeddings`)                                                   |
| `security.enabled`          | `true`         | Enable `tester security scan`                                                                     |
| `security.deep`             | `false`        | Intra-file taint pass                                                                             |
| `security.fetch_advisories` | `false`        | Opt-in OSV HTTPS queries                                                                          |

Optional UI: `[tests.ui]`.

Needed for LLM work: selected model (+ active provider).  
Not needed for: `test scan`, `test plan`, `test eval`, `security scan`, `ui init`, `sessions`, `models`.
