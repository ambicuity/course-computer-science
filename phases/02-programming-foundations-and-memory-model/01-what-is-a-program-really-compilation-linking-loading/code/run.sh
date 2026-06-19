#!/usr/bin/env bash
# Walk the compilation pipeline, inspect the binary, and show the live memory map.

set -uo pipefail
cd "$(dirname "$0")"

hr() { printf "\n── %s ──────────────────────────────────────\n" "$1"; }

hr "Pipeline: source → .i → .s → .o → executable"
gcc -E main.c -o main.i;  echo "  preprocessed:  $(wc -l < main.i) lines (was $(wc -l < main.c))"
gcc -S main.c -o main.s;  echo "  assembly:      $(wc -l < main.s) lines"
gcc -c main.c -o main.o;  echo "  object file:   $(stat -f %z main.o 2>/dev/null || stat -c %s main.o) bytes"
gcc -O0 -g main.c -o main
echo "  executable:    $(stat -f %z main 2>/dev/null || stat -c %s main) bytes"

hr "Run"
./main

hr "Sections (Linux: readelf; macOS: size)"
if command -v readelf >/dev/null 2>&1; then
  readelf -S main | grep -E '\.(text|data|bss|rodata)' | head
elif command -v size >/dev/null 2>&1; then
  size main
fi

hr "Symbols (largest 10 by name visibility)"
if command -v nm >/dev/null 2>&1; then
  nm main 2>/dev/null | head -15
fi

hr "Dynamic dependencies"
if command -v ldd >/dev/null 2>&1; then
  ldd main 2>&1 | head -5
elif command -v otool >/dev/null 2>&1; then
  otool -L main 2>&1 | head -5
fi

hr "Live memory map (sample run)"
./main >/dev/null &
PID=$!
sleep 0.3
if [[ -f /proc/$PID/maps ]]; then
  head -8 /proc/$PID/maps | sed 's/^/  /'
elif command -v vmmap >/dev/null 2>&1; then
  vmmap "$PID" 2>/dev/null | head -20 | sed 's/^/  /'
else
  echo "  no maps/vmmap available"
fi
wait $PID 2>/dev/null

rm -f main.i main.s main.o main
