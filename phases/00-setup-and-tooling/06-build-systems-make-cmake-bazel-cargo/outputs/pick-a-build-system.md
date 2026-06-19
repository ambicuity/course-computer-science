# Pick a Build System — Decision Tree

Six questions, one answer.

```
Q1. Is the project all-Rust?
    → YES: use Cargo. Stop reading.

Q2. Is it a polyglot monorepo (multiple languages, shared deps)?
    → YES: use Bazel (or Buck2). Distributed cache pays off above ~50 engineers.

Q3. Is it C/C++ and you need cross-platform (Windows, macOS, Linux, embedded)?
    → YES: use CMake (+ Ninja). Generate compile_commands.json for editor LSPs.

Q4. Is it C/C++, single platform, < ~50 files?
    → YES: use Make.  Add -MMD -MP dependency tracking on day one.

Q5. Is it Python / Node / Go?
    → YES: use the language's native tooling (uv / npm / go). Don't fight it.

Q6. Is it a research / one-off?
    → YES: a Makefile or a shell script. Don't introduce CMake or Bazel for code
           that won't outlive the month.
```

## Anti-patterns to avoid

| Smell | Why it bites you later |
|-------|------------------------|
| Hand-rolled shell script as build system | No incremental rebuilds; one bug touches every artifact |
| Make without `-MMD -MP` | Stale binaries because header edits don't trigger rebuilds |
| CMake without `target_*` (still using `include_directories(...)` globally) | Symbols leak across targets; bigger projects can't isolate libs |
| Bazel before you have ≥ 5 engineers | The hermetic-build benefit doesn't outweigh the BUILD-file authoring cost |
| Mixing Cargo with another build system at the same level | The two will disagree about feature flags and rebuild logic; pick one as primary |

## Reproducibility checklist

If your build claims to be reproducible, verify:

1. Same source + same toolchain version + same flags = byte-identical binary on two machines.
2. Build doesn't read environment variables that aren't declared as inputs.
3. Build doesn't read system paths (`/usr/include`, `/opt/...`) that aren't declared.
4. Build doesn't depend on the current working directory (`$PWD`).
5. Build doesn't depend on the file system mtime — only on content.

If any of those fail, you have a non-reproducible build, regardless of which system you picked.
