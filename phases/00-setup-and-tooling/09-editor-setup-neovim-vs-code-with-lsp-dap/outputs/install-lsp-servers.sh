#!/usr/bin/env bash
# install-lsp-servers.sh — cross-platform installer for the LSP servers used
# in the course. Idempotent; skips already-installed servers.

set -uo pipefail

GREEN='\033[0;32m'; YELLOW='\033[0;33m'; RED='\033[0;31m'; RESET='\033[0m'

have() { command -v "$1" >/dev/null 2>&1; }
ok()   { printf "${GREEN}✓${RESET} %s\n" "$*"; }
skip() { printf "${YELLOW}!${RESET} %s\n" "$*"; }
err()  { printf "${RED}✗${RESET} %s\n" "$*"; }

OS="$(uname -s)"

# ── clangd (C/C++) ─────────────────────────────────────────────
if have clangd; then
  ok "clangd already installed: $(clangd --version | head -1)"
else
  case "$OS" in
    Darwin) brew install llvm && skip "Add /opt/homebrew/opt/llvm/bin to PATH if clangd not found" ;;
    Linux)  if have apt; then sudo apt install -y clangd; else err "Install clangd from your package manager"; fi ;;
  esac
fi

# ── rust-analyzer ──────────────────────────────────────────────
if have rust-analyzer; then
  ok "rust-analyzer already installed: $(rust-analyzer --version)"
elif have rustup; then
  rustup component add rust-analyzer && ok "rust-analyzer installed via rustup"
else
  err "rustup not found; install Rust first (Lesson 05)"
fi

# ── pyright (Python) ───────────────────────────────────────────
if have pyright; then
  ok "pyright already installed"
elif have pipx; then
  pipx install pyright && ok "pyright installed via pipx"
elif have npm; then
  npm install -g pyright && ok "pyright installed via npm"
else
  skip "pyright not installed; install pipx or npm first"
fi

# ── ruff (Python linter as LSP) ────────────────────────────────
if have ruff; then
  ok "ruff already installed"
elif have pipx; then
  pipx install ruff && ok "ruff installed via pipx"
else
  skip "ruff not installed; install pipx first"
fi

# ── gopls (Go) ─────────────────────────────────────────────────
if have gopls; then
  ok "gopls already installed"
elif have go; then
  go install golang.org/x/tools/gopls@latest && ok "gopls installed via go"
else
  skip "go not installed; skip gopls"
fi

# ── typescript-language-server ─────────────────────────────────
if have typescript-language-server; then
  ok "typescript-language-server already installed"
elif have npm; then
  npm install -g typescript typescript-language-server && ok "tsserver installed via npm"
else
  skip "npm not installed; skip TypeScript"
fi

echo
echo "Done. Restart your editor; it should pick up the new servers automatically."
