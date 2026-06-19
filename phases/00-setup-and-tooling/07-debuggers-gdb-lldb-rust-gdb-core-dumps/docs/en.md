# Debuggers — gdb, lldb, rust-gdb, core dumps

> A debugger isn't for catching bugs. It's a programmable lens on a running program. Learn the lens and the bugs reveal themselves.

**Type:** Build
**Languages:** C, Rust
**Prerequisites:** Phase 00, Lessons 04–05
**Time:** ~75 minutes

## Learning Objectives

- Drive `gdb` or `lldb` through the standard inspection loop: set a breakpoint, run, step, print, continue, finish.
- Inspect program state — locals, args, the call stack, raw memory, watch expressions — and explain what each one *physically* is.
- Capture a core dump and do post-mortem debugging: load the dump, view the stack at crash time, find the offending instruction.
- Apply the same workflow to Rust binaries via `rust-gdb` / `rust-lldb`, with Rust-specific pretty-printers.

## The Problem

When a small program misbehaves, `printf` debugging works. When the bug is in a 500-line function, an off-by-one in pointer arithmetic, a use-after-free, or a deadlock between two threads, `printf` becomes a torture device — you keep adding prints, recompile, rerun, narrow your guess, repeat. Hours go by.

A debugger collapses that loop. It freezes the program at a specific instruction, lets you read every value in scope, lets you step instruction by instruction, lets you change values *in place* and continue. When a program crashes in production, a debugger lets you load the corpse (the core dump) and ask "what was the call stack at the moment of death?"

You don't need to memorize all the commands. You need a working subset and a model of what the debugger is doing. This lesson gets you both, in C and Rust.

## The Concept

### What the debugger sees

When a debugger attaches to (or launches) a program, it uses OS facilities (`ptrace` on Linux, `mach_*` on macOS) to:

1. Read and write the target's memory.
2. Pause the target on a chosen instruction (breakpoint = "replace this byte with INT 3; on the trap, return control to the debugger").
3. Read CPU registers — including the program counter `rip`/`pc`.
4. Map raw addresses back to source-line/variable names via *debug info* (DWARF, baked into the binary when you compile with `-g`).

Without `-g`, the debugger still works but you'll only see addresses, not names. Always compile dev builds with `-g` (Rust: `cargo build` is dev profile, which has `debug = true` by default).

```
     source.c ──[ gcc -g ]──> a.out (code + DWARF debug info)
                                         │
                                         ▼
                                       gdb a.out
                                         │
                                         ▼ (ptrace)
                                   ┌──── target process ────┐
                                   │   registers, memory,    │
                                   │   stack, breakpoints    │
                                   └─────────────────────────┘
```

### The standard loop

Five verbs. Most sessions use only these:

| Verb | gdb | lldb | What it does |
|------|-----|------|--------------|
| set breakpoint | `b <func>` or `b file:line` | `b <func>` or `br set --file f --line N` | Stop when execution reaches that location |
| run | `r [args]` | `run [args]` | Start the program (with args after `--`) |
| step into | `s` | `s` / `step` | Execute one source line; descend into calls |
| step over | `n` | `n` / `next` | Execute one source line; treat calls as atomic |
| continue | `c` | `c` / `continue` | Resume until the next breakpoint or exit |
| print expression | `p <expr>` | `p <expr>` | Evaluate an expression in the target's state |
| backtrace | `bt` | `bt` | Print the call stack at the current PC |
| finish | `fin` | `fin` | Run until the current function returns |
| quit | `q` | `q` | Exit the debugger |

`lldb`'s commands are also available in a tidier form (`breakpoint set`, `frame variable`, `thread step-in`), but the gdb-compatible shortcuts work too.

### Inspecting state

Once stopped at a breakpoint:

- **Locals and args:** `info args`, `info locals` (gdb) or `frame variable` (lldb).
- **The call stack:** `bt` shows each frame's function, source line, and arguments.
- **A frame:** `frame 2` (or `f 2`) — switch to frame #2; `info locals` then shows that frame's locals.
- **Raw memory:** `x/16xb 0x7fff...` (gdb) — examine 16 bytes in hex, byte by byte.
- **Watchpoints:** `watch counter` — stop when `counter`'s memory changes. Crucial for "who's mutating this?"
- **Conditional breakpoints:** `b foo if x > 10` — useful when a function is called a million times and you only care about the one bad input.

### Core dumps: post-mortem debugging

When a program crashes (SIGSEGV, SIGABRT, etc.), the OS can write its memory image to disk as a "core dump." You can then load the dump in a debugger long after the process is gone:

```sh
ulimit -c unlimited       # enable core dumps in this shell
./crash                   # ... it crashes ...
gdb ./crash core          # or:  lldb ./crash -c core
(gdb) bt                  # see the stack at the moment of death
```

On Linux, system policy may write cores to `/var/lib/systemd/coredump/` and you retrieve them via `coredumpctl list` / `coredumpctl debug`. On macOS, cores go under `/cores/`.

### Rust-specific: pretty-printers

Rust's `String`, `Vec`, `HashMap`, etc. are not raw C structs — they have layout that's not obvious in raw memory. `rust-gdb` and `rust-lldb` are wrappers that load *Rust pretty-printers* so:

```
(gdb) p my_vec
$1 = Vec(size=3) = {1, 2, 3}     ← rust-gdb formats this nicely
```

Without the pretty-printers you'd see raw fields (`{ ptr: 0x..., len: 3, cap: 4 }`). Use the wrappers for any Rust debugging.

## Build It

### Step 1: Build the C example with debug info

The `code/main.c` is a small program with a bug: it has an array index off by one, and it has a deliberate `abort()` path when given a specific input.

```sh
cd code/
gcc -g -O0 main.c -o main          # -g for debug info, -O0 to avoid optimizations
./main 5                            # works
./main 100                          # crashes (or asserts)
```

### Step 2: Drive gdb (or lldb) through the loop

```sh
gdb ./main
(gdb) b main
Breakpoint 1 at 0x...: file main.c, line 18.
(gdb) r 5
Starting program: ./main 5
Breakpoint 1, main (argc=2, argv=0x...) at main.c:18
(gdb) n                # step over `int n = atoi(argv[1]);`
(gdb) p n
$1 = 5
(gdb) n
(gdb) p sum
$2 = 15
(gdb) c                # continue to exit
```

On macOS where `lldb` is the default:

```sh
lldb ./main
(lldb) b main
(lldb) r 5
(lldb) n
(lldb) p n
(lldb) c
```

The commands you typed in one are valid in the other (modulo a few aliases).

### Step 3: Find the off-by-one with a watchpoint

`main.c` writes to `arr[n]` instead of `arr[n-1]`. Find it without reading the source carefully:

```sh
gdb ./main
(gdb) b main.c:30          # the line that writes to arr[n]
(gdb) r 5
(gdb) p &arr[0]            # base address of the array
(gdb) p &arr[4]            # last valid element
(gdb) p &arr[5]            # one past the end — about to be written
(gdb) watch arr[5]         # break on next write to the bad slot
(gdb) c
```

You'd be watching the offending write right there. Then you can read the source and fix it.

### Step 4: Capture a core dump

```sh
ulimit -c unlimited
./main 999                  # path designed to abort()
ls core*                    # or:  coredumpctl list (systemd-linux)
gdb ./main core             # load executable + core
(gdb) bt
(gdb) frame 0
(gdb) info locals
(gdb) p some_variable
```

You're inspecting a corpse. Same commands, but you can't `continue` — the process is dead.

### Step 5: Rust debugging

Build the Rust example with debug info (cargo dev profile is debug by default):

```sh
cd code/
rustc -g main.rs -o main_rs        # or build via cargo
rust-gdb ./main_rs                  # uses gdb + Rust pretty-printers
(rust-gdb) b main::main
(rust-gdb) r 5
(rust-gdb) n
(rust-gdb) p v             # rust-gdb formats Vec, String, etc. nicely
```

On macOS, `rust-lldb` is the equivalent.

## Use It

These same primitives extend everywhere:

- **Kernel debugging** (Phase 07): use `gdb` attached to a running QEMU with `-s -S` to step the boot of a kernel you wrote.
- **Multi-threaded debugging**: `info threads` shows all threads; `thread 3` switches frame; `set scheduler-locking on` freezes the others. Crucial for race-condition triage.
- **Remote debugging**: `gdb` can talk to `gdbserver` over TCP; you debug a target on an embedded device or a different machine.
- **Reverse debugging**: `gdb`'s `record` mode (or `rr` from Mozilla) lets you step *backwards* in time over the recorded execution. Game-changer for "what's the previous state of this variable?"

## Read the Source

- [GDB manual — chapter 5 (Breakpoints) and 9 (Examining data)](https://sourceware.org/gdb/current/onlinedocs/gdb.html/) — the working subset.
- [LLDB Tutorial](https://lldb.llvm.org/use/tutorial.html) — short, official, gets you to parity with gdb.
- [`rr` (Mozilla)](https://rr-project.org/) — record-and-replay debugger for Linux. Once you've used `rr` you understand why people complain about plain forward-only debuggers.
- [The DWARF Debugging Standard](https://dwarfstd.org/) — the format that lets the debugger map addresses back to source lines and variable names.

## Ship It

This lesson ships **`outputs/debug-checklist.md`** — a one-page "when a binary crashes" checklist: collect the core, take the backtrace, capture environment, attach the symbols.

## Exercises

1. **Easy.** Compile `main.c` with and without `-g` and load each in `gdb`. In the `-g` build, set a breakpoint at `main` and `print` a local variable. In the non-`-g` build, do the same — what's different about the output?
2. **Medium.** Use a *conditional breakpoint* (`b file:line if expr`) to break only on the 100th iteration of a loop. Verify with `info breakpoints` and inspect the iteration counter when it fires.
3. **Hard.** Trigger a core dump from `main.c` and produce a backtrace that shows the precise line of the crash, the values of the arguments, and the value of the global `g_state` at the time of the abort. Save the gdb session as a script (`.gdbinit`) you could re-run.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Breakpoint | "Pause on this line" | A byte in the program text replaced with INT 3 / BRK; when execution hits it, the OS notifies the debugger |
| Watchpoint | "Pause on variable change" | A hardware-assisted (DR registers on x86) or software watch over a memory location |
| Core dump | "Crash file" | The OS's snapshot of process memory at the moment of crash, plus register state |
| DWARF | "Debug info" | The format encoding source-line / variable / type info inside the binary, generated by `-g` |
| Pretty-printer | "Nicely formatted output" | A Python extension that teaches gdb/lldb how to display non-trivial types (`std::vector`, Rust's `String`) |

## Further Reading

- *Advanced Programming in the UNIX Environment* (Stevens & Rago) — Chapter 10 (Signals) and 18 (Terminal I/O) cover the OS underpinnings of `ptrace` and core dumps.
- [Brendan Gregg's "Linux Crisis Tools"](https://www.brendangregg.com/blog/2024-03-24/linux-crisis-tools.html) — `gdb` is there; so is everything you'd reach for next.
- [The Art of Debugging with GDB and DDD](https://nostarch.com/debugging.htm) — the book on it.
