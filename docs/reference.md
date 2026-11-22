# Reference

## Exit codes

| Code | Meaning                                                  |
| ---- | -------------------------------------------------------- |
| `0`  | success                                                  |
| `2`  | no goals in the plan / security findings ≥ `--fail-on`   |
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
