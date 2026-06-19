#!/usr/bin/env bash
# CTF Automation Script — pwntools, GDB, Ghidra
# Phase 12 — Cryptography & Security
#
# Wraps the Python CTF toolkit into a single CLI for analyzing binaries,
# finding offsets, running ret2win / ROP exploits, and remote challenges.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MAIN_PY="$SCRIPT_DIR/main.py"
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${CYAN}[*]${NC} $*"; }
ok()    { echo -e "${GREEN}[+]${NC} $*"; }
warn()  { echo -e "${YELLOW}[!]${NC} $*"; }
err()   { echo -e "${RED}[-]${NC} $*" >&2; }

check_deps() {
    local missing=0
    if ! command -v python3 &>/dev/null; then
        err "python3 is required but not installed."
        missing=1
    fi
    if ! command -v nc &>/dev/null; then
        warn "netcat (nc) not found; remote connect will fall back to pwntools"
    fi
    if ! command -v objdump &>/dev/null; then
        warn "objdump not found; static analysis limited"
    fi
    if ! command -v readelf &>/dev/null; then
        warn "readelf not found; ELF header inspection limited"
    fi
    if [[ "$missing" -eq 1 ]]; then
        exit 1
    fi
    if ! python3 -c "from pwn import *" &>/dev/null 2>&1; then
        info "Installing pwntools..."
        pip3 install pwntools --quiet
        ok "pwntools installed"
    fi
}

check_binary() {
    local binary="$1"
    if [[ ! -f "$binary" ]]; then
        err "Binary not found: $binary"
        exit 1
    fi
    if [[ ! -x "$binary" ]]; then
        warn "Binary is not executable; attempting chmod +x"
        chmod +x "$binary" 2>/dev/null || {
            err "Cannot make binary executable"
            exit 1
        }
    fi
}

analyze() {
    local binary="$1"
    check_binary "$binary"
    info "Analyzing binary: $binary"
    python3 "$MAIN_PY" analyze "$binary"
}

offset() {
    local binary="$1"
    check_binary "$binary"
    info "Finding RIP offset for: $binary"
    python3 "$MAIN_PY" offset "$binary"
}

ret2win() {
    local binary="$1"
    local mode="${2:-local}"
    shift 2 || true
    check_binary "$binary"
    if [[ "$mode" == "local" ]]; then
        info "Running ret2win exploit (local): $binary"
        exec python3 "$MAIN_PY" ret2win "$binary"
    elif [[ "$mode" == "remote" ]]; then
        local host="${1:-}"
        local port="${2:-}"
        if [[ -z "$host" || -z "$port" ]]; then
            err "Remote mode requires: ret2win <binary> remote <host> <port>"
            exit 1
        fi
        info "Running ret2win exploit (remote): $binary @ $host:$port"
        exec python3 "$MAIN_PY" ret2win "$binary" --host "$host" --port "$port"
    fi
}

rop() {
    local binary="$1"
    local libc="${2:-}"
    check_binary "$binary"
    if [[ -n "$libc" ]]; then
        if [[ ! -f "$libc" ]]; then
            err "libc not found: $libc"
            exit 1
        fi
        info "Running ROP ret2libc exploit with libc: $binary"
        exec python3 "$MAIN_PY" rop "$binary" --libc "$libc"
    else
        info "Running ROP ret2libc exploit (local libc): $binary"
        exec python3 "$MAIN_PY" rop "$binary"
    fi
}

remote() {
    local host="${1:-}"
    local port="${2:-}"
    local binary="${3:-}"
    shift 3 || true
    local technique="${1:-ret2win}"
    local libc="${2:-}"
    if [[ -z "$host" || -z "$port" || -z "$binary" ]]; then
        err "Usage: remote <host> <port> <binary> [ret2win|rop] [libc_path]"
        exit 1
    fi
    check_binary "$binary"
    info "Remote exploit: $binary @ $host:$port (technique: $technique)"
    if [[ "$technique" == "rop" && -n "$libc" ]]; then
        exec python3 "$MAIN_PY" remote "$binary" \
            --host "$host" --port "$port" \
            --technique "$technique" --libc "$libc"
    else
        exec python3 "$MAIN_PY" remote "$binary" \
            --host "$host" --port "$port" \
            --technique "$technique"
    fi
}

download() {
    local host="${1:-}"
    local port="${2:-}"
    local output_dir="${3:-./challenge}"
    if [[ -z "$host" || -z "$port" ]]; then
        err "Usage: download <host> <port> [output_dir]"
        exit 1
    fi
    info "Downloading challenge binary from $host:$port"
    mkdir -p "$output_dir"
    # Try common CTF patterns to retrieve the binary
    if command -v curl &>/dev/null; then
        curl -s --connect-timeout 5 \
            "http://$host:$port/binary" \
            -o "$output_dir/binary" 2>/dev/null && {
            ok "Binary downloaded via HTTP"
            chmod +x "$output_dir/binary" 2>/dev/null || true
            return 0
        }
    fi
    # Fallback: try sending commands via netcat
    if command -v nc &>/dev/null; then
        echo "cat binary" | nc -w 3 "$host" "$port" > "$output_dir/binary" 2>/dev/null && {
            if [[ -s "$output_dir/binary" ]]; then
                ok "Binary downloaded via netcat"
                chmod +x "$output_dir/binary" 2>/dev/null || true
                return 0
            fi
        }
    fi
    warn "Could not auto-download binary; try manual download"
}

checksec() {
    local binary="$1"
    check_binary "$binary"
    info "Running checksec on: $binary"
    echo ""
    echo "  RELRO           STACK CANARY      NX            PIE             RPATH      RUNPATH      Symbols         FORTIFY  Fortified  Fortifiable  FILE"
    readelf -d "$binary" 2>/dev/null | grep -q "BIND_NOW" && \
        echo -n "  Full RELRO    " || \
        echo -n "  Partial RELRO "
    readelf -s "$binary" 2>/dev/null | grep -q "__stack_chk_fail" && \
        echo -n "  Canary found    " || \
        echo -n "  No canary       "
    readelf -l "$binary" 2>/dev/null | grep -q "GNU_STACK" && \
        readelf -l "$binary" 2>/dev/null | grep "GNU_STACK" | grep -q "E" && \
        echo -n "  NX disabled     " || \
        echo -n "  NX enabled      "
    readelf -h "$binary" 2>/dev/null | grep -q "DYN (Shared object file)" && \
        echo -n "  PIE enabled     " || \
        echo -n "  No PIE          "
    echo "  $binary"
    echo ""
}

disassemble() {
    local binary="$1"
    local function="${2:-main}"
    check_binary "$binary"
    info "Disassembling '$function' from: $binary"
    objdump -d -M intel "$binary" | sed -n "/<$function>:/,/^$/p" || {
        warn "Function '$function' not found or objdump unavailable"
    }
}

strings_search() {
    local binary="$1"
    local pattern="${2:-flag}"
    check_binary "$binary"
    info "Searching for '$pattern' in: $binary"
    strings "$binary" | grep -i "$pattern" || {
        info "No matches found for '$pattern'"
    }
}

gdb_attach() {
    local binary="$1"
    check_binary "$binary"
    if command -v gdb &>/dev/null; then
        info "Launching GDB with pwndbg/GEF/PEDA (whichever is available)"
        exec gdb -q "$binary"
    else
        err "GDB is not installed"
        exit 1
    fi
}

print_banner() {
    cat << "EOF"
 ╔═══════════════════════════════════════════╗
 ║       CTF Exploit Development Toolkit     ║
 ║    pwntools  ·  GDB  ·  Ghidra           ║
 ╚═══════════════════════════════════════════╝
EOF
}

print_usage() {
    cat << USAGE
Usage: $0 <command> [args]

Commands:
  analyze <binary>              Show security properties, symbols, gadgets
  offset <binary>               Find RIP overwrite offset
  ret2win <binary> [local]      Exploit: ret2win (local)
  ret2win <binary> remote <h> <p>  Exploit: ret2win (remote)
  rop <binary> [libc_path]      Exploit: ROP ret2libc
  remote <h> <p> <bin> [tech]   Exploit: remote challenge
       [libc_path]              tech: ret2win (default) or rop
  download <host> <port> [dir]  Download challenge binary
  checksec <binary>             Show binary security protections
  disassemble <bin> [func]      Disassemble a function (default: main)
  strings <binary> [pattern]    Search strings in binary (default: "flag")
  gdb <binary>                  Open binary in GDB
  help                          Show this help message

Examples:
  $0 analyze ./challenge
  $0 ret2win ./vuln local
  $0 rop ./vuln ./libc.so.6
  $0 remote 10.0.0.2 1337 ./challenge rop ./libc.so.6
USAGE
}

main() {
    print_banner
    check_deps
    echo ""

    if [[ $# -lt 1 ]]; then
        print_usage
        exit 0
    fi

    case "$1" in
        analyze|offset|checksec|gdb)
            if [[ $# -lt 2 ]]; then
                err "$1 requires a binary argument"
                exit 1
            fi
            "$1" "${@:2}"
            ;;
        ret2win)
            if [[ $# -lt 2 ]]; then
                err "ret2win requires a binary argument"
                exit 1
            fi
            ret2win "${@:2}"
            ;;
        rop)
            if [[ $# -lt 2 ]]; then
                err "rop requires a binary argument"
                exit 1
            fi
            rop "${@:2}"
            ;;
        remote)
            if [[ $# -lt 4 ]]; then
                err "remote requires host, port, and binary"
                exit 1
            fi
            remote "${@:2}"
            ;;
        download)
            if [[ $# -lt 3 ]]; then
                err "download requires host and port"
                exit 1
            fi
            download "${@:2}"
            ;;
        disassemble)
            if [[ $# -lt 2 ]]; then
                err "disassemble requires a binary"
                exit 1
            fi
            disassemble "${@:2}"
            ;;
        strings)
            if [[ $# -lt 2 ]]; then
                err "strings requires a binary"
                exit 1
            fi
            strings_search "${@:2}"
            ;;
        help|--help|-h)
            print_usage
            ;;
        *)
            err "Unknown command: $1"
            print_usage
            exit 1
            ;;
    esac
}

main "$@"
