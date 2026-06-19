# Rust Toolchain — cargo, rustup, build profiles

> Rust ships its compiler the way npm ships dependencies. Once you see it, you'll wish every language did the same.

**Type:** Learn
**Languages:** Rust
**Prerequisites:** Phase 00, Lessons 01–04
**Time:** ~60 minutes

## Learning Objectives

- Distinguish `rustup`, `rustc`, `cargo`, and `crates.io` — and explain who calls whom.
- Configure per-project Rust versions via `rust-toolchain.toml` and switch components (rust-analyzer, clippy, miri) on demand.
- Read and customize `Cargo.toml` and `[profile.*]` sections to tune debug vs release vs benchmark builds.
- Build, test, and benchmark a Rust crate, and inspect the resulting binary's size and dependencies.

## The Problem

In C-world, "install the compiler" is one decision you make once. In Rust-world it's three decisions:

1. **Which toolchain manager** — `rustup`, almost always.
2. **Which channel** — stable, beta, or nightly?
3. **Which version on that channel** — pinned per project, or latest from CI?

Then within a project, `cargo` adds *its own* layers: which profile (`dev`, `release`, `test`, `bench`), which features, which target triple. Get one of those wrong and you'll spend hours wondering why your code "compiles for some teammates but not others," or why your release binary is suddenly 200 MB.

This lesson is the map of `rustup`/`cargo`/`crates.io`. Once you can read a `Cargo.toml` and a `rust-toolchain.toml` and predict exactly what build you'll get, every Rust lesson in the course becomes a routine `cargo run`.

## The Concept

### The four players

```
   you ──→ rustup ──→ rustc, cargo, rustfmt, clippy, rust-analyzer
                          │
                          ▼
                       cargo  ←→  crates.io
                          │           (registry of published packages)
                          ▼
                       compiled binary
```

| Tool | What it does |
|------|--------------|
| `rustup`  | Installs and switches Rust toolchains (stable/beta/nightly), components (rust-src, clippy, miri), and targets (cross-compilation triples) |
| `rustc`   | The actual Rust compiler. You will rarely call it directly |
| `cargo`   | Project manager: build, test, run, doc, publish, fetch deps. Calls `rustc` for you |
| `crates.io` | The default public package registry. `cargo` fetches dependencies from here |

`rustup` is itself a tiny program that doesn't depend on Rust to install Rust. It downloads `rustc` and `cargo` into `~/.rustup/toolchains/<channel>-<host>/`.

### Channels: stable, beta, nightly

Three channels track different points on the same trunk:

```
   nightly  ━━●━━━━━━━━━━━●━━━━━━━━━━●━━━━ (new every night)
   beta            ━━━━━━━●━━━━━━━━━━●━━━━ (every 6 weeks)
   stable                ━━━━━━━━━━━━●━━━━ (every 6 weeks)
```

- **Stable** has compatibility guarantees: a crate that compiles on stable today compiles on stable tomorrow.
- **Beta** is the next stable, branched ~6 weeks early. Mostly used for catching regressions.
- **Nightly** lets you opt into unstable features (gated by `#![feature(name)]`). Some libraries (`rocket` historically, certain perf crates) require nightly.

The course uses **stable**. If a lesson needs nightly, it'll say so explicitly.

### Pinning per project: `rust-toolchain.toml`

A project can pin its toolchain so every contributor (and CI) uses the same Rust:

```toml
# rust-toolchain.toml at the repo root
[toolchain]
channel = "1.78.0"           # or "stable" / "beta" / "nightly"
components = ["rustfmt", "clippy", "rust-analyzer"]
targets = ["x86_64-unknown-linux-musl"]   # for cross-compilation
```

When you `cd` into the project, `rustup` auto-installs and selects this toolchain. No more "works on my machine" caused by Rust-version drift.

### `Cargo.toml` and the project model

```toml
[package]
name = "myapp"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.36", features = ["full"] }

[dev-dependencies]
criterion = "0.5"

[profile.release]
lto = "thin"           # link-time optimization
opt-level = 3
codegen-units = 1
strip = "symbols"

[[bin]]
name = "myapp"
path = "src/main.rs"
```

- **`[package]`** — your crate's identity. `edition` selects 2015 / 2018 / 2021 / 2024 — each enables different language features.
- **`[dependencies]`** — runtime deps from crates.io (or git, or path).
- **`[dev-dependencies]`** — only present in tests/benches; not pulled into the release binary.
- **`[profile.*]`** — knobs per build profile. See below.

### Build profiles

| Profile | Default flags | When it runs |
|---------|---------------|--------------|
| `dev`     | `opt-level = 0`, `debug = true`         | `cargo build`, `cargo run` |
| `release` | `opt-level = 3`, `debug = false`        | `cargo build --release` |
| `test`    | Inherits `dev` defaults                 | `cargo test` |
| `bench`   | Inherits `release` defaults             | `cargo bench` |

Customize any of them in `Cargo.toml`. The most common production tuning for `[profile.release]`:

- `lto = "thin"` or `"fat"` — link-time optimization. Smaller, faster binaries; slower compile.
- `codegen-units = 1` — single codegen unit. Trades compile time for a few % runtime perf.
- `strip = "symbols"` — strip debug symbols from the final binary (smaller artifact).
- `panic = "abort"` — replace stack-unwinding on panic with abort. Saves a few KB.

### `cargo` subcommand cheat sheet

```sh
cargo new myapp                 # create a new project
cargo new --lib mylib           # create a library crate

cargo build                     # debug build
cargo build --release           # release build
cargo run -- arg1 arg2          # build + run; args after `--`
cargo test                      # run all tests (`#[test]` fns + tests/*.rs)
cargo bench                     # run benchmarks (in benches/)
cargo doc --open                # build docs + open in browser
cargo fmt                       # format the project
cargo clippy -- -D warnings     # lints; treat warnings as errors
cargo update                    # refresh Cargo.lock
cargo tree                      # show dep graph
cargo bloat --release           # what's making my binary fat? (third-party)
```

`cargo` is one of the most consistently-designed CLIs in modern programming. Every subcommand respects `--release`, `--features`, `--target`, `--bin`, `--example`.

## Build It

### Step 1: A minimal crate

```sh
cargo new --bin hello && cd hello
ls -la
# .gitignore   Cargo.toml   src/main.rs
cat Cargo.toml
cat src/main.rs

cargo run
# Compiling hello v0.1.0 (...)
#     Finished `dev` profile [unoptimized + debuginfo] target(s) in ...
# Hello, world!
```

### Step 2: Add a dependency

Edit `Cargo.toml`:

```toml
[dependencies]
anyhow = "1.0"
```

Edit `src/main.rs`:

```rust
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let n: i32 = "42".parse().context("failed to parse number")?;
    println!("Got: {n}");
    Ok(())
}
```

```sh
cargo run                       # cargo fetches `anyhow` and rebuilds
cat Cargo.lock                  # locked dep versions appear
```

### Step 3: See the difference between dev and release

```sh
cargo build
cargo build --release
ls -la target/debug/hello target/release/hello   # release is much smaller
file target/release/hello                          # less debug info
```

Then run the lesson's `main.rs` (in `code/main.rs`) with `cargo run` directly, since it's a `single-file` Rust runnable with `rustc`:

```sh
cd code/
rustc main.rs -O -o demo
./demo
```

### Step 4: Pin a per-project toolchain

Create `rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
components = ["clippy", "rustfmt"]
```

`cd` out and back in:

```sh
rustup show          # should select stable + the listed components
```

### Step 5: Add a benchmark profile

In `Cargo.toml`:

```toml
[profile.release]
lto = "thin"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

Rebuild release:

```sh
cargo build --release
ls -la target/release/hello       # smaller; faster to launch
```

### Step 6: Inspect the binary

```sh
cargo bloat --release             # what crates contribute size?
cargo tree                        # what's the dep DAG?
cargo audit                       # any deps have known CVEs?
```

These three commands answer "what am I shipping?" Get in the habit before publishing anything.

## Use It

This whole `rustup`/`cargo` model is what every other modern language has been borrowing. `pyenv` + `pip` + `pyproject.toml` is a poor man's `rustup` + `cargo` + `Cargo.toml`. `Node.js` has `nvm` + `npm` + `package.json`, same shape.

When you look at real Rust projects on GitHub:

- The Rust compiler itself: `rust-lang/rust` uses `bootstrap.toml` + a custom builder, but ultimately every crate inside it is built by `cargo`.
- `tokio`, `serde`, `clap`: a `Cargo.toml`, a `src/`, a `tests/`, a `benches/`. Nothing exotic.
- `ripgrep`: study its `Cargo.toml` for a tuned release profile (LTO, codegen-units=1, strip, panic=abort).

## Read the Source

- [The Cargo Book](https://doc.rust-lang.org/cargo/) — read chapters 2 (Cargo.toml), 3 (workspaces), and 6 (profiles).
- [`rustup`'s install script](https://sh.rustup.rs) — `curl https://sh.rustup.rs` is a few hundred lines of POSIX shell that installs Rust without Rust.
- [`rustc` driver source](https://github.com/rust-lang/rust/tree/master/compiler/rustc_driver) — see how `cargo` and `rustup` glue down to the actual compiler.

## Ship It

This lesson ships **`outputs/cargo-init.sh`** — a script that creates a new Rust project with the course's preferred defaults: a `rust-toolchain.toml`, a tuned `[profile.release]`, `clippy` configured to deny warnings, and a `.gitignore` that covers `target/` and `*.profraw`.

## Exercises

1. **Easy.** Add the `anyhow` crate to a fresh `cargo new --bin` project, then use `cargo tree` to enumerate its transitive dependencies. How many crates were pulled in?
2. **Medium.** Write a `rust-toolchain.toml` that pins to a specific stable version, requires `clippy` and `rustfmt`, and prepares `x86_64-unknown-linux-musl` for static-musl cross-compilation. Verify by building a release binary that runs in a `scratch` Docker container.
3. **Hard.** Take a small Rust binary (≥ a few thousand SLOC; pick from `ripgrep` or `bat`) and use `cargo bloat`, `cargo tree`, and the `[profile.release]` knobs to halve its binary size *without* changing the source code. Document each step's effect.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| `cargo` | "Rust's npm" | The build orchestrator + dependency resolver + test/bench runner; calls `rustc` |
| Crate | "A Rust file" | A compilation unit: either a binary (`src/main.rs`) or a library (`src/lib.rs`) |
| Profile | "Build flags" | A named set of compile/link options (`dev`, `release`, `test`, `bench`); each one has its own `target/<profile>/` output |
| Edition | "Rust version" | A language profile (2015/18/21/24) that opts in to syntactic and semantic changes; older editions keep compiling forever |

## Further Reading

- [The Rust Book](https://doc.rust-lang.org/book/) — Ch. 1 covers `rustup` and `cargo`; Ch. 14 covers workspaces and publishing.
- [Rust Reference — Editions](https://doc.rust-lang.org/edition-guide/) — read the migration notes between editions; surprisingly small surface.
- [Min-sized Rust](https://github.com/johnthagen/min-sized-rust) — a tour of `[profile.release]` settings and what each one buys you.
