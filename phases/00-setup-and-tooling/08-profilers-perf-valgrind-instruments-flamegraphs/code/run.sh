#!/usr/bin/env bash
# Build the profile target, run with /usr/bin/time, and (if perf or instruments
# is available) capture a sampling profile.

set -uo pipefail
cd "$(dirname "$0")"

hr() { printf "\n── %s ──────────────────────────────────────\n" "$1"; }

hr "Build with -O2 -g (release-ish + symbol info)"
gcc -O2 -g main.c -o profile-target
ls -lh profile-target

hr "Time the workload"
TIMEFORMAT='real %3R  user %3U  sys %3S'
{ time ./profile-target ; } 2>&1

hr "Sampling profiler"
if command -v perf >/dev/null 2>&1; then
  echo "Capturing 999 Hz call-graph for 2 seconds with perf..."
  perf record -F 999 -g -- ./profile-target >/dev/null 2>&1 || true
  echo "Top symbols (perf report):"
  perf report --stdio --no-children 2>/dev/null | head -25
  rm -f perf.data perf.data.old
elif command -v xctrace >/dev/null 2>&1; then
  echo "Recording with xctrace (Instruments CLI)..."
  rm -rf trace.trace
  xctrace record --template "Time Profiler" --launch ./profile-target --output trace.trace 2>&1 | tail -5
  echo "Open trace.trace in Instruments.app to view."
else
  echo "Neither perf nor xctrace found; skipping sampling profile."
  echo "On Linux: sudo apt install linux-tools-common linux-tools-\$(uname -r)"
  echo "On macOS: xctrace ships with Xcode Command Line Tools."
fi

hr "Memory-checker (valgrind)"
if command -v valgrind >/dev/null 2>&1; then
  gcc -O0 -g main.c -o profile-target-dbg
  valgrind --tool=memcheck --leak-check=full --error-exitcode=1 ./profile-target-dbg 2>&1 | tail -15
  rm -f profile-target-dbg
else
  echo "valgrind not installed; skipping memory check."
fi

rm -f profile-target
