# Agent Ergonomics Scorecard - hashbytes pass 1

Target: `/Users/zac/Source/cmdrvl/hash`
Binary: `hashbytes`
Mode: full

## Summary

- Inventoried surfaces: 69
- Scored surfaces: 7 high-leverage agent surfaces
- Applied recommendations: 7 / 7
- Intent corpus: 114 total, 114 useful_hint, 0 silent_fail, 0 useless_error
- Median uplift estimate: +180 points across the applied surfaces

## Biggest Uplifts

1. `hashbytes --robot-triage` now works at top level and emits machine-readable health/capability/recommendation JSON before input or witness side effects.
2. `hashbytes capabilities --json` now gives agents a stable contract without requiring the nested doctor namespace.
3. `hashbytes robot-docs guide` gives paste-ready in-tool guidance.
4. `hashbytes --json` is accepted as structured-output intent while preserving the existing JSONL stream contract.
5. Refusals now populate `.refusal.next_command` with concrete recovery commands.
6. `hashbytes doctor --fix` exits 2 with safe alternatives instead of generic parser help.
7. README and release formula generation now align with the `cmdrvl-hash` Homebrew formula and `hashbytes` binary identity.

## Remaining Risks

- The tool relies on clap's built-in typo hints rather than a custom correction layer. Current deterministic intent corpus classified all 114 generated wrong invocations as useful hints.
- `hashbytes --json` is a no-op for the main stream by design; future agents should not reinterpret it as a pretty JSON array mode.
