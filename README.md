# hash

<div align="center">

[![CI](https://github.com/cmdrvl/hash/actions/workflows/ci.yml/badge.svg)](https://github.com/cmdrvl/hash/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![GitHub release](https://img.shields.io/github/v/release/cmdrvl/hash)](https://github.com/cmdrvl/hash/releases)

**Prove what's inside every file — byte-level identity for every artifact in a manifest.**

```bash
brew install cmdrvl/tap/hash
```

</div>

---

You have a manifest of files. You know they exist. But if someone asks "has this file changed since Tuesday?" — timestamps lie, filenames are meaningless, and `ls -la` proves nothing. You need byte-level proof.

**hash reads each file through a 64 KB streaming buffer and adds its cryptographic identity — SHA-256 or BLAKE3 — to every record in the manifest.** Parallel by default, constant memory, deterministic output order regardless of how many workers are running. Same files in, same hashes out.

### What makes this different

- **Constant memory** — 64 KB buffer per worker. A 100 GB file uses the same RAM as a 1 KB file.
- **Parallel with deterministic ordering** — `--jobs N` workers hash simultaneously, but output order always matches input order. No surprises.
- **Two algorithms** — SHA-256 (the default, universally accepted) or BLAKE3 (`--algorithm blake3`, faster on large files).
- **Pipeline native** — reads `vacuum` JSONL, emits enriched JSONL for `fingerprint` and `lock`.

---

## Quick Example

```bash
$ vacuum /data/dec | hash
```

```jsonl
{"version":"hash.v0","path":"/data/dec/model.xlsx","relative_path":"model.xlsx","root":"/data/dec","size":2481920,"mtime":"2025-12-31T12:00:00.000Z","extension":".xlsx","mime_guess":"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet","bytes_hash":"sha256:e3b0c44298fc1c149afbf4c8996fb924","hash_algorithm":"sha256","tool_versions":{"vacuum":"0.1.0","hash":"0.1.0"}}
{"version":"hash.v0","path":"/data/dec/tape.csv","relative_path":"tape.csv","root":"/data/dec","size":847201,"mtime":"2025-12-15T08:30:00.000Z","extension":".csv","mime_guess":"text/csv","bytes_hash":"sha256:7d865e959b2466918c9863afca942d0f","hash_algorithm":"sha256","tool_versions":{"vacuum":"0.1.0","hash":"0.1.0"}}
```

Two files hashed — SHA-256 by default, tool versions accumulated, ready for `fingerprint` or `lock`.

```bash
# Use BLAKE3 for faster hashing:
$ vacuum /data/dec | hash --algorithm blake3

# Sequential processing (deterministic debugging):
$ vacuum /data/dec | hash --jobs 1

# Full pipeline into lockfile:
$ vacuum /data/dec | hash | lock --dataset-id "dec" > dec.lock.json

# With fingerprinting:
$ vacuum /data/models | hash | fingerprint --fp argus-model.v1 \
    | lock --dataset-id "models" > models.lock.json
```

---

## Where hash Fits

`hash` is the **second tool** in the stream pipeline — it establishes byte identity.

```
vacuum  →  hash  →  fingerprint  →  lock  →  pack
(scan)    (hash)    (template)     (pin)    (seal)
```

Each tool reads JSONL from stdin and emits enriched JSONL to stdout. `hash` receives records from `vacuum` and adds `bytes_hash` — the cryptographic content identity that downstream tools depend on.

---

## What hash Is Not

`hash` does not replace other pipeline tools.

| If you need... | Use |
|----------------|-----|
| Enumerate files in a directory | [`vacuum`](https://github.com/cmdrvl/vacuum) |
| Match files against template definitions | [`fingerprint`](https://github.com/cmdrvl/fingerprint) |
| Pin artifacts into a self-hashed lockfile | [`lock`](https://github.com/cmdrvl/lock) |
| Check structural comparability of CSVs | [`shape`](https://github.com/cmdrvl/shape) |
| Explain numeric changes between CSVs | [`rvl`](https://github.com/cmdrvl/rvl) |
| Bundle into immutable evidence packs | [`pack`](https://github.com/cmdrvl/pack) |

`hash` only answers: **what is the exact byte identity of each artifact?**

---

## The Three Outcomes

`hash` emits exactly one domain outcome.

### 1. ALL_HASHED (exit `0`)

Every input record was successfully hashed. No skipped records in the output.

```bash
$ vacuum /data/dec | hash
# exit 0 — all files hashed
```

### 2. PARTIAL (exit `1`)

At least one record has `_skipped: true` — either passed through from upstream or newly skipped because `hash` couldn't read the file. Remaining records are hashed normally. The output is valid but incomplete.

```bash
$ vacuum /data/dec | hash
# exit 1 — some files couldn't be hashed
# check: jq 'select(._skipped == true)' to see which
```

### 3. REFUSAL (exit `2`)

Input stream is invalid — not valid JSONL, missing required fields, or I/O error on stdin/stdout.

```json
{
  "code": "E_BAD_INPUT",
  "message": "Invalid JSONL on line 3",
  "detail": { "line": 3, "error": "expected value at line 1 column 1" },
  "next_command": null
}
```

---

## How hash Compares

| Capability | hash | `shasum` / `sha256sum` | `b3sum` | Custom script |
|------------|------|------------------------|---------|---------------|
| Streaming JSONL pipeline | Yes | No | No | You write it |
| Parallel with deterministic order | Yes | No | Yes | You write it |
| SHA-256 + BLAKE3 in one tool | Yes | SHA only | BLAKE3 only | You write it |
| Skipped file tracking | Yes (with warnings) | Fails | Fails | You write it |
| Upstream field passthrough | Yes | No | No | You write it |
| Tool version accumulation | Yes | No | No | No |
| Audit trail (witness ledger) | Yes | No | No | No |
| Constant memory (streaming) | Yes | Yes | Yes | Depends |

**When to use hash:**
- Middle of the epistemic pipeline — add byte identity between `vacuum` and `lock`
- Deduplication and caching — detect identical files by hash
- Integrity verification — prove file contents haven't changed

**When hash might not be ideal:**
- You just need a quick hash of one file — use `shasum` or `b3sum` directly
- You need content-aware hashing (e.g., ignoring whitespace) — use `fingerprint` content hashes
- You need hash trees or verified streaming — deferred in v0

---

## Installation

### Homebrew (Recommended)

```bash
brew install cmdrvl/tap/hash
```

### Shell Script

```bash
curl -fsSL https://raw.githubusercontent.com/cmdrvl/hash/main/scripts/install.sh | bash
```

### From Source

```bash
cargo build --release
./target/release/hash --help
```

---

## CLI Reference

```bash
hash [<INPUT>] [OPTIONS]
hash witness <query|last|count> [OPTIONS]
```

### Arguments

- `[INPUT]`: JSONL manifest file. Defaults to stdin.

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--algorithm <ALG>` | string | `sha256` | Hash algorithm: `sha256` or `blake3` (case-insensitive) |
| `--jobs <N>` | integer | CPU count | Parallel workers; `--jobs 1` for sequential |
| `--no-witness` | flag | `false` | Suppress witness ledger recording |
| `--describe` | flag | `false` | Print compiled `operator.json` to stdout, exit `0` |
| `--schema` | flag | `false` | Print JSON Schema to stdout, exit `0` |
| `--progress` | flag | `false` | Emit structured progress JSONL to stderr |
| `--version` | flag | `false` | Print `hash <semver>` to stdout, exit `0` |

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | ALL_HASHED (every record hashed successfully) |
| `1` | PARTIAL (some records skipped) |
| `2` | REFUSAL or CLI error |

### Streams

- `stdout`: enriched JSONL records (one per input record)
- `stderr`: progress diagnostics (with `--progress`) or warnings

---

## Input / Output Contract

### Input

JSONL records from `vacuum` (or any tool producing `vacuum.v0` records). Required fields:

- `path` — absolute file path (used to open and read the file)
- `version` — upstream record version

### Output Record

Each input record is enriched with hash fields:

```json
{
  "version": "hash.v0",
  "path": "/data/dec/tape.csv",
  "relative_path": "tape.csv",
  "root": "/data/dec",
  "size": 847201,
  "mtime": "2025-12-15T08:30:00.000Z",
  "extension": ".csv",
  "mime_guess": "text/csv",
  "bytes_hash": "sha256:7d865e959b2466918c9863afca942d0fb7903eb3",
  "hash_algorithm": "sha256",
  "tool_versions": { "vacuum": "0.1.0", "hash": "0.1.0" }
}
```

| Added Field | Type | Description |
|-------------|------|-------------|
| `bytes_hash` | string | `<algorithm>:<lowercase-hex>` (null if `_skipped`) |
| `hash_algorithm` | string | `"sha256"` or `"blake3"` (null if `_skipped`) |

All upstream fields are passed through. `version` is updated to `"hash.v0"`. `tool_versions` is merged with `hash` added.

### Hash Formats

- **SHA-256**: `sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` (64 hex chars)
- **BLAKE3**: `blake3:af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262` (64 hex chars)

### Skipped Record Handling

- **Upstream `_skipped`**: Passed through unchanged — hash does NOT attempt to hash, does NOT modify `_warnings`, DOES update `version` and `tool_versions`
- **New skip**: If hash can't read a file, it marks `_skipped: true` and appends a warning:

```json
{
  "_skipped": true,
  "_warnings": [
    { "tool": "hash", "code": "E_FILE_READ", "message": "Cannot read file", "detail": { "path": "/data/dec/locked.xlsx", "error": "Permission denied" } }
  ]
}
```

---

## Refusal Codes

| Code | Trigger | Next Step |
|------|---------|-----------|
| `E_BAD_INPUT` | Not valid JSONL or missing required fields | Check upstream output (run `vacuum` first) |
| `E_IO` | Cannot read input/output stream | Check stdin/stdout and filesystem |

---

## Troubleshooting

### "E_BAD_INPUT" — invalid input

You're piping something that isn't valid JSONL. Most common cause: piping raw file paths instead of vacuum output.

```bash
# Wrong:
echo "/data/tape.csv" | hash

# Right:
vacuum /data | hash
```

### Some files show `_skipped: true`

hash couldn't read the file (permission denied, file deleted between vacuum and hash). Check the `_warnings` array:

```bash
vacuum /data | hash | jq 'select(._skipped == true) | {path, _warnings}'
```

### Different hashes with `--jobs 1` vs `--jobs 4`

This should not happen — hash guarantees deterministic output order regardless of parallelism. If you see different hashes, the files may have changed between runs.

### BLAKE3 vs SHA-256 — which to use?

SHA-256 is the default and most widely verified. BLAKE3 is faster (especially on large files) but produces different hashes. Choose one and stick with it — `lock` records `hash_algorithm` so downstream tools know which was used.

### hash seems slow on many small files

Parallel workers have overhead per file. For many small files, try `--jobs 1` to reduce scheduling overhead. For large files, more `--jobs` helps.

---

## Limitations

| Limitation | Detail |
|------------|--------|
| **Whole-file hashing only** | No range hashing or content-aware hashing — use `fingerprint` content hashes for that |
| **No hash trees** | No Merkle tree or `bao` verified streaming — deferred in v0 |
| **No XXH3** | Non-cryptographic fast hashing not available — deferred in v0 |
| **In-memory record buffering** | Output ordering requires buffering; not true streaming for very large manifests |
| **No hash verification** | hash computes hashes — it doesn't verify stored hashes against current files |
| **No dedup output** | hash reports hashes but doesn't flag duplicates — do that downstream |

---

## FAQ

### Why not just use `shasum`?

`shasum` produces unstructured text (`hash  filename`). hash produces structured JSONL that preserves all upstream fields, accumulates tool versions, tracks skipped files, and pipes directly into `fingerprint` and `lock`. It's also parallel by default.

### Why is SHA-256 the default instead of BLAKE3?

SHA-256 is ubiquitous — auditors, compliance teams, and most tooling expect it. BLAKE3 is available via `--algorithm blake3` when speed matters more than compatibility.

### Does hash read entire files into memory?

No. hash uses a 64 KB streaming buffer — memory usage is constant regardless of file size. A 100 GB file uses the same memory as a 1 KB file.

### Why does output order match input order with parallel jobs?

Determinism. Each record gets a sequence number. Workers hash in parallel, but the output thread reorders results by sequence number before emitting. Same input always produces the same output.

### Can I hash files without vacuum?

Yes — any JSONL with a `path` field works. But vacuum provides the standard record format that downstream tools expect.

### What happens to upstream `_skipped` records?

hash passes them through without attempting to hash. It updates `version` to `hash.v0` and merges `tool_versions`, but leaves `_skipped`, `_warnings`, and other fields untouched.

### Is the empty file hash stable?

Yes. SHA-256 of zero bytes is always `sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. BLAKE3 of zero bytes is always `blake3:af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262`.

---

## Agent / CI Integration

### Self-describing contract

```bash
$ hash --describe | jq '.exit_codes'
{
  "0": { "meaning": "ALL_HASHED" },
  "1": { "meaning": "PARTIAL" },
  "2": { "meaning": "REFUSAL" }
}

$ hash --describe | jq '.pipeline'
{
  "upstream": ["vacuum"],
  "downstream": ["fingerprint", "lock", "pack"]
}
```

### Agent workflow

```bash
# 1. Hash all artifacts
vacuum /data/dec | hash > hashed.jsonl

case $? in
  0) echo "all hashed" ;;
  1) echo "partial — some files couldn't be hashed"
     jq -s '[.[] | select(._skipped == true)] | length' hashed.jsonl ;;
  2) echo "refusal"
     cat hashed.jsonl | jq '.code'
     exit 1 ;;
esac

# 2. Continue pipeline
cat hashed.jsonl | lock --dataset-id "dec" > dec.lock.json
```

### What makes this agent-friendly

- **Exit codes** — `0`/`1`/`2` map to complete/partial/error branching
- **Structured JSONL only** — stdout is always machine-readable
- **`--describe`** — prints `operator.json` so an agent discovers the tool without reading docs
- **`--schema`** — prints the record JSON schema for programmatic validation
- **Deterministic** — same input always produces the same output, enabling reliable caching

---

<details>
<summary><strong>Witness Subcommands</strong></summary>

`hash` records every run to an ambient witness ledger. You can query this ledger:

```bash
# Query by date range or outcome
hash witness query --tool hash --since 2026-01-01 --outcome ALL_HASHED --json

# Get the most recent run
hash witness last --json

# Count runs matching a filter
hash witness count --since 2026-02-01
```

### Subcommand Reference

```bash
hash witness query [--tool <name>] [--since <iso8601>] [--until <iso8601>] \
  [--outcome <ALL_HASHED|PARTIAL|REFUSAL>] [--input-hash <substring>] \
  [--limit <n>] [--json]

hash witness last [--json]

hash witness count [--tool <name>] [--since <iso8601>] [--until <iso8601>] \
  [--outcome <ALL_HASHED|PARTIAL|REFUSAL>] [--input-hash <substring>] [--json]
```

### Exit Codes (witness subcommands)

| Code | Meaning |
|------|---------|
| `0` | One or more matching records returned |
| `1` | No matches (or empty ledger for `last`) |
| `2` | CLI parse error or witness internal error |

### Ledger Location

- Default: `~/.epistemic/witness.jsonl`
- Override: set `EPISTEMIC_WITNESS` environment variable
- Malformed ledger lines are skipped; valid lines continue to be processed.

</details>

---

## Spec and Development

The full specification is [`docs/PLAN.md`](./docs/PLAN.md). This README covers intended v0 behavior; the spec adds implementation details, edge-case definitions, and testing requirements.

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
