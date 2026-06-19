# Functions, the Stack, and Calling Conventions

> A function call is a *contract* between caller and callee about who saves which registers, who pushes args, and who cleans up. Hold up your side and the universe stays sane.

**Type:** Build
**Languages:** C, RISC-V Assembly
**Prerequisites:** Phase 02, Lessons 01-03
**Time:** ~75 minutes

## Learning Objectives

- Draw a stack frame: return address, saved frame pointer, callee-saved registers, locals, arg overflow area.
- State the **SysV AMD64** calling convention's main rules: first six integer args in `rdi, rsi, rdx, rcx, r8, r9`; return in `rax`; specific callee/caller-saved register sets.
- Describe what a prologue (`push rbp; mov rbp, rsp; sub rsp, N`) and epilogue (`leave; ret`) do; recognize them in assembly.
- Reproduce, from C code, the assembly the compiler emits for a function call — including argument marshalling and stack cleanup.

## The Problem

The function-call abstraction is one of the deepest in CS: you write `foo(a, b)` and out comes a return value. But beneath that:

- Where do `a` and `b` go before `foo` starts executing? (Registers? Stack?)
- Where does `foo` save the return address so it can come back?
- Which registers must `foo` preserve, and which can it trash?
- Where do local variables live during `foo`'s execution?

Every architecture (x86_64, ARM, RISC-V) and every OS (Linux, macOS, Windows) has its own answer. The set of answers is the **calling convention** (or "ABI" — Application Binary Interface). When you debug crashes, work with assembly, or write FFI bindings, you're working with this contract.

## The Concept

### The stack

The **stack** is a contiguous region of memory that grows down (toward lower addresses on x86 and most modern CPUs). Two registers manage it:

- **`rsp`** (stack pointer) — points to the top (lowest in-use address).
- **`rbp`** (base pointer / frame pointer) — points to the start of the current function's frame. Optional but conventional.

A **frame** is the slice of stack belonging to one running function:

```
   high address                  ↑ caller's frame
   ┌────────────────────────┐
   │  arg 7+, if any        │   pushed by caller
   ├────────────────────────┤
   │  return address        │   pushed by 'call' instruction
   ├────────────────────────┤
   │  saved rbp (old base)  │   pushed by callee's prologue
   ├────────────────────────┤ ← rbp
   │  callee-saved regs     │   pushed by prologue if used
   ├────────────────────────┤
   │  locals                │
   │  …                     │
   ├────────────────────────┤ ← rsp (during execution)
   │  (stack grows down)    │
   ▼   low address
```

### Prologue / epilogue

Compilers emit a standard *prologue* at the start of each function:

```asm
push rbp           ; save caller's frame pointer
mov  rbp, rsp      ; set our frame pointer
sub  rsp, 0x20     ; allocate 32 bytes for locals
```

…and an *epilogue* at the end:

```asm
leave              ; mov rsp, rbp; pop rbp
ret                ; pop return address, jump to it
```

`call` and `ret` are paired: `call foo` pushes the address of the next instruction and jumps; `ret` pops that address back into the program counter.

### SysV AMD64 (Linux & macOS) integer calling convention

| First 6 integer / pointer args | `rdi, rsi, rdx, rcx, r8, r9` |
| Return value | `rax` (low) + `rdx` (high) for 128-bit |
| Floating-point args | `xmm0..xmm7` |
| Beyond the first 6 args | Pushed on the stack, right-to-left |
| Caller-saved (volatile) | `rax, rcx, rdx, rsi, rdi, r8-r11` + xmm0-xmm15 |
| Callee-saved (preserved) | `rbx, rbp, r12-r15`, plus rsp |
| Stack alignment | 16-byte aligned at the call site (before `call`) |

The caller assumes the callee may clobber any caller-saved register; the callee promises to preserve callee-saved registers.

### Windows x64 ABI is different

Same architecture, different convention:

- First 4 args in `rcx, rdx, r8, r9` (32 bytes of *shadow space* reserved by caller).
- Different callee-saved set.

Code compiled against one ABI cannot be linked against the other without a thunk.

### RISC-V calling convention (used in Phase 06)

| First 8 integer args | `a0, a1, a2, ..., a7` (also `x10..x17`) |
| Return value | `a0`, `a1` |
| Callee-saved | `s0..s11` (`x8, x9, x18..x27`) |
| Caller-saved | `t0..t6`, `a0..a7`, `ra` (return address) |

Same conceptual contract, different register names.

### Variable-argument functions (`printf`)

`printf` accepts a variable number of args; the SysV ABI requires the caller to set `al` to the number of XMM register args (for floating-point). Stepping through `printf`'s assembly is the fastest way to internalize the convention.

### Why you care

- **Stack overflow / smashing**: writing past a buffer onto the saved return address gives an attacker control of the program counter (the classic security exploit). Phase 12 covers stack canaries, NX bit, and ASLR.
- **Reverse engineering**: any binary you disassemble follows the ABI; identifying prologues and parameter loads is how you find function boundaries.
- **FFI**: calling C from Rust / Python / Go requires matching the ABI on both sides.
- **Tail calls**: TCO is only legal when the call respects the ABI (matching args setup, register clobbers).

## Build It

Open `code/main.c`. We'll write a tiny function and inspect the assembly produced.

### Step 1: Build with `-S` to emit assembly

```sh
cd code/
gcc -O0 -S main.c -o main.s
cat main.s   # platform assembly; differs by ARM vs x86_64
```

### Step 2: Identify the prologue, epilogue, and arg placement

For each function, find:

- The prologue (`push %rbp`, `mov %rsp, %rbp`, `sub $..., %rsp` on x86_64; `stp x29, x30, [sp, #-N]!` on ARM64).
- Where arguments are read from (`%edi`, `%esi`, … or `w0`, `w1`, …).
- The body.
- The epilogue (`leaveq`, `ret` or `ldp x29, x30, [sp], #N; ret`).

### Step 3: Trace a deliberate recursive call

`factorial(n) = n * factorial(n - 1)`. Watch the stack grow:

```sh
gcc -O0 -g main.c -o main
gdb ./main
(gdb) b factorial
(gdb) r
(gdb) bt          # see the stack
(gdb) c           # next call
(gdb) bt          # one frame deeper
```

### Step 4: Stack-overflow demo

A function that allocates a 1 KB array and recurses unconditionally. Hitting the OS-default 8 MB stack limit:

```c
void recurse(void) {
    char filler[1024];
    filler[0] = 0;
    recurse();
}
```

`./main_overflow` will SIGSEGV after ~8000 calls. Read the core dump (Phase 00 Lesson 07).

### Step 5: Read the lesson's `main.s`

The repo's `code/main.s` is a hand-written RISC-V assembly file implementing `factorial`. It shows the same contract on a different ISA — `ra` is the return-address register, `s0` (callee-saved) holds n across the recursive call.

## Use It

- **Debugging**: every gdb `bt` reads the chain of saved `rbp`s.
- **Profiling** (Phase 00 Lesson 08): perf walks the same chain to attribute samples.
- **JIT compilers** (Phase 08): emit ABI-compliant code or risk crashes when calling library functions.
- **Security exploits** (Phase 12): stack smashing and ROP gadget chains all rely on knowing where the return address lives.

## Read the Source

- [System V Application Binary Interface — AMD64](https://gitlab.com/x86-psABIs/x86-64-ABI) — the spec, downloadable PDF.
- *Programming from the Ground Up* by Jonathan Bartlett — free book on x86 assembly; Chapter 4 is the calling convention.
- [Eli Bendersky's assembly tutorials](https://eli.thegreenplace.net/category/programming/assembly) — clear, modern.

## Ship It

This lesson ships **`outputs/abi-cheatsheet.md`** — a one-page reference for SysV AMD64, Windows x64, and RISC-V calling conventions side by side.

## Exercises

1. **Easy.** Write a 3-arg function in C; build with `gcc -O0 -S`; identify which register each arg uses on x86_64 or ARM64.
2. **Medium.** Implement `factorial(n)` in raw assembly (x86_64 or RISC-V); link with the C runtime; call it from main. Match the platform's ABI.
3. **Hard.** Demonstrate a stack overflow with a buffer write that smashes the return address. Compile with and without `-fstack-protector` (a canary). Observe the difference.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Stack frame | "Function's slot on the stack" | Memory region containing saved registers, locals, and (sometimes) overflow args for one running call |
| Prologue/epilogue | "Setup/teardown code" | Compiler-emitted instructions saving the caller's state and allocating locals (prologue), then restoring and returning (epilogue) |
| Calling convention / ABI | "Rules for calling functions" | The contract specifying argument placement, return-value placement, register save responsibilities, and stack alignment |
| Callee-saved | "Preserved across calls" | Registers the called function must restore before returning (the caller can rely on their value) |
| Caller-saved | "Volatile" | Registers the caller must save itself if it wants to preserve them across a call |

## Further Reading

- *The Linkers and Loaders book* (Levine), Chapter 1 — gives historical context for why ABIs differ.
- *Optimizing Software in C++* (Agner Fog) — appendix B has detailed register-usage tables.
- [The Linux `man 7 syscalls`](https://man7.org/linux/man-pages/man2/syscall.2.html) — the system-call ABI is its own subset of the calling convention.
