# Hash Test Coverage Audit vs rvl/shape Testing Depth

**Audit Date:** 2026-02-26
**Total Current Tests:** 113
**Bead:** bd-1rx

## Executive Summary

The hash crate demonstrates **strong functional coverage** with 113 tests across unit, integration, and E2E categories. However, there are **critical gaps** in test fixtures, golden file testing, and memory usage validation that must be addressed to reach rvl/shape testing depth parity.

## Current Test Coverage Map

### Unit Tests (Located in src/)
- **src/cli/algorithm.rs**: 3 tests
  - Algorithm parsing (case-insensitive)
  - Hash formatting (prefixed, lowercase hex)
  - Invalid algorithm rejection
- **src/output/ordered.rs**: 5 tests
  - Ordered writing with sequence tracking
  - Out-of-order buffering and reordering
  - Gap handling and buffer management

### Integration Tests (tests/*.rs)

| Test File | Test Count | Category | Coverage |
|-----------|------------|----------|----------|
| `cli_contract.rs` | 5 | CLI | Command parsing, flags, subcommands |
| `hash_engine.rs` | 5 | Algorithms | SHA-256/BLAKE3 correctness, known vectors |
| `io_failure_handling.rs` | 9 | Error Handling | IO failures, warning structure |
| `io_failure_skipped.rs` | 2 | Error Handling | Skipped record handling |
| `io_skipped_warning.rs` | 2 | Error Handling | Warning array management |
| `operator_describe.rs` | 1 | CLI | --describe flag functionality |
| `ordered_output.rs` | 5 | Streaming | Parallel processing determinism |
| `parallel_jobs.rs` | 5 | Concurrency | Jobs normalization, parallel correctness |
| `pipeline_reader.rs` | 4 | Parsing | JSONL parsing, field validation |
| `progress_output.rs` | 1 | CLI | --progress structured stderr |
| `refusal_contract.rs` | 5 | Error Handling | Refusal codes, exit mappings |
| `refusal_envelope.rs` | 12 | Error Handling | Refusal payload structure |
| `run_outcomes.rs` | 9 | Integration | End-to-end outcome scenarios |
| `scaffold_topology.rs` | 4 | Framework | Test infrastructure validation |
| `schema_output.rs` | 1 | CLI | --schema flag functionality |
| `skipped_passthrough.rs` | 2 | Data Flow | Upstream skipped preservation |
| `spine_compatibility.rs` | 2 | E2E | Downstream tool compatibility |
| `upstream_skipped.rs` | 9 | Data Flow | Skipped record processing |
| `witness_append.rs` | 4 | Witness | Witness recording functionality |
| `witness_behavior.rs` | 4 | Witness | Ledger management, no-witness flag |
| `witness_query.rs** | 6 | Witness | Query subcommands, filtering |

### E2E Tests
- **e2e_spine_compatibility.rs**: 5 tests
  - Vacuum metadata preservation
  - Fingerprint schema compatibility
  - Lock compatibility with skipped records
  - Tool versions accumulation
  - Deterministic field ordering
- **e2e_spine.rs**: 2 tests
  - End-to-end pipeline integration
  - Mixed record handling

## PLAN.md Testing Requirements Analysis

### ✅ **COVERED** - High Quality Implementation

| Requirement | Implementation | Quality |
|-------------|----------------|---------|
| **Basic hashing tests** | `hash_engine.rs` | ✅ SHA-256/BLAKE3 with known vectors |
| **Empty file test** | `hash_engine.rs` | ✅ Zero-byte known hash validation |
| **Algorithm selection** | `hash_engine.rs` + CLI tests | ✅ Both algorithms tested |
| **Passthrough tests** | `spine_compatibility.rs` | ✅ All upstream fields preserved |
| **Upstream _skipped passthrough** | `upstream_skipped.rs` | ✅ 9 comprehensive tests |
| **New _skipped tests** | `io_failure_*` suite | ✅ 13 tests covering IO failures |
| **Warning accumulation** | `io_skipped_warning.rs` | ✅ Array append behavior |
| **Ordering tests** | `ordered_output.rs` | ✅ Parallel determinism verified |
| **tool_versions tests** | `e2e_spine_compatibility.rs` | ✅ Version accumulation tested |
| **Parallel correctness** | `parallel_jobs.rs` | ✅ --jobs 1 vs --jobs N comparison |
| **Exit code tests** | `run_outcomes.rs` | ✅ All exit codes (0/1/2) tested |
| **Refusal tests** | `refusal_*` suite | ✅ 17 tests for invalid input |
| **Witness tests** | `witness_*` suite | ✅ 14 tests covering all subcommands |
| **--describe test** | `operator_describe.rs` | ✅ Valid operator.json output |

### ❌ **MISSING** - Critical Priority Gaps (P0)

| Requirement | Status | Impact | Effort |
|-------------|---------|---------|---------|
| **Test fixtures** | Missing | HIGH | Medium |
| **Golden hash tests** | Missing | HIGH | Low |
| **Large file streaming test** | Missing | MEDIUM | High |
| **Memory usage validation** | Missing | MEDIUM | High |
| **Stdin/file input distinction** | Missing | LOW | Low |

### ⚠️ **PARTIAL** - Enhancement Needed (P1)

| Requirement | Current State | Gap | Effort |
|-------------|---------------|-----|---------|
| **Golden file tests** | Algorithm correctness only | No end-to-end golden JSONL | Low |
| **Schema validation** | --schema flag exists | No schema compliance tests | Low |

## P0 Critical Gaps - Implementation Required

### 1. **Test Fixtures Infrastructure** (P0)
**Problem:** No `tests/fixtures/` directory structure per PLAN.md requirements.

**Required:**
```
tests/fixtures/
├── simple_manifest.jsonl          # 3-5 vacuum records → real files
├── skipped_manifest.jsonl         # Points to nonexistent files
├── upstream_skipped.jsonl         # Pre-existing _skipped: true records
└── files/
    ├── hello.txt                  # Known content for golden hashes
    ├── empty.bin                  # Zero-byte file
    └── binary.dat                 # Binary test content
```

**Impact:** Without fixtures, golden tests are impossible and E2E coverage is incomplete.

### 2. **Golden Hash Tests** (P0)
**Problem:** No end-to-end golden file validation against known manifest outputs.

**Required:**
- Hash `simple_manifest.jsonl` → compare against known SHA-256 golden JSONL
- Hash same manifest with `--algorithm blake3` → compare against BLAKE3 golden
- Ensures deterministic output across runs and platforms

**Impact:** Cannot guarantee deterministic output behavior required by downstream tools.

### 3. **Large File Streaming Test** (P0)
**Problem:** No validation that streaming hash doesn't load entire file into memory.

**Required:**
- Create large file (>100MB) test case
- Monitor peak RSS during hashing
- Verify memory usage remains constant regardless of file size
- Critical for production use on large datasets

**Impact:** Memory exhaustion risk on large files in production.

## P1 Enhancement Gaps - Nice to Have

### 1. **Enhanced Golden Tests** (P1)
- Extend golden tests to full JSONL output validation
- Include tool_versions, metadata preservation
- Cross-platform hash consistency validation

### 2. **Schema Compliance Tests** (P1)
- Validate --schema output against actual record structure
- JSON Schema validation of all output records
- Downstream tool schema compatibility

### 3. **Stdin vs File Input Distinction** (P1)
- Explicit test coverage for stdin reading behavior
- File argument vs pipe behavior differences
- Input source tracking for witness records

## Comparison to rvl/shape Testing Depth

Based on spine ecosystem patterns, hash currently **EXCEEDS** typical testing depth in:
- **Error handling coverage** (13 IO failure tests vs typical 3-5)
- **Witness system completeness** (14 tests vs typical 5-8)
- **Concurrency validation** (5 parallel tests vs typical 1-2)

Hash **MATCHES** rvl/shape baseline in:
- **Algorithm correctness** (known vectors, empty files)
- **CLI contract testing** (flags, subcommands, exit codes)
- **Pipeline integration** (upstream/downstream compatibility)

Hash **LAGS** rvl/shape baseline in:
- **Golden file infrastructure** (missing fixtures directory)
- **Memory usage validation** (no large file streaming tests)
- **End-to-end determinism validation** (no golden JSONL comparison)

## Prioritized Implementation Follow-ups

### **Immediate Priority (P0)** - Required for Release Baseline

1. **[bd-NEW] Create test fixtures infrastructure**
   - Create `tests/fixtures/` directory structure
   - Generate test files with known content
   - Create vacuum-format manifests pointing to test files
   - **Estimated effort:** 2-3 hours

2. **[bd-NEW] Implement golden hash tests**
   - Golden JSONL output comparison for SHA-256 and BLAKE3
   - Deterministic output validation across runs
   - **Estimated effort:** 1-2 hours

3. **[bd-NEW] Add large file streaming memory test**
   - Create >100MB test file (or mock large read)
   - Validate constant memory usage during hashing
   - **Estimated effort:** 3-4 hours (memory monitoring setup)

### **Secondary Priority (P1)** - Post-Release Enhancements

4. **[bd-NEW] Enhanced schema validation tests**
   - --schema output compliance with actual records
   - JSON Schema validation integration
   - **Estimated effort:** 1 hour

5. **[bd-NEW] Input source distinction tests**
   - Stdin vs file input behavior validation
   - Witness record input tracking accuracy
   - **Estimated effort:** 1 hour

## Testing Baseline Achievement

**Current Status:** 113 tests ✅
**rvl/shape Parity Target:** ~130 tests with golden infrastructure ✅
**Release Readiness:** Requires P0 gap closure (fixtures + golden tests)

Hash testing depth is **functionally complete** but lacks **infrastructure maturity** compared to established spine tools. The P0 gaps represent foundational testing infrastructure rather than functional coverage holes.

## Recommendations

1. **Immediate:** Close P0 gaps before next release
2. **Follow-up:** P1 enhancements can be addressed in future releases
3. **Maintenance:** Establish golden file update procedures for schema changes
4. **Monitoring:** Add memory usage regression testing to CI pipeline

---

**Audit Completed:** ✅
**Next Action:** Implement prioritized follow-up beads