#!/usr/bin/env bash
# Walk through Make, CMake (if installed), and Cargo (if installed).
# Ends with the build-flakiness scenario.

set -uo pipefail
cd "$(dirname "$0")"

hr() { printf "\n── %s ──────────────────────────────────────\n" "$1"; }

hr "1. Make build"
make clean >/dev/null
make test
./hello
echo

hr "2. CMake + Ninja (or Make) build"
if command -v cmake >/dev/null 2>&1; then
  GEN=Ninja
  command -v ninja >/dev/null 2>&1 || GEN="Unix Makefiles"
  rm -rf build
  cmake -S . -B build -G "$GEN" >/dev/null
  cmake --build build >/dev/null
  ./build/hello
  echo
  ctest --test-dir build --output-on-failure 2>&1 | tail -5
else
  echo "  cmake not installed; skipping."
fi
echo

hr "3. Cargo (Rust translation)"
if command -v cargo >/dev/null 2>&1 && [[ -d cargo-version ]]; then
  ( cd cargo-version && cargo run --quiet )
else
  echo "  cargo not installed (or no cargo-version/ subdir); skipping."
fi
echo

hr "4. Build-flakiness demo: env-dependent behavior"
make clean >/dev/null
GREET_PREFIX="hi" make >/dev/null
echo "Run binary with GREET_PREFIX=hi in the env:"
GREET_PREFIX="hi" ./hello | sed 's/^/  /'
echo "Same binary without the env var:"
./hello | sed 's/^/  /'
echo
echo "Lesson: this binary reads env at runtime, so behavior changes per invocation."
echo "Build-flakiness happens when ENV is consulted at BUILD time (e.g., -DPREFIX=\$ENV{...})"
echo "and the build system doesn't list env as a dep. Bazel sandboxes ALL inputs;"
echo "Make sees no env deps unless you tell it."
make clean >/dev/null
