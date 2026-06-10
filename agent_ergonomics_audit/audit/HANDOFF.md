# HANDOFF - hashbytes agent ergonomics pass 1

## Landed

- Top-level `hashbytes --robot-triage`.
- Top-level `hashbytes capabilities --json`.
- Top-level `hashbytes robot-docs guide`.
- Global `hashbytes --json` accepted as a no-op for the existing JSONL stream.
- Refusal envelopes now include concrete `.refusal.next_command` values.
- `hashbytes doctor --fix` exits 2 with safe read-only alternatives.
- README/spec/operator/release workflow aligned for version `0.5.0` and Homebrew formula `cmdrvl-hash`.

## Verification

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- Runtime probes for triage, capabilities, robot docs, doctor fix, and refusal next command
- Intent corpus: 114 total, 114 useful_hint, 0 silent_fail, 0 useless_error

## Next Pass

Only add a custom typo/intent inference layer if real agent traces show gaps beyond clap's current suggestions. Current generated corpus did not justify a bespoke handler.
