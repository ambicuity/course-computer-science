# Shell Cheat Sheet

> One page. Pin it. Reference it for the rest of the course.

## Redirects

| Syntax | Meaning |
|--------|---------|
| `cmd > file`       | stdout → file (truncate) |
| `cmd >> file`      | stdout → file (append) |
| `cmd 2> file`      | stderr → file |
| `cmd > out 2> err` | split stdout/stderr |
| `cmd > file 2>&1`  | both → file (order matters — redirect stdout first, then dup stderr to it) |
| `cmd &> file`      | bash/zsh shorthand for the above |
| `cmd < file`       | file → stdin |
| `a | b`            | a's stdout → b's stdin |
| `a |& b`           | a's stdout AND stderr → b's stdin (bash 4+) |

## Job control

| Command / key | Effect |
|---------------|--------|
| `cmd &`      | start backgrounded |
| `Ctrl-Z`     | suspend foreground job (SIGTSTP) |
| `Ctrl-C`     | interrupt foreground job (SIGINT) |
| `jobs`       | list jobs in this shell |
| `fg %N`      | bring job N to foreground |
| `bg %N`      | resume job N in background |
| `kill %N`    | kill via job id |
| `disown %N`  | detach so shell won't HUP it on exit |
| `nohup cmd & ` | start a job that survives terminal close |

## Defensive script header

```sh
#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'        # safer field splitting in for-loops
```

| Flag | What it does |
|------|--------------|
| `-e` | exit on any command failure |
| `-u` | error on unset variable |
| `-o pipefail` | pipeline exit = rightmost non-zero |
| `IFS` reset | prevent surprises in `for f in $(...)` |

## Buffering trap

Long-running pipelines stall if intermediate stages block-buffer:

```sh
tail -f log | grep --line-buffered ERROR | tee errors.txt
# or
tail -f log | stdbuf -oL grep ERROR | tee errors.txt
```

## Common pipelines

```sh
# top 10 largest files
du -ah . | sort -rh | head -10

# count unique IPs in nginx log
awk '{print $1}' access.log | sort | uniq -c | sort -rn | head

# git commit count per author
git log --format='%an' | sort | uniq -c | sort -rn

# kill all processes matching a pattern
pkill -f 'python my_server.py'

# follow a log and highlight ERROR lines
tail -f app.log | grep --color=always --line-buffered -i error
```

## Exit codes

- `0` — success
- `1`–`125` — generic error (program-defined)
- `126` — command found but not executable
- `127` — command not found
- `128 + N` — killed by signal N (e.g., `137` = SIGKILL = 128+9)

`echo $?` after any command shows its exit code.
