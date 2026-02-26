# Coverage Parity Audit (`bd-1rx`)

Date: 2026-02-26 (refreshed after `bd-kmg` closure)  
Scope: `hash` test depth versus `docs/PLAN.md` requirements and parity expectations relative to `rvl` / `shape`.

## Current Coverage Map

### Unit coverage
- Algorithm parsing and hash vector correctness: `tests/hash_engine.rs`, `tests/scaffold_topology.rs`.
- JSONL parsing and refusal mapping: `tests/pipeline_reader.rs`, `tests/refusal_contract.rs`, `tests/refusal_envelope.rs`.
- `_skipped` and warning semantics: `tests/upstream_skipped.rs`, `tests/skipped_passthrough.rs`, `tests/io_failure_handling.rs`, `tests/io_failure_skipped.rs`, `tests/io_skipped_warning.rs`.
- Ordered output and jobs normalization: `tests/ordered_output.rs`, `tests/parallel_jobs.rs`.

### Integration coverage
- CLI contract and flags: `tests/cli_contract.rs`, `tests/operator_describe.rs`, `tests/schema_output.rs`.
- Outcome/exit routing (`0/1/2` + refusal envelope): `tests/run_outcomes.rs`.
- Witness append/query/last/count/no-witness behavior: `tests/witness_append.rs`, `tests/witness_query.rs`, `tests/witness_behavior.rs`.
- Progress stderr structure and stream separation: `tests/progress_output.rs`.

### E2E / spine compatibility coverage
- Vacuum-like manifest passthrough + downstream field compatibility: `tests/e2e_spine.rs`, `tests/e2e_spine_compatibility.rs`, `tests/spine_compatibility.rs`.
- Deterministic output checks across repeated runs and mixed skipped states: `tests/e2e_spine_compatibility.rs`, `tests/spine_compatibility.rs`.

## PLAN Requirement Mapping

From `docs/PLAN.md` “Testing Requirements”:
- Basic hashing tests: **covered**.
- Empty file and known vectors: **covered**.
- Algorithm selection: **covered**.
- Passthrough / `_skipped` / warning accumulation: **covered**.
- Ordering and `--jobs` determinism: **covered**.
- Exit code routing + refusals: **covered**.
- Witness append/query/no-witness: **covered**.
- `--describe`: **covered**.
- Large-file streaming behavior: **partially covered** (deterministic large-input hashing exists; no explicit RSS/peak-memory assertion).
- Golden fixture tests in `tests/fixtures/`: **not covered** (no committed fixture/golden corpus yet).

## Parity Snapshot vs `rvl` / `shape`

- `hash` now has broad unit+integration+E2E contract tests and witness coverage.
- `rvl` / `shape` still have deeper fixture/snapshot harness depth (golden outputs, broader regression corpus organization, matrix-style harness files).
- `hash` parity is strong on functional contracts, but weaker on canonical fixture/golden test assets and long-lived regression corpus structure.

## Prioritized Gaps

### P0 (highest)
1. Add canonical fixture/golden corpus (`tests/fixtures/`) for SHA-256 and BLAKE3 outputs and compare exact JSONL against golden files.
2. Add explicit streaming-memory guard test strategy for large files (at least a bounded-memory assertion approach appropriate for CI stability).

### P1
1. Consolidate overlapping E2E spine files into one canonical suite to reduce duplicate maintenance and keep assertions non-divergent.
2. Add cross-tool smoke harness (optional/conditional) that validates `hash` output shape against expected downstream `fingerprint`/`lock` contract predicates.

### P2
1. Expand negative-path corpus (additional malformed JSONL edge variants and path edge cases).
2. Add regression fixture naming conventions aligned with `rvl`/`shape` test organization.

## Recommended Follow-ups

- Completed in this parity cycle:
  - `bd-kmg` (structured `--progress` stderr events + integration coverage).
- Keep remaining open follow-ups aligned with active backlog:
  - `bd-342` (release workflow parity),
  - `bd-vwp` (final parity gate closure).
- Add new fixture/golden-focused bead(s) for the remaining P0 coverage gaps above.
