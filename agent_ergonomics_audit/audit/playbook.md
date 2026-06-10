# Agent Ergonomics Playbook - hashbytes

## Canonical Agent Commands

```bash
hashbytes --robot-triage
hashbytes capabilities --json
hashbytes robot-docs guide
vacuum <ROOT> | hashbytes --json
hashbytes doctor --fix
```

## Recovery Rules

- Bad JSONL or missing required fields: read `.refusal.next_command`, usually `vacuum <ROOT> | hashbytes`.
- Invalid algorithm: retry with `hashbytes --algorithm sha256` or `hashbytes --algorithm blake3`.
- Input file open failure: inspect the manifest path with the emitted `ls -la '<path>'` command.
- Repair requests: `doctor --fix` is intentionally unavailable; use top-level triage and capabilities for read-only diagnosis.

## Regression Commands

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
for t in agent_ergonomics_audit/audit/regression_tests/*.test.sh; do bash "$t"; done
```
