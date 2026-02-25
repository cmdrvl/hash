# AGENTS.md — hash

> Guidelines for AI coding agents working in this Rust codebase.

---

## hash — What This Project Does

`hash` computes byte identity for manifest records by adding deterministic cryptographic content hashes (`bytes_hash`) to each record in a JSONL stream.

Pipeline position:

```
vacuum → hash → fingerprint → lock → pack
```

### Quick Reference

```bash
# Core pipeline
vacuum /data/dec | hash > dec.hashed.jsonl

# Alternate algorithm
vacuum /data/dec | hash --algorithm blake3 > dec.hashed.jsonl

# Quality gate
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

### Source of Truth

- **Spec:** [`docs/PLAN.md`](./docs/PLAN.md) — behavior must follow this document.
- Do not invent behavior not present in the plan.

### Key Files (planned)

| File | Purpose |
|------|---------|
| `src/main.rs` | CLI entry + exit code mapping |
| `src/lib.rs` | orchestration flow |
| `src/cli/` | argument parsing + witness subcommands |
| `src/hash/` | streaming SHA-256 / BLAKE3 implementations |
| `src/pipeline/` | input reader, enricher, parallel ordered pipeline |
| `src/output/` | JSONL output writer |
| `src/refusal/` | refusal envelope and codes |
| `src/witness/` | witness append/query behavior |
| `operator.json` | machine-readable operator contract |

---

## Output Contract (Critical)

`hash` is a **stream enrichment tool**:

- Normal path emits JSONL records to stdout (one output per input record).
- Refusal path emits one refusal JSON envelope to stdout.
- No human-report mode on stdout.

| Exit | Meaning |
|------|---------|
| `0` | `ALL_HASHED` — every record hashed successfully |
| `1` | `PARTIAL` — some records marked `_skipped: true` |
| `2` | `REFUSAL` — pipeline/input-level failure |

---

## Core Invariants (Do Not Break)

### 1. Hash format contract

- `bytes_hash` must be `<algorithm>:<lowercase-hex>`.
- Supported algorithms in v0: `sha256`, `blake3`.
- `hash_algorithm` must match `bytes_hash` prefix.

### 2. Deterministic ordering

- Output order must match input order regardless of parallel worker completion order.
- Same input + same files must yield identical output bytes across `--jobs` values.

### 3. `_skipped` semantics

- Upstream `_skipped: true` records are passed through (not re-hashed).
- Hashing I/O failures produce `_skipped: true` with appended `_warnings` (code `E_IO`) and continue processing.
- `_skipped` records must have `bytes_hash: null` and `hash_algorithm: null`.

### 4. Version + provenance accumulation

- Output record `version` must be `hash.v0`.
- `tool_versions` must preserve upstream versions and include `hash`.

### 5. Refusal boundary

- Invalid JSONL / missing required fields / stream-level I/O are refusals (`exit 2`).
- Per-record file-read failures are partial records, not refusals.

### 6. Witness parity

Ambient witness semantics must match spine conventions (`vacuum`/`shape`/`rvl`/`lock` parity):
- append by default,
- `--no-witness` opt-out,
- witness failures do not mutate domain outcome semantics,
- witness query subcommands supported (`query`, `last`, `count`).

---

## Toolchain

- **Language:** Rust, Cargo only.
- **Edition:** 2024 (or `rust-toolchain.toml` when present).
- **Unsafe code:** forbidden in binary (`#![forbid(unsafe_code)]`).
- **Dependencies:** explicit versions, small and pinned.

Release profile:

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

---

## Quality Gate

Run after any substantive change:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

### Minimum Coverage Areas

- algorithm correctness and known vectors,
- JSONL parsing and refusal paths,
- upstream/new `_skipped` handling,
- deterministic output ordering across `--jobs`,
- outcome and exit-code routing,
- witness append/query behavior,
- E2E spine compatibility (`vacuum → hash → fingerprint → lock`).

---

## Git and Release

- **Primary branch:** `main`.
- **`master`** exists for legacy URL compatibility — keep synced: `git push origin main:master`.
- Bump `Cargo.toml` semver appropriately on release.
- Sync `Cargo.lock` before release workflows that use `--locked`.

---

## Editing Rules

- **No file deletion** without explicit written user permission.
- **No destructive git commands** (`reset --hard`, `clean -fd`, `rm -rf`, force push) without explicit authorization.
- **No scripted mass edits** — make intentional, reviewable changes.
- **No file proliferation** — edit existing files; create new files only for real new functionality.
- **No surprise behavior** — do not invent behavior not in `docs/PLAN.md`.
- **No backwards-compatibility shims** unless explicitly requested.

---

## RULE 0

If the user gives a direct instruction, follow it even if it conflicts with defaults in this file.

---

## Beads (`br`) Workflow

Use Beads as source of truth for task state.

```bash
br ready              # Show unblocked ready work
br list --status=open # All open issues
br show <id>          # Full issue details
br update <id> --status=in_progress
br close <id> --reason "Completed"
br sync --flush-only  # Export to JSONL (no git ops)
```

Pick unblocked beads. Mark in-progress before coding. Close with validation evidence.

---

## Agent Mail (Multi-Agent Sessions)

When Agent Mail is available:

- Register identity in this project.
- Reserve only specific files you are actively editing — never entire directories.
- Send start/finish updates per bead.
- Poll inbox regularly and acknowledge `ack_required` messages promptly.
- Release reservations when done.

---

## Session Completion

Before ending a session:

1. Run quality gate (`fmt` + `clippy` + `test`).
2. Confirm docs/spec alignment for behavior changes.
3. Commit with precise message.
4. Push `main` and sync `master`.
5. Summarize: what changed, what was validated, remaining risks.
