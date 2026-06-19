#!/usr/bin/env bash
# Demo: read PATH, find duplicates, show toolchain provenance.
# Run: bash run.sh

set -euo pipefail

echo "== Tools provenance =="
for cmd in cc gcc clang make rustc cargo go python3 node git; do
  if command -v "$cmd" >/dev/null 2>&1; then
    printf "  %-10s -> %s\n" "$cmd" "$(command -v "$cmd")"
  else
    printf "  %-10s -> (not installed)\n" "$cmd"
  fi
done

echo
echo "== PATH (one entry per line) =="
echo "$PATH" | tr ':' '\n' | awk '{printf "  %2d  %s\n", NR, $0}'

echo
echo "== Duplicate binaries on PATH =="
echo "$PATH" | tr ':' '\n' | while read -r d; do
  [[ -d "$d" ]] || continue
  find "$d" -maxdepth 1 -type f -perm -u+x -printf '%f\n' 2>/dev/null \
    || find "$d" -maxdepth 1 -type f -perm +u+x 2>/dev/null | xargs -I{} basename {}
done | sort | uniq -d | head -20 | sed 's/^/  /'

echo
echo "Done. If any tool above prints (not installed), see outputs/verify-toolchain.sh."
