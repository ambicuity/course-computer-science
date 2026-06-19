#!/usr/bin/env bash
# Demonstrate reading /proc and /sys on Linux. On macOS prints what would be
# equivalent (sysctl / vm_stat / ifconfig).

set -uo pipefail
cd "$(dirname "$0")"

hr() { printf "\n── %s ──────────────────────────────────────\n" "$1"; }

OS="$(uname -s)"

hr "1. Build and run procreader.c"
gcc main.c -o procreader
./procreader
rm -f procreader

if [[ "$OS" != "Linux" ]]; then
  echo
  echo "(non-Linux host: rest of the demo is Linux-only;"
  echo " /proc, /sys, cgroups, namespaces don't exist on macOS.)"
  exit 0
fi

hr "2. Top 5 processes by RSS (no ps)"
for s in /proc/[0-9]*/status; do
  pid=$(basename "$(dirname "$s")")
  name=$(awk '/^Name:/{print $2}' "$s" 2>/dev/null)
  rss=$(awk '/^VmRSS:/{print $2}' "$s" 2>/dev/null)
  [[ -n "$rss" ]] && echo "$rss $pid $name"
done 2>/dev/null | sort -rn | head -5 | sed 's/^/  /'

hr "3. cgroup v2 root view"
if mount | grep -q cgroup2; then
  ls /sys/fs/cgroup/ | head -10 | sed 's/^/  /'
  echo "  available controllers: $(cat /sys/fs/cgroup/cgroup.controllers)"
else
  echo "  cgroup v2 not mounted"
fi

hr "4. Namespaces of this shell"
ls -l /proc/self/ns/ | sed 's/^/  /'

hr "5. Optional: PID namespace demo (requires sudo)"
if [[ "${EUID:-0}" -eq 0 ]] || sudo -n true 2>/dev/null; then
  sudo unshare --pid --fork --mount-proc bash -c 'echo "  inside PID ns, ps sees:"; ps aux | tail -5'
else
  echo "  skipped — needs sudo to unshare PID namespace"
fi
