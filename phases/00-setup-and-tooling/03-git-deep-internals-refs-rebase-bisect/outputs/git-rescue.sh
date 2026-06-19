#!/usr/bin/env bash
# git-rescue.sh — produce a panel of "what just happened" info when someone
# says "git ate my work." Run it inside the broken repo. Read-only; never
# modifies state.

set -uo pipefail

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "Not inside a git work tree. cd into the repo first." >&2
  exit 1
fi

hr() { printf "\n── %s ──────────────────────────────────────\n" "$1"; }

hr "Repo root"
git rev-parse --show-toplevel

hr "HEAD"
echo "  Current: $(git rev-parse HEAD 2>/dev/null || echo '(no commits yet)')"
echo "  Symbolic: $(git symbolic-ref --short HEAD 2>/dev/null || echo '(detached)')"

hr "Working tree status"
git status --short --branch || true

hr "Reflog (last 15 HEAD moves — recoverable for ~90 days)"
git reflog -15 || true

hr "Dangling commits (commits no branch points at)"
git fsck --no-reflogs --lost-found 2>/dev/null | grep '^dangling commit' | head -10
if [[ $(git fsck --no-reflogs --lost-found 2>/dev/null | grep -c '^dangling commit') -eq 0 ]]; then
  echo "  (none)"
fi

hr "Untracked-but-ignored files (won't appear in status)"
git ls-files --others --ignored --exclude-standard | head -10
if [[ $(git ls-files --others --ignored --exclude-standard | wc -l) -eq 0 ]]; then
  echo "  (none)"
fi

hr "Stashes"
git stash list || true

hr "Recent local commits (last 10 on this branch)"
git log -10 --oneline --decorate || true

echo
echo "Recovery hints:"
echo "  - A recent commit you 'lost'?    → git reset --hard <sha-from-reflog>"
echo "  - Files you 'lost'?              → git checkout <sha> -- <path>"
echo "  - Branch you deleted?            → git branch <name> <sha-from-reflog>"
echo "  - Unsure?                        → DO NOTHING DESTRUCTIVE. Paste this panel and ask for help."
