# Build Systems — Make, CMake, Bazel, Cargo

> Every build system answers the same question: "given a graph of files, what's the minimum work to bring the outputs up to date?" The differences are in how strict, how hermetic, and how language-specific they are.

**Type:** Learn
**Languages:** Makefile, Shell
**Prerequisites:** Phase 00, Lessons 01–05
**Time:** ~75 minutes

## Learning Objectives

- Place Make, CMake, Bazel, and Cargo on the spectrum from "shell scripts with timestamps" to "hermetic distributed graph execution," and pick the right one for a project.
- Describe what a *clean build* and an *incremental build* mean in each system, and what each system uses to decide a target is up to date.
- Read a `Makefile`, a `CMakeLists.txt`, and a `BUILD.bazel` file for the same toy project and trace which files become which artifacts.
- Recognize "build flakiness" symptoms — stale outputs, missing rebuilds, non-reproducible artifacts — and connect each to the underlying model that allowed it.

## The Problem

In Lesson 04 you wrote a `Makefile` by hand and watched it skip work on rebuilds. That was a small project. Real projects are not small. Linux has 80,000 source files. Google's monorepo has billions of lines. Chrome takes nine hours to build from scratch on a laptop.

At those scales, "is this output up to date?" becomes the central engineering question. Get it wrong in one direction and your build takes hours longer than it should. Get it wrong in the other direction and a buggy artifact ships because the system thought it was current.

Different build systems made different bets about how to answer that question. None of them are universally right. This lesson is the map: when each one makes sense, where each one breaks down, and what the underlying primitive is in every case (a DAG, a key, a verifier).

## The Concept

### The shared model: targets, prerequisites, recipes

Every build system, deep down, looks like this:

```
        ┌──────────┐
        │ target   │  ← what to build
        └────┬─────┘
             │ depends on
        ┌────▼────────┐
        │ prereqs     │  ← inputs that the target is a function of
        └────┬────────┘
             │ when out of date, run
        ┌────▼────────┐
        │ recipe      │  ← the command that produces the target
        └─────────────┘
```

A target is *out of date* if any prereq is newer than it (Make), or if the hash of all prereqs has changed (Bazel), or if a cached fingerprint mismatches (Cargo). That's the only difference between the four systems we'll survey.

### The four contenders

| System | Decides "out of date" by | Hermetic? | Language scope | Typical use |
|--------|--------------------------|-----------|----------------|-------------|
| **Make**   | mtime (file modification time) | No  | Any language | Small/medium C projects, ad hoc tasks |
| **CMake**  | mtime + a generator (it produces a Makefile or Ninja file) | No | C/C++ + others via toolchains | Most cross-platform C/C++ |
| **Bazel**  | Content hashes of inputs + exact command line | Yes (sandboxes each action) | Polyglot, with rules per language | Large monorepos, distributed builds |
| **Cargo**  | A fingerprint over source + Cargo.lock + features + rustc version | Mostly | Rust + build scripts | Every Rust project |

**Hermetic** means: the build doesn't depend on the state of the developer's machine (system headers, env vars, installed compilers). Same inputs → same output, byte for byte, on any machine. Make and CMake are *not* hermetic by default — Bazel is hermetic by design.

### Make: timestamps and tab characters

Make is forty years old, written for C, and the model is "mtime-based dependency tracking." Pros: tiny, available everywhere, easy to teach. Cons: no header dependency tracking by default (Lesson 04, Exercise 3), `cp` and `clock skew` and NFS can fool it, and any command in a recipe can do anything (untracked side effects).

### CMake: the meta-build system

CMake isn't itself a builder. It's a *generator* that reads `CMakeLists.txt` and emits a real build file (Make, Ninja, MSBuild, Xcode project). The pay-off: one description, many native builds; great cross-platform story; standard `find_package(...)` for finding system libraries. The cost: CMake is its own language, and that language has historical quirks (variable scoping, list semantics) that occasionally bite.

```
CMakeLists.txt ──[ cmake ]──> build.ninja  ──[ ninja ]──> artifacts
```

Ninja is what CMake almost always targets nowadays — same model as Make but built for parallelism and tight dependency graphs.

### Bazel: hermetic, content-addressed, distributed

Bazel (open-source descendant of Google's "Blaze") treats every build action as a pure function from inputs to outputs. Inputs are hashed; the system caches outputs by input hash. Hermetic sandboxing means an action only sees the files it declared as inputs. The pay-off: builds are *correct* (no false hits, no missed rebuilds) and *distributed* (the cache is shared across the whole org). The cost: every rule has to be declared explicitly. No "scan the system for openssl" shortcuts.

```
input hashes ──[ key ]──> remote cache ──> artifact
                       ↓ on miss
                    sandbox + run recipe ──> upload to cache
```

### Cargo: convention over configuration, for one language

Cargo is the simplest *to use* of the four because it works only for one language (Rust) and assumes a fixed project layout. There's almost no configuration — just a `Cargo.toml` and a `src/` directory. Cargo tracks a fingerprint (a hash of source + `Cargo.lock` + features + rustc version) and rebuilds when it changes. It's not fully hermetic (build scripts can read env vars), but in practice it's reproducible enough that most teams never notice.

### When to pick which

| Situation | Pick |
|-----------|------|
| Single C/C++ project, < 50 files, no cross-platform needs | Make |
| Cross-platform C/C++ library, multiple build environments | CMake (+ Ninja) |
| Polyglot monorepo, want reproducibility across the org | Bazel |
| A Rust project | Cargo (don't second-guess this) |
| A Python or Node project | Their own tooling (uv, npm) — but borrow Bazel for polyglot orgs |

Real teams sometimes use two: Cargo for the Rust parts, plus Bazel rules orchestrating across the rest of a monorepo.

## Build It

The `code/` folder has the same tiny C project from Lesson 04 (`hello.c` + `greet.c`), set up to be built four ways. We'll walk each.

### Step 1: Make

```sh
cd code/
make            # uses code/Makefile (same one from Lesson 04)
./hello
make clean
```

Look at `code/Makefile`. The dependency graph is *implicit* — `make` infers it from prereq lists. Edit `greet.h`, run `make`, and notice it does NOT rebuild — because `greet.h` isn't a listed prereq. Lesson 04 Exercise 3 fixed this with `-MMD -MP`. Real projects use that or graduate to one of the other systems.

### Step 2: CMake (generate a Ninja build)

`code/CMakeLists.txt`:

```cmake
cmake_minimum_required(VERSION 3.15)
project(hello LANGUAGES C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

add_library(greet STATIC greet.c)
target_include_directories(greet PUBLIC .)

add_executable(hello hello.c)
target_link_libraries(hello PRIVATE greet)

enable_testing()
add_test(NAME hello_runs COMMAND hello)
```

Build:

```sh
cmake -S . -B build -G Ninja
cmake --build build
./build/hello
ctest --test-dir build
```

`cmake -S . -B build` says "source in `.`, write build files to `build/`." `-G Ninja` picks the Ninja generator (faster, parallel-friendly). `cmake --build build` is a portable invocation of whatever generator was used. CMake handles cross-platform paths, parallelism, library discovery (`find_package`), and produces `compile_commands.json` for editors.

### Step 3: Cargo (Rust translation of the same logic)

`code/cargo-version/Cargo.toml` and `code/cargo-version/src/main.rs` re-implement the same hello/greet pattern in Rust. Build:

```sh
cd code/cargo-version
cargo run
```

Total config: 6 lines of TOML. No prereqs, no recipe, no Makefile.

### Step 4: Bazel (a sketch)

For brevity we won't run a full Bazel build (it requires a separate install). The `code/BUILD.bazel` file shows what it would look like:

```python
cc_library(
    name = "greet",
    srcs = ["greet.c"],
    hdrs = ["greet.h"],
)

cc_binary(
    name = "hello",
    srcs = ["hello.c"],
    deps = [":greet"],
)
```

The same DAG (`hello` depends on `greet`) appears, but every input is enumerated explicitly. `bazel build //:hello` would sandbox the action, hash inputs, check the cache, and either fetch or compile.

### Step 5: Make a "build flakiness" demo

Show what a non-hermetic build looks like by hand:

```sh
cd code/
make clean
touch /tmp/some-env-flag           # an "input" Make doesn't know about
GREET_PREFIX="hi" make             # recipe reads $GREET_PREFIX
./hello                            # picks up "hi"
unset GREET_PREFIX
make                               # nothing rebuilds (Makefile didn't list env as dep)
./hello                            # WRONG — stale "hi" baked in
```

This is *the* failure mode that hermetic systems prevent.

## Use It

Real-world choices:

- **Linux kernel** uses a custom Kbuild system built on Make. Pragmatic, scales to ~30M lines, well-understood by the kernel community.
- **LLVM, KDE, OpenCV** use CMake. The dominant C++ build system today.
- **Google, Twitter (X), Stripe** use Bazel-style monorepo builds for cross-language reproducibility.
- **Every Rust project of any size** uses Cargo. Workspace mode (`[workspace]` in a top-level Cargo.toml) handles multi-crate projects fine.

Look at the build files of a project you respect. The DAG you read there is exactly the model in this lesson.

## Read the Source

- [The GNU Make manual, Chapter 4 (Writing Rules)](https://www.gnu.org/software/make/manual/html_node/Rules.html) — concise, authoritative.
- [Modern CMake](https://cliutils.gitlab.io/modern-cmake/) — Henry Schreiner's free book; teaches CMake without the historical baggage.
- [The Bazel "Build Concepts" doc](https://bazel.build/concepts/build-files) — explains the action graph model in three pages.
- [`rustc_codegen_ssa`](https://github.com/rust-lang/rust/tree/master/compiler/rustc_codegen_ssa) — the part of `rustc` that Cargo orchestrates; reading the entry-point shows what Cargo is really doing.

## Ship It

This lesson ships **`outputs/pick-a-build-system.md`** — a decision tree (six questions) that turns "what should I use?" into a definitive answer in under a minute.

## Exercises

1. **Easy.** Take the lesson's `code/Makefile` and add a `-MMD -MP` flow so editing `greet.h` triggers a rebuild of `hello.o`. Verify with `touch greet.h && make`.
2. **Medium.** Convert `code/CMakeLists.txt` to also produce a shared library `libgreet.so`/`.dylib` and an executable that links against it dynamically. Use `target_link_libraries(... INTERFACE ...)` to keep the interface clean.
3. **Hard.** Install Bazel (or Buck2, an open-source descendant). Get `bazel build //:hello` to succeed. Then introduce a non-determinism (e.g., a `#include` from a system path not declared in `srcs`) and observe how Bazel surfaces it — Make would silently let it happen.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Hermetic | "Reproducible" | The build's output depends only on declared inputs, not on the developer's machine state |
| Incremental | "Faster rebuild" | The build system only re-runs recipes whose declared inputs have changed |
| Generator (CMake) | "It builds the project" | A program that reads `CMakeLists.txt` and emits a real build file (Make, Ninja, etc.) |
| Action (Bazel) | "A build step" | A pure function from a labeled set of input files to a labeled set of output files |
| Fingerprint (Cargo) | "Cache key" | A hash over source + lockfile + features + rustc version + flags |

## Further Reading

- *Software Engineering at Google*, chapter 18 (Build Systems) — best one-chapter treatment of why hermetic builds matter at scale.
- [Bazel: Correct, reproducible, fast builds for everyone](https://bazel.build/about) — the official intro deck.
- [The Make-Ninja-CMake history](https://www.scivision.dev/cmake-ninja-make-difference/) — short note on why Ninja came to dominate.
