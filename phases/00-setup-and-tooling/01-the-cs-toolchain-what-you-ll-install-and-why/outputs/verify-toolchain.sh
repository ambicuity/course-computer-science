#!/usr/bin/env bash
# verify-toolchain.sh — single-script check that the course's toolchain is installed.
#
# Exits 0 iff every required tool is on PATH and runnable. On any failure, prints a
# table with the missing tool and returns the count of failures as the exit code.
#
# Drop into CI, run after fresh clone, paste output in help threads.

set -u  # NOTE: not -e — we collect every failure rather than stopping on the first

CHECKS=(
  "cc:--version:C compiler (gcc or clang via cc)"
  "make:--version:GNU/BSD make"
  "rustc:--version:Rust compiler"
  "cargo:--version:Rust package manager"
  "go:version:Go compiler"
  "python3:--version:Python interpreter"
  "node:--version:Node.js runtime"
  "git:--version:Git version control"
  "curl:--version:curl HTTP client"
  "jq:--version:jq JSON processor"
)

# Optional tools — warn but don't fail
OPTIONAL=(
  "gdb:--version:GNU Debugger"
  "valgrind:--version:Valgrind memory checker"
  "rg:--version:ripgrep"
  "ghc:--version:Glasgow Haskell Compiler"
  "uv:--version:uv Python package manager"
)

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
RESET='\033[0m'

pad() { printf "%-12s" "$1"; }

run_check() {
  local entry="$1"; local required="$2"
  local cmd="${entry%%:*}"
  local rest="${entry#*:}"
  local flag="${rest%%:*}"
  local desc="${rest#*:}"

  if ! command -v "$cmd" >/dev/null 2>&1; then
    if [[ "$required" == "1" ]]; then
      printf "  ${RED}✗${RESET} %s %-30s ${RED}missing${RESET}\n" "$(pad "$cmd")" "$desc"
      return 1
    else
      printf "  ${YELLOW}!${RESET} %s %-30s ${YELLOW}optional, not installed${RESET}\n" "$(pad "$cmd")" "$desc"
      return 0
    fi
  fi

  local version
  if ! version=$("$cmd" "$flag" 2>&1 | head -1); then
    printf "  ${RED}✗${RESET} %s %-30s ${RED}installed but failed: %s${RESET}\n" "$(pad "$cmd")" "$desc" "$version"
    return 1
  fi
  printf "  ${GREEN}✓${RESET} %s %-30s %s\n" "$(pad "$cmd")" "$desc" "$version"
  return 0
}

failures=0

echo "── Required tools ───────────────────────────────────────────"
for entry in "${CHECKS[@]}"; do
  run_check "$entry" 1 || failures=$((failures + 1))
done

echo
echo "── Optional tools ───────────────────────────────────────────"
for entry in "${OPTIONAL[@]}"; do
  run_check "$entry" 0
done

echo
if [[ $failures -eq 0 ]]; then
  printf "${GREEN}All required tools present. You're ready.${RESET}\n"
  exit 0
else
  printf "${RED}%d required tool(s) missing. Re-run the install steps in docs/en.md.${RESET}\n" "$failures"
  exit "$failures"
fi
