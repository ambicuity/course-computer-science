# Terminal, Shell, Pipes, Job Control

> The terminal isn't a chat interface. It's a programmable composition layer for processes — learn that, and you can build anything.

**Type:** Learn
**Languages:** Shell
**Prerequisites:** Phase 00, Lesson 01
**Time:** ~60 minutes

## Learning Objectives

- Distinguish *terminal*, *shell*, *process*, and *job* — and explain what each one owns.
- Compose pipelines with stdin/stdout/stderr redirection, understanding when bytes are buffered versus line-buffered.
- Drive jobs in the foreground and background with `&`, `Ctrl-Z`, `fg`, `bg`, `jobs`, `disown`, and `nohup`.
- Read and write a short shell pipeline that solves a real CS task (parsing logs, counting tokens, scanning git history) end-to-end.

## The Problem

For most of CS, the terminal is *not* a place to type commands one at a time. It's the place where small, single-purpose Unix tools compose into ad-hoc programs. The author of `grep` did not know you'd be using it to count error rates in a Kubernetes pod log, and yet you can do that in one line because of how shells, pipes, and processes were designed together in the early 1970s.

When learners struggle with the terminal, the symptom is usually one of these:

- They run a long command, press `Ctrl-C` to "cancel," and the program leaves behind half-written files.
- They press `Ctrl-Z` thinking it's undo, the program suspends, and they don't know how to get it back.
- They redirect a command's output and end up with an empty file because they didn't realize the error went to stderr, not stdout.
- They write a pipeline that mostly works but occasionally produces wrong output because the upstream process is buffering and the downstream one reads partial data.

Every one of those traces back to a piece of the *process model* that the terminal exposes. This lesson lays that model out and gives you the muscle memory to drive it.

## The Concept

### Terminal, shell, process, job — four distinct things

These words are often used interchangeably. They aren't:

| Layer | What it is | Examples |
|-------|-----------|----------|
| **Terminal** | A device (real TTY or pseudo-TTY) that does I/O — bytes in from the keyboard, bytes out to a screen | `xterm`, `iTerm2`, `Alacritty`, the Linux console |
| **Shell** | A program that reads commands from a terminal, parses them, and forks child processes to run them | `bash`, `zsh`, `fish`, `sh` |
| **Process** | A running instance of a program with its own PID, memory, file descriptors | `ls`, `python3 main.py`, `vim` |
| **Job** | One or more processes (a pipeline counts as one job) that the shell tracks together | The "job" `cat foo.txt \| sort \| uniq` is three processes treated as one unit |

This matters because Ctrl-C, Ctrl-Z, `&`, `fg`, and `bg` all act on **jobs**, not on individual processes. When you press Ctrl-C in a pipeline, the shell sends SIGINT to the *whole pipeline's process group*, not just the foreground process.

```
   You ──> Terminal ──> Shell ──> spawns ──> Process(es) grouped into a Job
   (keyboard,                 (parses cmds,
    screen)                    sets up pipes,
                               waits for jobs)
```

### Three file descriptors, three streams

Every Unix process is born with three file descriptors already open:

| FD | Name | Default destination | Convention |
|----|------|---------------------|------------|
| 0 | stdin | terminal keyboard | Input data |
| 1 | stdout | terminal screen | Normal output (data, results) |
| 2 | stderr | terminal screen | Errors, progress, logs |

The convention is the source of many bugs: **errors go to fd 2**. If you redirect *only* fd 1 (`>file`), errors still print to the terminal. To capture both:

```sh
cmd > file 2>&1     # send stderr (fd 2) wherever stdout (fd 1) is going
cmd >& file         # bash/zsh shorthand for the same
cmd > out.txt 2> err.txt   # split them apart
```

### Pipes: byte streams between processes

A pipe `|` connects the stdout of one process to the stdin of the next:

```
A | B   ≡   open a kernel-level pipe;
            fork A with its stdout = write-end;
            fork B with its stdin  = read-end;
            wait for both.
```

Pipes are byte streams, not message streams. The kernel buffers a small amount (typically 64 KiB), and `B` only sees data when `A` flushes its stdio buffer. When `A` is a long-running tool like `tail -f log`, the default block-buffering can cause `B` to see nothing for minutes — see "the buffering trap" below.

### Job control: foreground vs background

A *foreground* job owns the terminal — the shell waits for it, and Ctrl-C and Ctrl-Z reach it. A *background* job runs without the terminal blocking; the shell prints its prompt immediately.

```
$ long-build &                    # start in background
[1] 41273                         # job 1, pid 41273
$ jobs                            # list active jobs
[1]+  Running   long-build
$ fg %1                           # bring job 1 to foreground
...
Ctrl-Z                            # suspend (SIGTSTP)
[1]+  Stopped   long-build
$ bg %1                           # resume in background
$ disown %1                       # detach so the shell won't kill it on exit
```

The terminal sends three special signals from the keyboard:

| Keys | Signal | What happens |
|------|--------|--------------|
| Ctrl-C | SIGINT  | "Interrupt" — most programs exit; some catch and clean up |
| Ctrl-Z | SIGTSTP | "Terminal stop" — process is suspended (resumable with `fg`/`bg`) |
| Ctrl-\\ | SIGQUIT | Like Ctrl-C but produces a core dump |

These don't go to one process; they go to the foreground *process group*, so every process in a pipeline gets them.

### The buffering trap

```sh
tail -f /var/log/syslog | grep ERROR | tee errors.txt
```

You might expect this to write errors to `errors.txt` in real time. It doesn't — by default, `grep` block-buffers when its stdout is *not* a terminal. So `tee` (which writes the file) only sees data when `grep`'s buffer fills up, which might be never on a low-traffic log.

The fix:

```sh
tail -f /var/log/syslog | grep --line-buffered ERROR | tee errors.txt
```

Or use `stdbuf -oL grep ERROR`. The lesson here: pipes don't change the buffering behavior of the programs in them — you have to tell each program to flush per line if you want interactivity.

## Build It

### Step 1: A pipeline that classifies your shell history

We'll write a one-liner that ranks the top 20 commands you use, by frequency. This exercises pipes, sorting, deduping, and field splitting.

```sh
history | awk '{print $2}' | sort | uniq -c | sort -rn | head -20
```

Read it from left to right:

- `history` — dumps the shell history, one entry per line, prefixed by an entry number.
- `awk '{print $2}'` — for each line, print the second whitespace-separated field (the command itself).
- `sort` — sort the commands lexically (required so `uniq` can collapse adjacent duplicates).
- `uniq -c` — replace each run of identical lines with a single line prefixed by its count.
- `sort -rn` — sort numerically, in reverse, by the count column.
- `head -20` — take the top 20.

Five tiny programs, one composable result. None of them knew about your shell, your history file, or each other.

### Step 2: Foreground/background drill

```sh
# Open the terminal and run:
sleep 60                # foreground — terminal is locked
# Press Ctrl-Z          # suspends
bg                      # resume in background
jobs                    # list jobs
fg %1                   # bring it back
# Press Ctrl-C          # kill it
```

Now try a pipeline:

```sh
yes | head -1000000 > /dev/null &
[1] 12345
jobs
# Press Ctrl-C in a foreground process — the pipeline is unaffected
kill %1
```

The key insight: `kill %1` uses the *shell's* job ID, not the OS PID. The shell tracks jobs; PIDs change every run, job IDs are sequential per shell session.

### Step 3: Redirect both streams correctly

Write a small script that prints to both stdout and stderr, then redirect each:

```sh
cat > demo.sh <<'EOF'
#!/usr/bin/env bash
echo "data line"          # stdout
echo "error line" >&2     # stderr
EOF
chmod +x demo.sh

./demo.sh                    # both lines appear on terminal
./demo.sh > out.txt          # only "error line" appears; out.txt has "data line"
./demo.sh 2> err.txt         # only "data line" appears; err.txt has "error line"
./demo.sh > all.txt 2>&1     # all.txt has both, in order they were emitted
./demo.sh &> all2.txt        # bash/zsh shorthand for "> all2.txt 2>&1"
```

The order of `> all.txt 2>&1` matters: `2>&1` says "send fd 2 to wherever fd 1 currently points." If you write `2>&1 > all.txt`, fd 2 ends up pointed at the terminal (where fd 1 was *before* the redirect), not at the file.

### Step 4: A real pipeline — error rates from a log file

Imagine a Kubernetes-style log with lines like:

```
2024-08-12T10:11:22Z level=info  msg="request handled"  status=200
2024-08-12T10:11:24Z level=error msg="db timeout"       status=500
```

Compute the percentage of error lines:

```sh
awk -F'level=' '{print $2}' app.log \
  | awk '{print $1}' \
  | sort \
  | uniq -c \
  | awk 'BEGIN{tot=0} {tot+=$1; counts[$2]=$1} END{
      for (k in counts) printf "%-8s %6d  %5.2f%%\n", k, counts[k], 100*counts[k]/tot
    }'
```

This is the shape of a *real* shell pipeline: each stage is small and replaceable. If `app.log` switches to JSON, swap the first `awk` for `jq -r '.level'` and the rest still works.

### Step 5: Defensive shell habits

```sh
set -euo pipefail        # at the top of every script
# -e: exit on first command error
# -u: error on unset variables
# -o pipefail: a pipeline's exit code is the rightmost non-zero exit, not just the last command
```

Without `pipefail`, `false | true` exits 0, hiding the failure. The course's CI scripts (Lesson 17) all use `set -euo pipefail`.

## Use It

The same primitives — process groups, pipes, file descriptors, signals — sit underneath everything else you'll touch in this course:

- **OS lessons** (Phase 07) reimplement `fork`, `exec`, `pipe`, and `dup2` from scratch. The shell uses exactly those syscalls.
- **Network lessons** (Phase 09) treat sockets as file descriptors — you'll `read()` and `write()` them the same way pipes work, just with the network in the middle.
- **Distributed systems** (Phase 11) extend the pipe idea: a Kafka topic is a pipe across machines, a Spark stage chain is a pipeline whose nodes are processes.

`bash` itself is a worth-reading C program; look at how it calls `pipe(2)` and `fork(2)` in `execute_cmd.c` when it sees a `|` in your command line.

## Read the Source

- `https://git.savannah.gnu.org/cgit/bash.git/tree/execute_cmd.c` — bash's command executor. Function `execute_pipeline` is exactly the kernel-pipe + fork + dup2 dance.
- `https://man7.org/linux/man-pages/man7/pipe.7.html` — the kernel's `pipe(7)` manpage. Read it; pipes are smaller than you think and richer than they look.
- `https://man7.org/linux/man-pages/man3/setbuf.3.html` — stdio buffering modes. This is the page that explains "the buffering trap" you hit in Step 4.

## Ship It

This lesson's reusable artifact is **`outputs/sh-cheatsheet.md`** — a one-page reference of the redirect operators, the job-control commands, and the `set -euo pipefail` boilerplate. Pin it in your tmux scratchpad or your editor's notes.

## Exercises

1. **Easy.** Write a one-line pipeline that prints the 10 largest files (by size) in the current directory tree, sorted descending.
2. **Medium.** Write a pipeline that, given a git repo, prints the 20 authors with the most commits, with each author's commit count. Hint: `git log --format='%an'`.
3. **Hard.** Reproduce `tail -f` plus filtering using only `awk` and shell job control. The result should follow a log file as it's written and print only lines matching a pattern, in real time. (You'll have to confront the buffering trap.)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Pipe | "Sending data" | A kernel object connecting one process's stdout to another's stdin, byte-buffered, blocking when full or empty |
| Job | "A running command" | One or more processes the shell tracks as a unit; targets of `%N` syntax and keyboard signals |
| Background | "Run later" | Run *now*, but without occupying the terminal — the shell returns its prompt immediately |
| stderr | "Errors" | Conventional stream for human-facing diagnostics; separate from stdout so logs and data don't intermix |

## Further Reading

- *The Linux Programming Interface* by Michael Kerrisk — Chapters 24 (process creation), 27 (program execution), and 34 (process groups) cover the model under the shell.
- [GNU Bash Manual — Shell Commands](https://www.gnu.org/software/bash/manual/html_node/Shell-Commands.html) — formal grammar of what `|`, `&`, `;`, `&&`, `||` mean.
- [The Unix Programming Environment](https://en.wikipedia.org/wiki/The_Unix_Programming_Environment) by Kernighan & Pike — old but still the cleanest explanation of pipeline-oriented thinking.
