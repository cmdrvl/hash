# hash — Byte Identity

## One-line promise

**Compute exact byte identity for every artifact in a manifest.**

Anchors deduplication, caching, and immutability. If two files have the same hash, they are the same bytes.

Second promise: **Name every file by its content, not its path.**

---

## Problem (clearly understood)

You have a manifest of files (from `vacuum`). Before you can fingerprint, lock, or compare them, you need to know which files are byte-identical and which have changed. Today this means:

- `sha256sum` in shell loops
- Inconsistent algorithm choices across tools
- No structured output tied to the manifest
- Silent failures when a file can't be read
- No evidence of which algorithm was used

`hash` replaces that with **a single streaming enrichment step** that adds content-addressed identity to every manifest record.

---

## Non-goals (explicit)

`hash` is NOT:

- A template recognizer (that's `fingerprint`)
- A content differ (that's `rvl` / `compare`)
- A lockfile generator (that's `lock`)
- A deduplication tool (it computes identity; dedup is a policy decision)

It does not tell you *what's in* a file.
It tells you *exactly which bytes the file contains*, expressed as a cryptographic hash.

---

## Relationship to the pipeline

`hash` is the second tool in the stream pipeline. It reads vacuum's JSONL, adds `bytes_hash`, and emits enriched JSONL:

```bash
vacuum /data/2025-12/ | hash | fingerprint --fp argus-model.v1 | lock --dataset-id "dec"
```

hash can also be used standalone on an existing manifest:

```bash
hash manifest.jsonl > hashed.jsonl
```

Or piped directly from vacuum for a lightweight lock (no fingerprinting):

```bash
vacuum /data/2025-12/ | hash | lock --dataset-id "raw-dec" > raw.lock.json
```

---

## CLI (v0)

```bash
hash [<INPUT>] [OPTIONS]
hash witness <query|last|count> [OPTIONS]
```

### Arguments

- `[INPUT]`: JSONL manifest file (default: stdin). When omitted, reads from stdin (for pipe composition).

### Flags (v0.1 — core)

- `--algorithm <ALG>`: Hash algorithm. Accepted values: `sha256` (default), `blake3`. Case-insensitive.
- `--jobs <N>`: Number of parallel hashing workers (default: number of available CPUs). `--jobs 1` for sequential processing.
- `--no-witness`: Suppress witness ledger recording for this run.
- `--describe`: Print the compiled-in `operator.json` to stdout and exit 0.
- `--schema`: Print the JSON Schema for the JSONL record to stdout and exit 0.
- `--progress`: Emit structured progress JSONL to stderr (see Progress reporting).
- `--version`: Print `hash <semver>` to stdout and exit 0.

### Exit codes

- `0`: All records hashed successfully.
- `1`: Partial success — one or more records marked `_skipped: true` with warnings recorded in-stream. The remaining records were hashed successfully.
- `2`: Refusal / CLI error.

### Streams

- Enriched JSONL records to stdout (always structured).
- Progress (when `--progress`): structured JSONL to stderr.
- Warnings (without `--progress`): unstructured one-per-line to stderr.

### Witness ledger (epistemic spine parity)

Same protocol as `vacuum`, `rvl`, and `shape`:

- Default: append one `witness.v0` record per run.
- Opt-out: `--no-witness`.
- Path: `EPISTEMIC_WITNESS` env var → `~/.epistemic/witness.jsonl`.
- Failures never change exit code.

Witness query subcommands (same shape as rvl/shape/vacuum):

```bash
hash witness query [--tool <name>] [--since <iso8601>] [--until <iso8601>] \
  [--outcome <ALL_HASHED|PARTIAL|REFUSAL>] [--input-hash <substring>] \
  [--limit <n>] [--json]

hash witness last [--json]
hash witness count [--tool <name>] [--since <iso8601>] [--until <iso8601>] \
  [--outcome <ALL_HASHED|PARTIAL|REFUSAL>] [--input-hash <substring>] [--json]
```

---

## Outcomes

### Exit 0: ALL_HASHED

Every input record was successfully hashed. All output records have `bytes_hash` populated.

### Exit 1: PARTIAL

One or more records could not be hashed (file not found, permission denied, IO error). Those records are emitted with `_skipped: true` and `_warnings`. All other records are hashed normally.

### Exit 2: REFUSAL

The input stream itself is invalid (not JSONL, missing required fields) or a pipeline-level error prevents any processing. Always includes a concrete next step.

---

## Output Record Schema (`hash.v0`)

Each record passes through all upstream fields and adds hash fields:

```json
{
  "version": "hash.v0",
  "path": "/data/2025-12/tape.csv",
  "relative_path": "tape.csv",
  "root": "/data/2025-12",
  "size": 48291,
  "mtime": "2025-12-31T12:00:00.000Z",
  "extension": ".csv",
  "mime_guess": "text/csv",
  "bytes_hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "hash_algorithm": "sha256",
  "tool_versions": { "vacuum": "0.1.0", "hash": "0.1.0" }
}
```

### Field definitions

| Field | Type | Nullable | Notes |
|-------|------|----------|-------|
| `version` | string | no | `"hash.v0"` (replaces upstream `version`) |
| `bytes_hash` | string | yes | `"<algorithm>:<hex>"`; `null` only when `_skipped: true` |
| `hash_algorithm` | string | yes | `"sha256"` or `"blake3"`; `null` only when `_skipped: true` |
| `tool_versions` | object | no | Merged: upstream versions + `{ "hash": "<semver>" }` |
| `_skipped` | bool | yes | Present and `true` when this record could not be hashed |
| `_warnings` | array | yes | Array of warning objects; may be empty or inherited from upstream |

All upstream fields (`path`, `relative_path`, `root`, `size`, `mtime`, `extension`, `mime_guess`) pass through unchanged. Only `version` and `tool_versions` are updated.

### Hash format

The `bytes_hash` value uses the format `<algorithm>:<lowercase-hex>`:

- SHA-256: `sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` (64 hex chars)
- BLAKE3: `blake3:af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262` (64 hex chars)

### Passthrough of upstream `_skipped` records

If an input record already has `_skipped: true` (set by vacuum), hash passes it through unchanged:
- Does NOT attempt to hash the file
- Does NOT modify `_skipped` or `_warnings`
- Does NOT add `bytes_hash` or `hash_algorithm`
- DOES update `version` to `"hash.v0"` and merge `tool_versions`

### New `_skipped` records from hash

When hash encounters a file it cannot read (file deleted since vacuum ran, permission change, IO error), it emits:

```json
{
  "version": "hash.v0",
  "path": "/data/2025-12/gone.csv",
  "relative_path": "gone.csv",
  "root": "/data/2025-12",
  "size": 48291,
  "mtime": "2025-12-31T12:00:00.000Z",
  "extension": ".csv",
  "mime_guess": "text/csv",
  "bytes_hash": null,
  "hash_algorithm": null,
  "_skipped": true,
  "_warnings": [
    { "tool": "hash", "code": "E_IO", "message": "Cannot read file", "detail": { "path": "/data/2025-12/gone.csv", "error": "No such file or directory" } }
  ],
  "tool_versions": { "vacuum": "0.1.0", "hash": "0.1.0" }
}
```

`_warnings` is an array so multiple tools can append. If upstream already had warnings, hash's warning is appended to the existing array.

### Ordering

Output order always matches input order. When processing records in parallel (`--jobs > 1`), records are buffered and emitted in their original input sequence regardless of which finishes first.

### tool_versions accumulation

hash reads `tool_versions` from each input record and merges in its own version:

```rust
// Input: { "vacuum": "0.1.0" }
// Output: { "vacuum": "0.1.0", "hash": "0.1.0" }
record.tool_versions.insert("hash".to_string(), env!("CARGO_PKG_VERSION").to_string());
```

This accumulation is how downstream tools (`fingerprint`, `lock`) know which tool versions processed each record without spawning subprocesses.

---

## Refusal Codes

| Code | Trigger | Next step |
|------|---------|-----------|
| `E_BAD_INPUT` | Input is not valid JSONL or missing required fields (`path`) | Check upstream tool output |
| `E_IO` | Cannot read input stream or cannot write output stream | Check pipeline / filesystem |

Per-file hashing failures are NOT refusals. They are recorded as `_skipped: true` records with `_warnings` (typically with `code: "E_IO"`) and cause exit code `1` (partial). Refusals are reserved for pipeline-level inability to operate.

Refusal envelope (same as all spine tools):

```json
{
  "code": "E_BAD_INPUT",
  "message": "Input is not valid JSONL",
  "detail": { "line": 42, "error": "expected value at line 1 column 1" },
  "next_command": null
}
```

### Refusal detail schemas

```
E_BAD_INPUT:
  { "line": 42, "error": "..." }
  or
  { "line": 1, "missing_field": "path" }

E_IO:
  { "error": "Broken pipe" }
```

---

## Progress Reporting (`--progress`)

When `--progress` is provided, hash emits structured JSONL to stderr:

```jsonl
{"type": "progress", "tool": "hash", "processed": 500, "total": 10000, "percent": 5.0, "elapsed_ms": 1200}
{"type": "progress", "tool": "hash", "processed": 1000, "total": 10000, "percent": 10.0, "elapsed_ms": 2400}
{"type": "warning", "tool": "hash", "path": "/data/gone.csv", "message": "skipped: No such file or directory"}
```

- `total` is the count of input records seen so far (may increase as more input arrives from stdin pipe; becomes final after input EOF).
- Progress records are emitted at regular intervals (every 100 files or every 500ms, whichever comes first).

---

## Implementation Notes

### Execution flow

```
 1. Parse CLI args (clap)               → exit 2 on bad args; --version handled by clap
 2. If --describe: print operator.json to stdout, exit 0
 3. If --schema: print JSON Schema to stdout, exit 0
 4. Open input (file or stdin)
 5. For each JSONL line:
    a. Parse as JSON                     → E_BAD_INPUT if not valid JSON (STOP)
    b. Extract required fields (path)    → E_BAD_INPUT if missing (STOP)
    c. If _skipped: true, pass through   → update version + tool_versions only
    d. Read file at `path`               → if fail: mark _skipped, append _warning, continue
    e. Hash file bytes (streaming)       → set bytes_hash, hash_algorithm
    f. Update version, merge tool_versions
    g. Serialize and emit to stdout
 6. Track: any _skipped records? → exit 1 if yes, exit 0 if all clean
 7. Append witness record (if not --no-witness)
 8. Exit
```

### Streaming hash computation

Files are hashed without loading entirely into memory. Both SHA-256 and BLAKE3 support streaming:

```rust
fn hash_file(path: &Path, algorithm: Algorithm) -> Result<String, io::Error> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);  // 64 KB buffer

    match algorithm {
        Algorithm::Sha256 => {
            let mut hasher = Sha256::new();
            loop {
                let buf = reader.fill_buf()?;
                if buf.is_empty() { break; }
                hasher.update(buf);
                let len = buf.len();
                reader.consume(len);
            }
            Ok(format!("sha256:{:x}", hasher.finalize()))
        }
        Algorithm::Blake3 => {
            let mut hasher = blake3::Hasher::new();
            loop {
                let buf = reader.fill_buf()?;
                if buf.is_empty() { break; }
                hasher.update(buf);
                let len = buf.len();
                reader.consume(len);
            }
            Ok(format!("blake3:{}", hasher.finalize().to_hex()))
        }
    }
}
```

### Parallel hashing

By default, hash processes records in parallel (bounded by CPU count). The parallelism model:

1. Read input records sequentially into a bounded channel (preserving order via sequence numbers).
2. Worker threads dequeue records, hash files, and send results to an output channel.
3. Output thread collects results, reorders by sequence number, and emits to stdout.

`--jobs 1` disables parallelism (sequential processing). Useful for debugging or when I/O is the bottleneck (e.g., network filesystems).

Use `rayon` or a custom channel-based pipeline. Key constraint: **output order must match input order regardless of processing order.**

### Core data structures

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    Sha256,
    Blake3,
}

impl Algorithm {
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Sha256 => "sha256",
            Self::Blake3 => "blake3",
        }
    }
}

pub struct HashResult {
    pub bytes_hash: String,      // "<algorithm>:<hex>"
    pub hash_algorithm: String,  // "sha256" or "blake3"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    AllHashed,   // exit 0
    Partial,     // exit 1
    Refusal,     // exit 2
}
```

### Module structure

```
src/
├── cli/
│   ├── args.rs          # clap derive Args struct
│   ├── algorithm.rs     # Algorithm enum, parsing from CLI string
│   ├── exit.rs          # Outcome, exit_code()
│   └── mod.rs
├── hash/
│   ├── sha256.rs        # Streaming SHA-256 computation
│   ├── blake3.rs        # Streaming BLAKE3 computation
│   ├── compute.rs       # hash_file() dispatcher
│   └── mod.rs
├── pipeline/
│   ├── reader.rs        # JSONL input reading + validation
│   ├── enricher.rs      # Record enrichment (add hash fields, update versions)
│   ├── parallel.rs      # Parallel processing with ordered output
│   └── mod.rs
├── output/
│   ├── jsonl.rs         # JSONL serialization to stdout
│   └── mod.rs
├── progress/
│   ├── reporter.rs      # Structured progress to stderr
│   └── mod.rs
├── refusal/
│   ├── codes.rs         # RefusalCode enum
│   ├── payload.rs       # RefusalPayload construction
│   └── mod.rs
├── witness/
│   ├── record.rs        # Witness record construction
│   ├── ledger.rs        # Append to witness ledger
│   ├── query.rs         # Witness query subcommands
│   └── mod.rs
├── lib.rs               # pub fn run() → Result<u8, Box<dyn Error>>
└── main.rs              # Minimal: calls hash::run(), maps to ExitCode
```

### `main.rs` (≤15 lines)

```rust
#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    let code = hash::run();
    std::process::ExitCode::from(code)
}
```

---

## Operator Manifest (`operator.json`)

```json
{
  "schema_version": "operator.v0",
  "name": "hash",
  "version": "0.1.0",
  "description": "Computes exact byte identity (SHA-256 or BLAKE3) for artifacts in a manifest",
  "repository": "https://github.com/cmdrvl/hash",
  "license": "MIT",

  "invocation": {
    "binary": "hash",
    "output_mode": "stream",
    "output_schema": "hash.v0",
    "json_flag": null
  },

  "arguments": [
    { "name": "input", "type": "file_path", "required": false, "position": 0, "description": "JSONL manifest file (default: stdin)" }
  ],

  "options": [
    { "name": "algorithm", "flag": "--algorithm", "type": "string", "default": "sha256", "description": "Hash algorithm: sha256 or blake3" },
    { "name": "jobs", "flag": "--jobs", "type": "integer", "description": "Number of parallel workers (default: CPU count)" }
  ],

  "exit_codes": {
    "0": { "meaning": "ALL_HASHED", "domain": "positive" },
    "1": { "meaning": "PARTIAL", "domain": "negative" },
    "2": { "meaning": "REFUSAL", "domain": "error" }
  },

  "refusals": [
    { "code": "E_BAD_INPUT", "message": "Input is not valid JSONL or missing required fields", "action": "escalate" },
    { "code": "E_IO", "message": "Cannot read input/output stream", "action": "escalate" }
  ],

  "capabilities": {
    "formats": ["*"],
    "profile_aware": false,
    "streaming": true
  },

  "pipeline": {
    "upstream": ["vacuum"],
    "downstream": ["fingerprint", "lock"]
  }
}
```

---

## Testing Requirements

### Fixtures

Provide test fixtures in `tests/fixtures/`:

- `simple_manifest.jsonl` — 3-5 vacuum records pointing to real test files
- `files/` — actual test files (small: 100 bytes - 10 KB each) for hashing
  - `hello.txt` — known content for golden hash verification
  - `empty.bin` — zero-byte file
  - `binary.dat` — non-text binary content
- `skipped_manifest.jsonl` — manifest with some records pointing to nonexistent files
- `upstream_skipped.jsonl` — manifest with pre-existing `_skipped: true` records from vacuum

### Test categories

- **Basic hashing tests:** SHA-256 and BLAKE3 produce correct hashes for known files
- **Empty file test:** zero-byte file produces the known empty-input hash
- **Golden hash tests:** `hello.txt` with known content produces exact expected hash
- **Algorithm selection:** `--algorithm sha256` vs `--algorithm blake3` produce different (correct) hashes
- **Passthrough tests:** all upstream fields preserved, only `version` and `tool_versions` changed
- **Upstream _skipped passthrough:** records with `_skipped: true` are passed through without hashing
- **New _skipped tests:** files that can't be read produce `_skipped` records with warnings
- **Warning accumulation:** hash appends to existing `_warnings` array from upstream
- **Ordering tests:** output order matches input order regardless of `--jobs`
- **tool_versions tests:** upstream versions preserved, hash version added
- **Parallel correctness:** `--jobs 4` produces identical output to `--jobs 1`
- **Stdin/file input:** reads from stdin when no argument, from file when provided
- **Exit code tests:** 0 for all-clean, 1 for partial, 2 for refusal
- **Refusal tests:** invalid JSONL input triggers E_BAD_INPUT
- **Witness tests:** witness record appended, `--no-witness` suppresses
- **`--describe` test:** prints valid operator.json
- **Large file test:** streaming hash doesn't load entire file into memory (verify via peak RSS)

### Golden file tests

- Hash `simple_manifest.jsonl` → compare output against golden JSONL (with known SHA-256 hashes)
- Hash same manifest with `--algorithm blake3` → compare against BLAKE3 golden file

---

## Scope: v0.1 (ship this)

### Must have

- `[INPUT]` optional positional arg (stdin default)
- `--algorithm <sha256|blake3>` flag (default sha256)
- `--jobs <N>` flag for parallelism
- Streaming hash computation (constant memory per file)
- Ordered output matching input order
- `_skipped` / `_warnings` for per-file failures
- Passthrough of upstream `_skipped` records
- `tool_versions` accumulation
- Ambient witness recording + `--no-witness`
- `hash witness <query|last|count>` subcommands
- `--version` flag
- `operator.json` + `--describe`
- Exit codes 0/1/2
- Refusal system with `E_BAD_INPUT`, `E_IO`

### Can defer

- `--schema` flag (JSON Schema output)
- `--progress` flag (structured progress)
- Size-first dedup optimization (skip hashing unique-sized files)
- XXH3 internal acceleration for dedup pre-screening
- `bao` verified streaming for large files

---

## Open Questions

*None currently blocking. Build it.*
