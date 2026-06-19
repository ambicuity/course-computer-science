# What Is a Program, Really (Compilation, Linking, Loading)

> A program is a sequence of *bytes* the kernel maps into memory and asks the CPU to execute. Source code is one layer in a stack of representations. Knowing the whole stack means knowing what's running.

**Type:** Build
**Languages:** C, Shell
**Prerequisites:** Phase 00, Lessons 04, 07
**Time:** ~60 minutes

## Learning Objectives

- Identify the stages turning C source into a running process: preprocessing, compilation, assembly, linking, then loading.
- Inspect a binary's ELF (Linux) or Mach-O (macOS) sections: .text, .data, .bss, .rodata, the symbol table.
- Explain what static and dynamic linking do at load time, and what the dynamic loader (`ld-linux.so` / `dyld`) does to start a process.
- Match each layer in `/proc/<pid>/maps` (or `vmmap` on macOS) to what's there: text, data, heap, stack, mmap'd libraries.

## The Problem

Most programmers can write code and not think about what happens between `gcc hello.c` and the executable actually running. That gap is fine until something breaks in it:

- "Why does my binary segfault on a different machine?" (Glibc version mismatch, dynamic library issues.)
- "Why does this `#define` not take effect?" (Preprocessor question.)
- "Why is my binary 200 MB?" (Static linking pulled the world in.)
- "Why does `LD_PRELOAD` matter for security?" (Dynamic loader hijack.)

Each is a question about the compilation-and-loading pipeline. This lesson opens it up.

## The Concept

### The pipeline

```
hello.c
  │  preprocessor (cpp / gcc -E)
  ▼
hello.i    (preprocessed: includes inlined, macros expanded)
  │  compiler (cc1 / gcc -S)
  ▼
hello.s    (assembly for your target architecture)
  │  assembler (as / gcc -c)
  ▼
hello.o    (relocatable object: machine code + symbol table + relocs)
  │  linker (ld / gcc)
  ▼
hello      (executable: machine code + program headers + section info)
  │  kernel exec + dynamic loader (ld-linux.so / dyld)
  ▼
running process (instructions executing, memory mapped)
```

### What's in an object file

A `.o` (and an executable) is divided into **sections**:

| Section | Contents | Properties |
|---------|----------|-----------|
| `.text` | Machine instructions | Read + execute, not write |
| `.data` | Initialized writable global vars | Read + write |
| `.bss` | Uninitialized globals (zero-filled at load) | Read + write |
| `.rodata` | String literals, const tables | Read-only |
| `.symtab` | Symbol table: function and global names → addresses | Used by the linker |
| `.rel.*` | Relocation info — "patch this address when location of X is decided" | Used by the linker |
| Debug sections (DWARF) | Source-line, type, variable info | `-g` |

Inspect with:

- Linux: `readelf -h hello.o`, `objdump -d hello.o`, `nm hello.o`.
- macOS: `otool -h hello`, `nm hello`, `dwarfdump hello`.

### What the linker does

Two main jobs:

1. **Section merging**: combine `.text` from every `.o` into one big `.text`; same for `.data`, `.bss`, `.rodata`.
2. **Symbol resolution**: replace each "reference to function X" with the address of X's definition (which might be in another `.o` or a library).

### Static vs dynamic linking

- **Static linking**: copy the relevant code from every static library (`libfoo.a`) into the executable. Self-contained; no runtime dependency. Bigger binary.
- **Dynamic linking**: leave a stub in the binary referring to `libfoo.so`. At load time, the *dynamic linker* (`ld-linux.so` on Linux, `dyld` on macOS) maps the shared library into the process and patches the stubs.

```
   static:   hello [text + libc + other libs all merged]
   dynamic:  hello [text + stubs] + ld.so loads libc.so at runtime
```

### Loading: from disk to running

When you `exec` a binary, the kernel:

1. Parses the executable headers (ELF / Mach-O).
2. **mmap's** the file's segments into the process's virtual address space — `.text` read-only-executable, `.data` read-write, etc. Zero-fills `.bss`. Allocates a stack.
3. If dynamically linked, transfers control to the dynamic loader (`ld-linux.so`), which:
   - Maps every required `.so`.
   - Resolves remaining symbol references (lazy binding via the PLT, or eager).
   - Calls each library's initialization functions.
4. Jumps to `_start` in the executable, which eventually calls `main`.

### Process memory layout (Linux x86_64, approximate)

```
   high address
       ┌────────────────────────┐
       │ kernel space (out of   │
       │ user-visible range)    │
       ├────────────────────────┤
       │ stack (grows down)     │
       │   ↓                    │
       │                        │
       │  mmap'd libraries,     │
       │  big malloc allocs     │
       │                        │
       │   ↑                    │
       │ heap (grows up via brk)│
       ├────────────────────────┤
       │ .bss                   │
       │ .data                  │
       │ .rodata                │
       │ .text                  │
       └────────────────────────┘
   low address
```

`/proc/<pid>/maps` (Linux) or `vmmap <pid>` (macOS) shows you this layout for a live process.

## Build It

Open `code/main.c`. The C file is a 30-line hello-world with a global initialized variable, a global uninitialized variable, a `const` string, and a `static` local — one example per section.

### Step 1: Walk the pipeline

```sh
cd code/
gcc -E main.c -o main.i      # preprocessed
gcc -S main.c -o main.s      # assembly
gcc -c main.c -o main.o      # object
gcc main.o -o main           # executable
./main
```

### Step 2: Inspect the object file

Linux: `readelf -h main.o`, `readelf -S main.o`, `readelf -s main.o`, `objdump -d main.o`.
macOS: `otool -h main`, `otool -tV main`, `nm main`.

Match each global / function / string literal in the source to a symbol-table entry.

### Step 3: Static vs dynamic linking

```sh
gcc main.c -o main_dyn         # default: dynamic
gcc main.c -o main_stat -static   # Linux; on macOS dynamic is default

file main_dyn                  # dynamically linked
file main_stat                 # statically linked (Linux)
ls -la main_dyn main_stat      # size difference often 100x+
ldd main_dyn                   # list dynamic deps (Linux)
otool -L main_dyn              # macOS equivalent
```

### Step 4: See the process memory layout

`/proc/<pid>/maps` on Linux or `vmmap <pid>` on macOS. The lesson's `run.sh` script picks the right tool.

### Step 5: LD_PRELOAD demo (Linux) — the dynamic-loader hijack

A tiny shim overrides `puts()` and intercepts every call. This is the foundation of `ltrace`, fault injection, mock libraries, and lots of attack surface.

## Use It

- **Debugging missing-symbol errors**: read the linker's "undefined reference to X" with `nm` and `readelf -s` to see which `.o` defines X.
- **Reducing binary size**: `strip --strip-debug`, `-fdata-sections -ffunction-sections -Wl,--gc-sections`, `-flto`, replacing static linking with dynamic.
- **Reverse engineering / malware analysis**: every binary obeys the rules in this lesson; the ELF/Mach-O parser is your starting point.
- **Reproducible builds**: bit-for-bit identical binaries from identical source require taming the entire pipeline.
- **Security**: PIE (Position-Independent Executable), RELRO, BIND_NOW, NX, ASLR — each is a defense at one of the pipeline's stages.

## Read the Source

- *Linkers and Loaders* by John Levine — free online; the textbook.
- [Eli Bendersky's "Position-Independent Code"](https://eli.thegreenplace.net/2011/11/03/position-independent-code-pic-in-shared-libraries) — clearest blog on PIC vs absolute.
- [The ELF specification (System V ABI)](http://refspecs.linuxfoundation.org/elf/gabi4+/contents.html) — for when you really need to parse the bytes.

## Ship It

This lesson ships **`outputs/inspect-binary.sh`** — a script that takes a binary path and prints section sizes, dynamic deps, and a summary of `/proc/<pid>/maps` for a sample run.

## Exercises

1. **Easy.** Compile `main.c` four different ways (`-O0 -g`, `-O0`, `-O2`, `-Os`). Compare binary sizes and section sizes with `size main`.
2. **Medium.** Use `nm -S main_dyn | sort -k2 -r` (or `otool -tS`) to find the largest functions in the binary. Often `__libc_start_main` is the surprise.
3. **Hard.** Write a minimal LD_PRELOAD shim that wraps `malloc`/`free`, prints each allocation, and forwards to the real glibc allocator via `dlsym(RTLD_NEXT, "malloc")`. Verify on the lesson's `main` binary.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Object file | ".o" | Relocatable machine code + symbol table; not yet runnable |
| Linker | "Joins .o files" | Section merging + symbol resolution + relocation; emits the executable |
| Dynamic loader | "Magic that loads .so files" | Userspace program (ld-linux.so / dyld) the kernel hands control to at exec; resolves dynamic symbols and maps libraries |
| `.bss` | "Uninitialized globals" | A section whose size is in the binary but whose contents are zero-filled at load — saves disk space |
| PLT / GOT | "Dynamic resolution tables" | The procedure linkage table (call stubs) and global offset table (resolved addresses) for lazy dynamic-symbol binding |

## Further Reading

- *Computer Systems: A Programmer's Perspective* (Bryant & O'Hallaron), Chapter 7 — definitive linking treatment.
- [LWN's "How programs get run" series](https://lwn.net/Articles/630727/) — from kernel exec to userspace, line by line.
- *Practical Binary Analysis* by Dennis Andriesse — modern tour of ELF, dynamic loading, and binary instrumentation.
