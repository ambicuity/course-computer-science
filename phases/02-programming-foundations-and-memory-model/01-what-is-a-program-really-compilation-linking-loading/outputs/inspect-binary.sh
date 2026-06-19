#!/usr/bin/env bash
# inspect-binary.sh — one-page summary of any binary's structure and dependencies.
# Usage: ./inspect-binary.sh path/to/binary

set -uo pipefail

BIN="${1:-}"
if [[ -z "$BIN" || ! -f "$BIN" ]]; then
  echo "usage: $0 path/to/binary" >&2
  exit 1
fi

hr() { printf "\n── %s ──────────────────────────────────────\n" "$1"; }

hr "File"
file "$BIN"
echo "  size: $(stat -f %z "$BIN" 2>/dev/null || stat -c %s "$BIN") bytes"

hr "Sections / segments"
if command -v readelf >/dev/null 2>&1; then
  readelf -S "$BIN" 2>/dev/null | grep -E '\.(text|data|bss|rodata|init|fini|plt|got)' | head
elif command -v size >/dev/null 2>&1; then
  size "$BIN"
fi

hr "Symbol summary"
if command -v nm >/dev/null 2>&1; then
  total=$(nm "$BIN" 2>/dev/null | wc -l)
  text=$(nm "$BIN" 2>/dev/null | grep -c '^[0-9a-f]* [tT]' || true)
  data=$(nm "$BIN" 2>/dev/null | grep -c '^[0-9a-f]* [dD]' || true)
  bss=$(nm "$BIN" 2>/dev/null | grep -c '^[0-9a-f]* [bB]' || true)
  echo "  total symbols: $total"
  echo "  text symbols (functions): $text"
  echo "  data symbols: $data"
  echo "  bss symbols: $bss"
fi

hr "Dynamic dependencies"
if command -v ldd >/dev/null 2>&1; then
  ldd "$BIN" 2>&1
elif command -v otool >/dev/null 2>&1; then
  otool -L "$BIN" 2>&1
fi

hr "Largest 10 functions (by symbol size)"
if command -v nm >/dev/null 2>&1; then
  nm -S "$BIN" 2>/dev/null | awk '{ if (NF >= 4) printf "%s %s\n", $2, $4 }' \
    | sort -rk1 | head -10 | awk '{print "  size=0x"$1, $2}'
fi
