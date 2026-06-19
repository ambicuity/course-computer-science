#!/usr/bin/env bash
# Build the lesson's image, exercise it, and demonstrate layer caching.
# Skips gracefully if docker isn't installed or the daemon isn't running.

set -uo pipefail
cd "$(dirname "$0")"

hr() { printf "\n── %s ──────────────────────────────────────\n" "$1"; }

if ! command -v docker >/dev/null 2>&1; then
  echo "docker not installed. Install Docker Desktop, OrbStack, or Podman."
  exit 0
fi
if ! docker info >/dev/null 2>&1; then
  echo "Docker daemon not running. Start it and retry."
  exit 0
fi

hr "1. Cold build"
time docker build -t cs-toolchain-demo .

hr "2. Verify tools inside the image"
docker run --rm cs-toolchain-demo bash -c '
  echo "user: $(whoami)"
  echo "cc:   $(cc --version | head -1)"
  echo "make: $(make --version | head -1)"
  echo "git:  $(git --version)"
  echo "rust: $(rustc --version)"
'

hr "3. Warm rebuild (no source change → all CACHED)"
time docker build -t cs-toolchain-demo .

hr "4. Bind-mount + non-root demo"
TMPDIR=$(mktemp -d)
echo "Created host dir: $TMPDIR"
docker run --rm -v "$TMPDIR":/work -w /work \
  --user "$(id -u):$(id -g)" \
  cs-toolchain-demo bash -c 'echo "hello from container" > out.txt'
echo "Host now sees:"
ls -l "$TMPDIR/out.txt"
cat "$TMPDIR/out.txt"
rm -rf "$TMPDIR"

hr "5. Image size"
docker image ls cs-toolchain-demo --format 'table {{.Repository}}\t{{.Tag}}\t{{.Size}}'
