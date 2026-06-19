# The CS Toolchain — What You'll Install and Why

> The wrong toolchain will quietly cost you a hundred hours over this course. Get it right once.

**Type:** Learn
**Languages:** Shell
**Prerequisites:** A laptop you control (macOS, Linux, or Windows + WSL2). Admin rights.
**Time:** ~45 minutes

## Learning Objectives

- Install a reproducible polyglot toolchain — C/C++, Rust, Go, Python, Haskell, Node — and verify each works.
- Pick the right package manager for your OS and understand what it actually does (downloads, builds, links).
- Diagnose a "command not found" failure in under thirty seconds by reading `PATH` and `which`.
- Produce a single-script repro of your environment that another learner can run end-to-end.

## The Problem

A CS course is, in practice, a tour through other people's tools. You will compile C, you will link assembly, you will run a kernel in QEMU, you will model a protocol in TLA+, you will prove an algorithm in Coq. Every one of those tools is its own ecosystem with its own conventions, and the moment any one is misconfigured, the lesson it's used in becomes unreadable: error messages reference paths that don't exist, version numbers don't match docs, the same command produces different output on your machine and the author's.

You can spend hours chasing a problem that's just "I'm on `gcc 11`, the lesson assumed `gcc 13`." You can spend a full evening debugging "my Rust binary segfaults" only to discover you'd installed `rustc` through three different package managers and your shell was picking up the oldest one.

This first lesson is the boring one that prevents those evenings. We will install the eight tools the rest of the course leans on, verify each, and write a single script that any future learner — including you, six months from now on a new machine — can run to recreate this environment.

## The Concept

A **toolchain** is the ordered set of programs that turn source code into a running artifact. For C, the chain is roughly:

```
source.c ──[ preprocessor (cpp) ]──> source.i
source.i ──[ compiler (cc1)     ]──> source.s   (assembly)
source.s ──[ assembler (as)     ]──> source.o   (object code)
source.o ──[ linker (ld)        ]──> a.out      (executable)
```

For Rust it's `rustc` invoking LLVM and the system linker. For Python it's the bytecode compiler and the CPython VM. Each language has its toolchain *and* a package manager that installs that toolchain on your machine.

On your laptop, you sit at the top of a small hierarchy:

```
   user's intent ──> shell command ──> binary on PATH ──> system files
                          ▲
                          │ "which <cmd>" answers: where did this come from?
                          ▼
                  package manager (apt / brew / winget / rustup / nvm)
```

The single most useful skill in this lesson is reading **what came from where**. Two commands answer that on any Unix:

- `which <cmd>` — prints the first match for `<cmd>` in your `PATH`.
- `command -v <cmd>` — same, but a shell builtin (works in stripped containers).

If `which gcc` prints `/opt/homebrew/bin/gcc` you know Homebrew installed it; if it prints `/usr/bin/gcc` you know the OS shipped it. When two `gcc`s exist on disk, your `PATH` order decides which one runs.

```
$ echo $PATH | tr ':' '\n'
/opt/homebrew/bin           ← Homebrew wins for things it installs
/usr/local/bin              ← old Homebrew on Intel macs
/usr/bin                    ← system tools
/bin                        ← core POSIX tools
```

Everything else in this lesson follows from that picture.

## Build It

We'll install eight tools, verify each, and roll the verification into one script. Pick the column for your OS — the commands are different but the *result* is identical.

### Step 1: Pick (or install) a base package manager

A package manager turns "install gcc" into "download the right binary, put it in `PATH`, register it for upgrades." Skip system-level package managers and you end up installing things by hand, which never stays clean for long.

| OS | Manager | Install command |
|----|---------|-----------------|
| macOS | Homebrew | `/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"` |
| Linux (Debian/Ubuntu) | apt | (preinstalled — `sudo apt update`) |
| Linux (Fedora/RHEL) | dnf | (preinstalled) |
| Linux (Arch) | pacman | (preinstalled) |
| Windows | WSL2 + Ubuntu | `wsl --install -d Ubuntu` from PowerShell, then use apt inside |

Verify:

```sh
brew --version || apt --version || dnf --version || pacman --version
```

One of those should print a version. Otherwise stop — every following step depends on this.

### Step 2: Install the C/C++ toolchain (and `make`)

```sh
# macOS — installs gcc, clang, make, and the SDK headers
xcode-select --install
brew install gcc           # adds GNU gcc alongside Apple's clang

# Debian/Ubuntu
sudo apt install -y build-essential gdb valgrind

# Fedora/RHEL
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y gdb valgrind

# Arch
sudo pacman -S --needed base-devel gdb valgrind
```

Verify:

```sh
cc --version
make --version
gdb --version || echo "gdb missing — install separately on macOS via brew install gdb"
```

`build-essential` (apt) is a metapackage: it installs `gcc`, `g++`, `make`, and headers in one go. Always prefer a metapackage when one exists — fewer packages to track means fewer mismatches.

### Step 3: Install Rust via `rustup`

Rust does not get installed through your system package manager. `apt install rustc` gives you whatever Rust version Debian shipped two years ago, and the course assumes a stable Rust from this year.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Accept defaults — they're correct.
source "$HOME/.cargo/env"
```

`rustup` is a *toolchain manager* for Rust: it can hold multiple Rust versions side by side. The course needs only the stable channel.

Verify:

```sh
rustc --version
cargo --version
rustup show
```

### Step 4: Install Go

```sh
# macOS / Homebrew
brew install go

# Debian/Ubuntu — install latest from the official tarball; apt's go is usually stale
curl -fsSL https://go.dev/dl/go1.22.0.linux-amd64.tar.gz | sudo tar -C /usr/local -xzf -
echo 'export PATH=$PATH:/usr/local/go/bin' >> ~/.bashrc
source ~/.bashrc
```

Verify:

```sh
go version
```

### Step 5: Install Python (and uv)

macOS ships an Apple Python that you should not use for development. Debian's `python3` is fine. Either way, also install `uv` — a fast Python package manager and venv tool.

```sh
# Make sure system python3 exists
python3 --version

# Install uv (recommended for the course's Python lessons)
curl -LsSf https://astral.sh/uv/install.sh | sh
```

Verify:

```sh
python3 --version
uv --version
```

`uv` will be used in algorithm lessons to spin up isolated environments per lesson without polluting your system Python.

### Step 6: Install Node.js (for the site builder + TypeScript lessons)

The site (`site/build.js`) is plain Node, no framework. Pick any modern Node ≥ 20.

```sh
# Use nvm so per-project Node versions can coexist
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash
source ~/.nvm/nvm.sh
nvm install --lts
nvm use --lts
```

Verify:

```sh
node --version
npm --version
```

### Step 7: Install Haskell (via GHCup) — for Phase 18

You don't need this on Day 1. Skip it until Phase 18 unless you want a one-and-done install.

```sh
curl --proto '=https' --tlsv1.2 -sSf https://get-ghcup.haskell.org | sh
# answer "yes" to PATH integration
source "$HOME/.ghcup/env"
ghc --version
cabal --version
```

### Step 8: A few common utilities

```sh
# Debian/Ubuntu
sudo apt install -y git curl wget jq ripgrep fd-find tree htop tmux

# macOS
brew install git curl wget jq ripgrep fd tree htop tmux
```

You'll use `git` constantly (Lesson 03). `jq` for parsing JSON in the site builder. `ripgrep` (`rg`) and `fd` for searching code — they're an order of magnitude faster than `grep -r` and `find`.

### Step 9: Roll the verification into one script

The single artifact this lesson ships is `outputs/verify-toolchain.sh`. It runs every check above and reports a green/red status per tool. Copy the file from this lesson's `outputs/` folder and run it:

```sh
bash outputs/verify-toolchain.sh
```

When all rows are green, the rest of the course will build.

## Use It

The toolchain you just installed isn't ad-hoc — it mirrors what production systems actually use:

- **Linux distro maintainers** assemble the same set with `apt` / `dnf` / `pacman` and ship it as `build-essential` or `@development-tools`. Look at the package list of `build-essential` on Debian: it's basically Steps 2 and 8.
- **GitHub Actions runners** install almost this exact set in their `setup-*` actions. `actions/setup-rust@v1` runs `rustup` under the hood. `actions/setup-go@v5` is a wrapper over the same tarball you just downloaded.
- **`devcontainer.json`** for VS Code declares the same dependencies — `image: mcr.microsoft.com/devcontainers/cpp` is a Debian image that runs Step 2 at build time.

Read one. Look at `.github/workflows/ci.yml` in any sizable C/C++ or Rust project on GitHub. The "install dependencies" job is the script you just ran, written for a CI runner instead of your laptop.

## Read the Source

- `https://github.com/Homebrew/install/blob/HEAD/install.sh` — the Homebrew installer. ~700 lines of POSIX shell. Worth reading line by line: it teaches you what installing a tool actually means (resolve target, download, verify, link into PATH, register).
- `https://github.com/rust-lang/rustup/blob/master/src/cli/self_update/install.rs` — `rustup`'s self-installer. Note how it does NOT depend on Rust to install Rust — bootstrap problem solved with a small downloader.

## Ship It

The reusable artifact this lesson produces is **`outputs/verify-toolchain.sh`** — a single script that exits non-zero if any required tool is missing, with a clear table of what passed and what failed. Drop it in CI, run it after a fresh `git clone`, paste its output into a help thread when something doesn't work.

## Exercises

1. **Easy.** Run `which gcc`, `which cc`, `which clang`. If two or more print different paths, what does that mean about your environment? Sketch the answer in three sentences.
2. **Medium.** Take `outputs/verify-toolchain.sh` and add a check for a tool the script doesn't currently verify (suggested: `tmux`, `rg`, or `valgrind`). Make sure your addition keeps the script's exit-code contract: zero on success, non-zero on any failure, even if other failures happened before.
3. **Hard.** Reproduce the verification script in PowerShell for native Windows (no WSL). It must check the same set of tools, with the same exit-code contract, and produce a table styled similarly. Note where the verification logic genuinely has to differ (e.g., `xcode-select`) and where you can keep the cross-platform path.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Toolchain | "The compiler" | The ordered set of preprocessor, compiler, assembler, linker, and runtime that turns source into a running binary |
| `PATH` | "Where programs are" | A `:`-separated list of directories the shell searches *in order* when you type a command name |
| Package manager | "Like an app store" | A program that resolves dependencies, downloads binaries (or builds from source), and registers them for upgrade and removal |
| Stable channel | "The current version" | The version of a fast-moving toolchain (Rust, Node) that's promised not to break compatibility within minor releases |

## Further Reading

- [The Linux Documentation Project — Bash Beginners Guide](https://tldp.org/LDP/Bash-Beginners-Guide/html/) — old but precise on `PATH`, environment, and shell startup.
- [The Rust Book, Ch. 1](https://doc.rust-lang.org/book/ch01-01-installation.html) — Rust's installation chapter is unusually thorough; it teaches `rustup`'s mental model in 20 minutes.
- [Homebrew's "How Brew Works"](https://docs.brew.sh/Homebrew-and-Python) — read this if you've ever wondered why `brew` installs Python in `/opt/homebrew/Cellar/python@3.12/...`.
