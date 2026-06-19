# Fuzz Testing - libFuzzer, AFL++, structured fuzzing

> Let the machine discover inputs you never thought to write.

**Type:** Learn
**Languages:** C, Rust
**Prerequisites:** Phase 17 lessons 01-04
**Time:** ~90 minutes

## Learning Objectives

- Explain coverage-guided fuzzing and why it finds deep parser/state bugs.
- Differentiate mutation fuzzing and structured fuzzing.
- Build minimal fuzz targets in C and Rust-style workflows.
- Interpret crashes, reduce reproducers, and turn findings into regression tests.

## The Problem

Manual tests for parsers and binary protocols often look fine on obvious inputs.
Then production receives malformed or adversarial data:

- out-of-bounds reads
- integer overflows
- unchecked length fields
- unexpected state-machine transitions

Classic unit tests usually miss these because humans are poor at inventing weird
byte patterns. Fuzzers are good at this search.

### How bugs hide in parsers

Consider a function that parses a binary message format:

```
  Wire format: [MAGIC:2][VERSION:1][LEN:1][PAYLOAD:LEN]

  Valid:   0xABCD 0x01 0x05 "hello"
  Crasher: 0xABCD 0x01 0xFF ...  (LEN=255 but only 10 bytes follow)
```

A developer might test the valid case, maybe a short payload. But what about:

- LEN=0 with no payload bytes
- LEN=255 with only 3 payload bytes
- MAGIC bytes that are off by one
- VERSION=255 (unknown version)
- PAYLOAD containing null bytes

Each of these is a plausible input that a unit test author might forget. A fuzzer
discovers them automatically through coverage-guided mutation.

### Real-world impact

Fuzzing has found thousands of critical vulnerabilities in production software:

| Project | Bug class | Impact |
|---|---|---|
| OpenSSL (Heartbleed) | Buffer over-read | Private key leakage |
| SQLite | Use-after-free | Remote code execution |
| FFmpeg | Integer overflow | Heap corruption |
| libpng | Null pointer deref | Denial of service |
| systemd | Stack overflow | Local privilege escalation |
| Chrome V8 | Type confusion | Sandbox escape |

Most of these were found by automated fuzzers, not human testers. The inputs that
trigger them are often dozens of bytes long, with specific bit patterns no human
would guess.

### The operational gap

A second issue: teams run fuzzers but never operationalize results. Crashes are
not minimized, not triaged, and never converted to regression suites. The same
vulnerability class reappears months later.

Fuzzing is valuable only when paired with discipline: target selection, sanitizers,
reproducer minimization, and test backfill.

## The Concept

### Coverage-guided loop

The core fuzzer algorithm is a tight feedback loop:

```
  ┌──────────────────────────────────────────────────┐
  │                                                  │
  │  ┌─────────┐    ┌──────────┐    ┌────────────┐  │
  │  │  Seed   │───▶│ Mutate   │───▶│  Execute   │  │
  │  │ Corpus  │    │ Input    │    │  Target    │  │
  │  └─────────┘    └──────────┘    └─────┬──────┘  │
  │       ▲                               │         │
  │       │          ┌──────────┐         │         │
  │       └──────────│ Coverage │◀────────┘         │
  │                  │ Feedback │                   │
  │                  └────┬─────┘                   │
  │                       │                         │
  │                  ┌────▼─────┐                   │
  │                  │  New     │                   │
  │                  │ Coverage?│                   │
  │                  └────┬─────┘                   │
  │                  yes  │  no → discard           │
  │                       ▼                         │
  │                  Add to corpus                   │
  └──────────────────────────────────────────────────┘
```

1. Start with a seed corpus (small set of valid inputs).
2. Mutate inputs (bit flips, byte inserts, block operations).
3. Execute target with instrumentation (compile-time or runtime).
4. Keep inputs that reach new coverage or trigger interesting behavior.
5. Repeat at high throughput (thousands of executions per second).

Coverage feedback helps avoid blind random search. Without it, a fuzzer would
waste time on inputs that exercise the same code paths. With it, the fuzzer
is steered toward unexplored branches.

### Mutation vs structured fuzzing

Two fundamentally different approaches:

| Aspect | Mutation fuzzing | Structured fuzzing |
|---|---|---|
| Input model | Raw bytes, no format knowledge | Grammar/schema-aware |
| Mutations | Bit flips, byte inserts, cross-over | Token swaps, rule applications |
| Setup cost | Low (just point at binary) | Medium (write generator/grammar) |
| Depth | Shallow to medium | Deep semantic paths |
| Speed | Very fast (1000s/sec) | Slower (generation overhead) |
| Best for | Parsers, binary formats | Complex protocols, SQL, JSON |

Mutation fuzzing is easy and fast. You compile the target with instrumentation,
point the fuzzer at it, and go. It works surprisingly well for C/C++ code that
processes bytes directly.

Structured fuzzing penetrates deeper semantic paths. A grammar-based fuzzer for
SQL knows that `SELECT` must come before `FROM`, so it generates syntactically
valid queries and explores the semantic space (nested subqueries, unusual joins,
edge-case expressions).

### How coverage instrumentation works

At compile time, the compiler inserts counters at every basic block edge:

```c
// Original code
if (x > 0) {
    do_positive();
} else {
    do_negative();
}

// Instrumented (conceptual)
edge_count[A]++;  // entry
if (x > 0) {
    edge_count[B]++;  // true branch
    do_positive();
} else {
    edge_count[C]++;  // false branch
    do_negative();
}
```

When the fuzzer sees a new edge count (first time visiting a branch), it saves
the input. This is how it "learns" to explore deeper code paths.

AFL++ uses compile-time instrumentation via `afl-cc`. libFuzzer uses LLVM's
`SanitizerCoverage` pass. Both achieve the same goal with different mechanics.

### Why sanitizers matter

Sanitizers convert silent memory corruption into actionable crashes.

| Sanitizer | What it catches | Runtime cost |
|---|---|---|
| ASan (AddressSanitizer) | Heap/stack OOB, use-after-free, double-free | ~2x slowdown |
| UBSan (UndefinedBehavior) | Signed overflow, null deref, alignment | ~1.5x slowdown |
| MSan (MemorySanitizer) | Uninitialized memory reads | ~3x slowdown |
| TSan (ThreadSanitizer) | Data races, deadlocks | ~5-15x slowdown |

Without sanitizers, many bugs remain latent. A buffer over-read might return
garbage data that happens to pass your assertions. With ASan, that same read
crashes immediately with a stack trace pointing at the exact line.

**Always fuzz with sanitizers enabled.** The slowdown is worth it. A bug that
doesn't crash is a bug you don't find.

### Effective target selection

Good fuzz targets share these properties:

```
  Good target                    Bad target
  ┌─────────────────────┐       ┌─────────────────────┐
  │ ✓ Deterministic     │       │ ✗ Randomized output  │
  │ ✓ No side effects   │       │ ✗ Writes to disk     │
  │ ✓ Fast (< 1ms)      │       │ ✗ Network calls      │
  │ ✓ Pure function     │       │ ✗ Wall-clock timing  │
  │ ✓ High-risk entry   │       │ ✗ Global mutable     │
  └─────────────────────┘       └─────────────────────┘
```

Ideal targets: parsers, decoders, serializers, validators, hash functions,
compression algorithms, crypto primitives.

Poor targets: GUI code, network clients, database drivers (mock these first).

### Corpus strategy

The seed corpus is your starting point. Quality matters more than quantity.

**Minimal viable corpus:**

- One valid input of each type (smallest examples).
- One near-valid input (one byte off from valid).
- Edge cases: empty input, max-length input, all-zeroes.

**Corpus hygiene:**

- Keep seeds small (under 1KB if possible).
- Deduplicate similar crashes by stack hash.
- Periodically minimize corpus to remove redundant coverage.
- Version control your corpus alongside your code.

### Crash triage pipeline

When the fuzzer finds a crash, follow this workflow:

```
  Crash found
      │
      ▼
  1. Reproduce deterministically
      │  (same binary, same input, same result)
      ▼
  2. Minimize input
      │  (afl-tmin / libFuzzer -minimize_crash=1)
      ▼
  3. Classify root cause
      │  (ASan report, stack trace, code inspection)
      ▼
  4. Patch the bug
      │  (fix the root cause, not the symptom)
      ▼
  5. Add regression test + seed
      │  (minimized input becomes a test case)
      ▼
  6. Update corpus
      (add the reproducer as a seed for future runs)
```

Skipping step 5 is the most common failure mode. Teams find and fix bugs but
never add regression tests. The same bug class returns in a different function.

### Structured fuzzing examples

For complex formats, mutation alone hits walls. Structured fuzzing helps:

**JSON parser with schema-aware generation:**

```
  Grammar rules:
    value  → object | array | string | number | bool | null
    object → '{' pairs '}'
    pairs  → pair (',' pair)*
    pair   → string ':' value
    array  → '[' values ']'
    number → integer | float | NaN | Infinity | -0
```

The generator produces valid JSON but varies: deeply nested objects, arrays
with mixed types, numbers at IEEE 754 boundaries, strings with escape sequences.

**Protocol frame parser:**

```
  [HEADER:4][FLAGS:1][SEQ:2][LEN:2][PAYLOAD:LEN][CRC:4]

  Generator: valid header, random flags, random seq,
             len matching payload, correct CRC.
  Mutations: flip bits in FLAGS, corrupt CRC, truncate PAYLOAD.
```

The generator ensures structural validity while mutations explore semantic edges.

## Build It

We create simple targets that parse a tiny message format:

`MAGIC(2 bytes) | version(1) | len(1) | payload(len)`

### Step 1: deterministic parser core

Build pure function parser with explicit length checks.

```c
// parser.h
#include <stdint.h>
#include <stddef.h>

#define MAGIC 0xABCD

typedef enum {
    PARSE_OK = 0,
    PARSE_TOO_SHORT,
    PARSE_BAD_MAGIC,
    PARSE_BAD_VERSION,
    PARSE_LEN_MISMATCH,
} ParseResult;

typedef struct {
    uint16_t magic;
    uint8_t  version;
    uint8_t  len;
    uint8_t  payload[255];
} Message;

ParseResult parse_message(const uint8_t *data, size_t size, Message *out) {
    if (size < 4) return PARSE_TOO_SHORT;

    out->magic = (data[0] << 8) | data[1];
    if (out->magic != MAGIC) return PARSE_BAD_MAGIC;

    out->version = data[2];
    if (out->version > 3) return PARSE_BAD_VERSION;

    out->len = data[3];
    if (size < 4 + out->len) return PARSE_LEN_MISMATCH;

    memcpy(out->payload, data + 4, out->len);
    return PARSE_OK;
}
```

### Step 2: C harness shape (libFuzzer)

Expose a function that accepts `const uint8_t *data, size_t size` and never
performs I/O or global mutable side effects.

```c
// fuzz_target.c
#include "parser.h"
#include <stdint.h>
#include <stddef.h>

// libFuzzer calls this function repeatedly with mutated inputs
int LLVMFuzzerTestOneInput(const uint8_t *data, size_t size) {
    Message msg;
    ParseResult result = parse_message(data, size, &msg);

    if (result == PARSE_OK) {
        // Exercise code that uses parsed message
        // This is where coverage feedback matters
        process_message(&msg);
    }

    return 0;  // non-zero return is reserved
}
```

Compile and run:

```bash
clang -g -O1 -fsanitize=fuzzer,address \
    fuzz_target.c parser.c -o fuzz_target

./fuzz_target corpus/ -max_len=1024 -runs=1000000
```

### Step 3: Rust harness shape (cargo-fuzz)

Implement parser over `&[u8]` and return typed errors, not panics.

```rust
// fuzz/fuzz_targets/parse_message.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

#[derive(Debug)]
enum ParseError {
    TooShort,
    BadMagic,
    BadVersion,
    LenMismatch,
}

fn parse_message(data: &[u8]) -> Result<Message, ParseError> {
    if data.len() < 4 {
        return Err(ParseError::TooShort);
    }

    let magic = u16::from_be_bytes([data[0], data[1]]);
    if magic != 0xABCD {
        return Err(ParseError::BadMagic);
    }

    let version = data[2];
    if version > 3 {
        return Err(ParseError::BadVersion);
    }

    let len = data[3] as usize;
    if data.len() < 4 + len {
        return Err(ParseError::LenMismatch);
    }

    Ok(Message {
        magic,
        version,
        len,
        payload: data[4..4+len].to_vec(),
    })
}

fuzz_target!(|data: &[u8]| {
    let _ = parse_message(data);
});
```

Run with:

```bash
cargo fuzz run parse_message -- -max_len=1024
```

### Step 4: seed corpus design

Add tiny valid and near-valid seeds:

```
corpus/
├── valid_minimal.bin     # 0xAB CD 01 00              (no payload)
├── valid_hello.bin       # 0xAB CD 01 05 "hello"      (normal)
├── bad_magic.bin         # 0x00 01 01 00              (wrong magic)
├── bad_version.bin       # 0xAB CD FF 00              (version=255)
├── short_payload.bin     # 0xAB CD 01 FF 00          (len=255, 1 byte)
└── empty.bin             # (0 bytes)
```

Each seed targets a different code path. The fuzzer uses these as starting
points for mutation.

### Step 5: failure workflow

If a crash appears, store reproducer and add unit test in parser module.

```bash
# Minimize the crashing input
./fuzz_target crash-abc123 -minimize_crash=1 -runs=10000

# The minimized file becomes a regression test
cp minimized-crash test/regression/issue-42.bin
```

Add a unit test:

```c
// test_regression.c
void test_issue_42_crash(void) {
    uint8_t data[] = {0xAB, 0xCD, 0x01, 0xFF, 0x00};
    Message msg;
    ParseResult result = parse_message(data, sizeof(data), &msg);
    // Should return error, not crash
    assert(result == PARSE_LEN_MISMATCH);
}
```

## Use It

Production usage pattern:

- Run fast fuzz budget in CI (seconds/minutes per target).
- Run extended fuzz jobs nightly or on dedicated clusters.
- Store crash artifacts with symbolized stack traces.
- Gate release on unresolved high-severity memory-safety findings.

### libFuzzer vs AFL++ workflow differences

| Aspect | libFuzzer | AFL++ |
|---|---|---|
| Integration | In-process, linked with target | Out-of-process, fork server |
| Instrumentation | LLVM SanitizerCoverage | AFL LLVM mode or QEMU |
| Corpus format | Directory of files | Directory of files |
| Parallelism | Single process (multi-threaded) | Multiple parallel instances |
| Best for | Library functions, APIs | Full programs, file processors |
| Sanitizers | Native (same process) | Separate runs or combined |

### OSS-Fuzz: industrial-scale fuzzing

Google's OSS-Fuzz runs continuous fuzzing for 1000+ open source projects. It
combines libFuzzer with ClusterFuzz, a distributed fuzzing infrastructure.

Key lessons from OSS-Fuzz:

- Fuzz targets should be fast (< 1ms per iteration).
- Corpus should be version-controlled.
- New code should get new fuzz targets (coverage tracking).
- Crash deduplication by stack hash prevents duplicate reports.

### CI integration pattern

```yaml
fuzz-ci:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Install cargo-fuzz
      run: cargo install cargo-fuzz
    - name: Run fuzz targets (fast)
      run: |
        for target in $(cargo fuzz list); do
          cargo fuzz run $target -- -max_total_time=60
        done
    - name: Upload crash artifacts
      if: failure()
      uses: actions/upload-artifact@v4
      with:
        name: fuzz-crashes
        path: fuzz/artifacts/
```

## Read the Source

- LLVM libFuzzer docs and examples for harness conventions.
- AFL++ docs for corpus management and fork-server execution model.
- Rust ecosystem fuzzing guides (`cargo-fuzz`) for sanitizer-backed targets.
- OSS-Fuzz project templates for production fuzzing infrastructure.

## Ship It

This lesson ships:

- `code/main.c`: tiny parser + fuzz-style entry function.
- `code/main.rs`: same parser model in Rust.
- `outputs/README.md`: fuzz campaign checklist and triage template.

## Exercises

1. **Easy** - Add a new field (checksum) to the message format and fuzz mismatch
   cases. Does the fuzzer find the checksum validation bug?
2. **Medium** - Build a grammar-based generator for a command protocol with
   nested subcommands. Compare coverage depth vs mutation-only fuzzing.
3. **Hard** - Integrate AddressSanitizer into your CI pipeline. Run fuzz targets
   for 60 seconds per commit and fail the build on new crashes.
4. **Hard** - Implement crash deduplication by stack hash. Group crash artifacts
   and report unique bug counts instead of raw crash counts.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Coverage-guided fuzzing | "smart random testing" | Mutation search steered by execution coverage signal |
| Seed corpus | "sample files" | Initial inputs that bootstrap mutation exploration |
| Sanitizer | "debug build option" | Runtime instrumentation exposing memory/UB defects |
| Reproducer | "crash file" | Minimal input that deterministically triggers a fault |
| Structured fuzzing | "grammar fuzzing" | Input generation constrained by known format structure |
| Harness | "test wrapper" | Deterministic function boundary fuzzers invoke repeatedly |
| Corpus minimization | "cleanup" | Removing redundant seeds while preserving coverage |
| Crash deduplication | "grouping" | Clustering failures by root-cause signature |
| Fork server | "process reuse" | Persistent process that forks for each input, avoiding startup cost |
| Power scheduling | "smart mutation" | Allocating more mutation cycles to high-coverage inputs |

## Further Reading

- [libFuzzer Documentation](https://llvm.org/docs/LibFuzzer.html) - core concepts and target patterns.
- [AFL++ Documentation](https://aflplus.plus/docs/) - advanced mutation and operational guidance.
- [OSS-Fuzz](https://google.github.io/oss-fuzz/) - industrial-scale continuous fuzzing model.
- [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) - Rust integration workflow.
- [ClusterFuzz](https://google.github.io/clusterfuzz/) - distributed fuzzing infrastructure.
- [Fuzzing Book](https://www.fuzzingbook.org/) - comprehensive academic resource on fuzzing techniques.
