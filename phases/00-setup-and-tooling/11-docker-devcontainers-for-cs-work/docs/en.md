# Docker & Devcontainers for CS Work

> A container is a recipe for "the same machine, anywhere." For CS work, that recipe replaces a long README.

**Type:** Build
**Languages:** Dockerfile, Shell
**Prerequisites:** Phase 00, Lessons 01, 04, 10
**Time:** ~60 minutes

## Learning Objectives

- Explain what an image, a container, a layer, and a tag are; predict how layer caching affects rebuild time.
- Write a `Dockerfile` that builds a multi-language CS toolchain image, using `RUN` / `COPY` / `WORKDIR` / `ENTRYPOINT` correctly.
- Configure a `.devcontainer/devcontainer.json` so the course works in VS Code / Codespaces / any devcontainer-aware editor.
- Diagnose the three classic Docker pitfalls: image bloat (large layers), cache invalidation (slow rebuilds), and permission mismatches (root inside, you outside).

## The Problem

Lesson 01 had you install ten tools. Tomorrow someone else takes the course on a different OS. Next week they upgrade their distro. The week after, they wipe and start over. Each time, hours of "install" and "tweak PATH" — and a non-zero chance of inconsistencies because the tool versions drift.

A container is the answer: a self-contained image with all the tools pre-installed, sealed, and rerunnable. Anyone with Docker (or Podman, or any OCI runtime) can pull and run it. CI pulls the same image. The "works on my machine" gap closes.

Devcontainers go one step further: they wrap a Docker image in a small JSON config that tells your editor (VS Code, JetBrains, neovim with `devpod`) "open this repo inside *that* container." Edits feel native; the toolchain stays containerized.

This lesson teaches both, on top of the kernel primitives from Lesson 10.

## The Concept

### Image vs container

| | Image | Container |
|--|-------|-----------|
| What it is | An immutable filesystem snapshot + metadata | A running (or stopped) process with a writable layer on top of an image |
| Where it lives | In an image registry (Docker Hub, ghcr.io) and pulled locally | Only on the host that's running it |
| Lifecycle | Built once, used many times | Started, stopped, possibly deleted |

The unit of distribution is the *image*. The unit of execution is the *container*.

### Layers and caching

A Dockerfile is a script. Each `RUN`, `COPY`, `ADD` line produces a **layer** — a tarball of filesystem changes. Layers are content-addressable: if a layer's inputs haven't changed, Docker reuses the cached layer instead of re-running the command.

```
FROM debian:stable-slim                  # layer 0 — base
RUN apt-get update && apt-get install...  # layer 1 — packages
COPY . /app                                # layer 2 — your source
RUN cd /app && make                        # layer 3 — build
```

Edit a file under `/app` and only layers 2 and 3 rebuild. Edit the `apt-get` line and *everything* below rebuilds.

The standard performance discipline:

- Put **stable** instructions first (base image, system packages).
- Put **volatile** instructions last (your source code).
- Group related `RUN` commands with `&&` to avoid producing many tiny layers.
- Pin versions explicitly so cache stays valid across CI runs.

### A clean Dockerfile

```dockerfile
# 1. Pin a specific base image. Latest is for amateurs.
FROM debian:bookworm-slim AS base

# 2. Install system packages in one layer.
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential gdb valgrind ca-certificates curl git pkg-config && \
    rm -rf /var/lib/apt/lists/*

# 3. Add a non-root user. Containers default to root; that's bad practice.
RUN useradd -m -s /bin/bash student

# 4. Set workdir and copy source.
WORKDIR /home/student/course
COPY --chown=student:student . .

# 5. Drop privileges.
USER student

# 6. Default command — what runs when someone `docker run`s the image.
CMD ["bash"]
```

Build: `docker build -t cs-course .`
Run interactively: `docker run -it --rm cs-course`

### Multi-stage builds

For binaries you ship, use multi-stage to keep the final image small:

```dockerfile
# Build stage — has the toolchain
FROM rust:1.78 AS build
WORKDIR /src
COPY . .
RUN cargo build --release

# Runtime stage — minimal
FROM debian:bookworm-slim
COPY --from=build /src/target/release/myapp /usr/local/bin/myapp
USER nobody
ENTRYPOINT ["/usr/local/bin/myapp"]
```

The final image carries only the compiled binary — no Rust toolchain, no source. Often shrinks a 1.5 GB image to ~80 MB.

### `.devcontainer/devcontainer.json`

```json
{
  "name": "course-computer-science",
  "image": "ghcr.io/your-org/cs-course:latest",
  "features": {
    "ghcr.io/devcontainers/features/rust:1": { "version": "stable" }
  },
  "postCreateCommand": "bash scripts/install-lsp-servers.sh",
  "customizations": {
    "vscode": {
      "extensions": ["llvm-vs-code-extensions.vscode-clangd",
                     "rust-lang.rust-analyzer"]
    }
  },
  "remoteUser": "student"
}
```

Open the repo in VS Code; press `Reopen in Container`; your editor reattaches inside the image. The terminal, the LSP, the debugger all run in the container; your laptop stays clean.

The same JSON works for GitHub Codespaces — push the repo, click "Open in Codespaces," get the same env in a browser.

### Three pitfalls

1. **Image bloat.** Forgetting `rm -rf /var/lib/apt/lists/*` after `apt-get install` leaves the cache in the image (50–200 MB wasted). Use multi-stage builds for compiled artifacts.
2. **Cache invalidation.** Putting `COPY . .` before slow installs causes every source-edit to invalidate the install layer, so every build re-runs `apt-get`. Put volatile copies LAST.
3. **Permission mismatch.** Docker defaults to running as root (UID 0). Files it writes to a bind-mounted volume end up owned by root on your host. Either run as your host UID (`docker run --user "$(id -u):$(id -g)"`) or use rootless Docker / Podman.

## Build It

### Step 1: Build the lesson's Dockerfile

`code/Dockerfile` builds an image with the basic CS toolchain. Build it:

```sh
cd code/
docker build -t cs-toolchain-demo .
docker run --rm cs-toolchain-demo bash -c 'cc --version; rustc --version'
```

(The image is small and the build is fast — a couple of minutes on a cold cache, seconds on a warm one.)

### Step 2: Layer caching demo

```sh
# First build — cold; downloads everything
docker build -t cs-toolchain-demo .

# Touch a source file — only the COPY and what's below should rebuild
echo "// touch" >> demo.txt
docker build -t cs-toolchain-demo .   # observe layer cache reuse

# Edit the package list — everything from there down rebuilds
sed -i.bak 's/curl/curl wget/' Dockerfile
docker build -t cs-toolchain-demo .
mv Dockerfile.bak Dockerfile         # restore
```

Watch the `CACHED` / `RUN` lines in the build output — they tell you exactly which layers were reused.

### Step 3: Run a container with a bind mount

```sh
docker run --rm -it \
  -v "$PWD":/work \
  -w /work \
  --user "$(id -u):$(id -g)" \
  cs-toolchain-demo
# Inside: edit files, build, exit
ls -l   # files on host owned by you (because of --user)
```

The bind mount makes your host directory visible inside the container. The `--user` flag prevents root-owned files. Two flags solve 90% of dev-loop pain.

### Step 4: Author a devcontainer

```sh
mkdir -p .devcontainer
cat > .devcontainer/devcontainer.json <<'JSON'
{
  "name": "course-computer-science",
  "build": { "dockerfile": "../code/Dockerfile" },
  "remoteUser": "student",
  "postCreateCommand": "echo Welcome to the CS course",
  "customizations": {
    "vscode": {
      "extensions": [
        "llvm-vs-code-extensions.vscode-clangd",
        "rust-lang.rust-analyzer"
      ]
    }
  }
}
JSON
```

Open the repo in VS Code with the "Dev Containers" extension; "Reopen in Container." VS Code rebuilds the image (using your Dockerfile) and re-opens the workspace inside it.

### Step 5: Push and share

```sh
docker tag cs-toolchain-demo ghcr.io/your-org/cs-toolchain:0.1
docker push ghcr.io/your-org/cs-toolchain:0.1
```

CI and other contributors can now pull this exact image. Pin the tag in `devcontainer.json`'s `image` field.

## Use It

Real-world container patterns:

- **CI runners** are containers. GitHub Actions' `ubuntu-latest` is a Debian image with pre-installed tools — the same model.
- **Production microservices** ship multi-stage Docker images. The runtime image carries just the binary + libc.
- **Codespaces / Gitpod / Coder** are all "devcontainer in the cloud" — you author `.devcontainer/devcontainer.json` once; the cloud spins up a VM with your image and proxies your editor to your browser.
- **`distroless` images** (Google's gcr.io/distroless/static) ship an image with no shell at all — just glibc and your static binary. Minimal attack surface for production.

## Read the Source

- [Docker docs — Build best practices](https://docs.docker.com/build/building/best-practices/) — concise; the cache-invalidation and `&&` rules are official.
- [Dev Containers spec](https://containers.dev/) — the formal `devcontainer.json` reference.
- [`runc`](https://github.com/opencontainers/runc) — the OCI reference runtime. Read `libcontainer/` to see the namespace+cgroup setup from Lesson 10 in production code.
- [Podman docs on rootless](https://docs.podman.io/en/latest/markdown/podman.1.html) — how to avoid the root-owned-files trap.

## Ship It

This lesson ships **`outputs/devcontainer.json`** — a ready-to-drop-in dev container for the entire course-computer-science repo, plus **`outputs/Dockerfile.cs-course`** — the production version of the lesson's Dockerfile.

## Exercises

1. **Easy.** Build the lesson's image. Confirm `docker image ls cs-toolchain-demo` shows its size. Predict whether changing the `CMD` line invalidates earlier layers; verify by rebuilding.
2. **Medium.** Convert the lesson's single-stage Dockerfile into a multi-stage build: stage 1 has the full toolchain; stage 2 carries only a runtime image with a precompiled `hello` binary. Measure the size of each.
3. **Hard.** Author a `.devcontainer/devcontainer.json` for the *whole* course repo that:
   - Uses the course's Dockerfile.
   - Installs the LSP servers from Lesson 09.
   - Sets `remoteUser` to a non-root account.
   - Pre-installs the VS Code extensions for clangd, rust-analyzer, and codelldb.

   Verify by opening the repo in Codespaces (or locally with Dev Containers).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Image | "A container" | An immutable, layered filesystem + metadata bundle; the input to running a container |
| Container | "A small VM" | A *process* the kernel started inside namespaces + a cgroup + (usually) a pivot_root onto an image |
| Layer | "Part of an image" | A tarball of filesystem changes produced by one Dockerfile instruction; content-addressed |
| Devcontainer | "Cloud dev env" | A JSON descriptor pairing an image with an editor; lets you open a repo "inside" the image |

## Further Reading

- [Adrian Mouat — Using Docker](https://www.oreilly.com/library/view/using-docker/9781491915752/) — practical and dated but still the clearest single-volume intro.
- [Docker Slim](https://github.com/slimtoolkit/slim) — automated image-size minimizer; instructive to watch what it strips.
- [BuildKit](https://github.com/moby/buildkit) — the modern Docker build backend. `docker buildx` uses it; understand it for serious build-perf work.
