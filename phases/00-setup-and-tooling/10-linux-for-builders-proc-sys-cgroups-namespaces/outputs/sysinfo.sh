#!/usr/bin/env bash
# sysinfo.sh — one-page host summary using /proc, /sys, ip, free, etc.
# Linux-focused. macOS falls back to sysctl-based equivalents.

set -uo pipefail

hr() { printf "\n── %s ──────────────────────────────────────\n" "$1"; }

hr "host"
hostname
date

OS="$(uname -s)"
if [[ "$OS" == "Linux" ]]; then
  hr "kernel / distro"
  uname -srv
  [[ -f /etc/os-release ]] && grep '^PRETTY_NAME' /etc/os-release | cut -d= -f2-

  hr "cpu"
  awk -F: '/model name/ {print "  "$2; exit}' /proc/cpuinfo
  echo "  cores online: $(nproc)"
  echo "  load (1/5/15): $(cat /proc/loadavg | awk '{print $1, $2, $3}')"

  hr "memory"
  awk '/^MemTotal|^MemAvailable|^Buffers|^Cached|^SwapTotal|^SwapFree/{print "  " $0}' /proc/meminfo

  hr "uptime"
  awk '{secs=int($1); h=int(secs/3600); m=int((secs%3600)/60);
        print "  up "h"h "m"m"}' /proc/uptime

  hr "top 5 processes by RSS"
  for s in /proc/[0-9]*/status; do
    pid=$(basename "$(dirname "$s")")
    name=$(awk '/^Name:/{print $2}' "$s" 2>/dev/null)
    rss=$(awk '/^VmRSS:/{print $2}' "$s" 2>/dev/null)
    [[ -n "$rss" ]] && echo "$rss $pid $name"
  done 2>/dev/null | sort -rn | head -5 | awk '{printf "  %8s KB  pid=%-6s  %s\n", $1, $2, $3}'

  hr "network interfaces"
  if command -v ip >/dev/null 2>&1; then
    ip -br addr 2>/dev/null | sed 's/^/  /'
  else
    cat /proc/net/dev | tail -n +3 | awk '{print "  "$1, "rx="$2, "tx="$10}'
  fi

  hr "block devices"
  if command -v lsblk >/dev/null 2>&1; then
    lsblk -no NAME,SIZE,TYPE | sed 's/^/  /'
  fi

  hr "cgroup root"
  if mount | grep -q cgroup2; then
    ls /sys/fs/cgroup/ | head -10 | sed 's/^/  /'
    echo "  controllers: $(cat /sys/fs/cgroup/cgroup.controllers 2>/dev/null)"
  else
    echo "  cgroup v2 not mounted"
  fi
else
  hr "kernel"
  uname -srv

  hr "cpu"
  echo "  $(sysctl -n machdep.cpu.brand_string 2>/dev/null)"
  echo "  cores: $(sysctl -n hw.ncpu)"

  hr "memory"
  echo "  total: $(sysctl -n hw.memsize | awk '{printf "%.1f GB\n", $1/1073741824}')"
  if command -v vm_stat >/dev/null 2>&1; then
    vm_stat | head -10 | sed 's/^/  /'
  fi

  hr "top 5 by RSS (ps)"
  ps -A -o pid=,rss=,comm= | sort -rnk2 | head -5 | sed 's/^/  /'

  hr "network interfaces (ifconfig)"
  ifconfig -l | tr ' ' '\n' | head -5 | sed 's/^/  /'
fi
