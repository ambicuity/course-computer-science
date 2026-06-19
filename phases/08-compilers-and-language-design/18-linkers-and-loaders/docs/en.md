# Lesson 18: Linkers and Loaders

A compiler turns source code into object files. A **linker** combines object files into an executable. A **loader** reads that executable into memory and starts it running. Understanding these tools is essential for diagnosing build errors, optimizing binary size, and writing systems software.

The compilation pipeline: `source.c` → (compiler) → `source.o` → (linker) → `program` → (loader) → running process. Each stage has a distinct responsibility, and errors at the linker or loader stage are among the most frustrating in C/C++ development because they are far from the source code.

## Object Files

An object file is the compiler's output — machine code and data, but not yet a runnable program. The dominant format on Linux is **ELF** (Executable and Linkable Format). Windows uses **PE/COFF**; macOS uses **Mach-O**.

### ELF Sections

| Section | Contents |
|---|---|
| `.text` | Machine code (instructions) |
| `.data` | Initialized writable data |
| `.bss` | Uninitialized data (allocated at load time, zeroed) |
| `.rodata` | Read-only data (string literals, constants) |
| `.symtab` | Symbol table: names and addresses |
| `.rel.text` | Relocation entries for `.text` |
| `.strtab` | String table for symbol names |

An ELF file begins with a header that identifies it (magic bytes `7f 45 4c 46` = "\x7fELF"), specifies the architecture (x86-64, AArch64, etc.), entry point, and offsets to the section header table and program header table.

### The Symbol Table

Every object file exports and imports **symbols** — named addresses. A symbol can be:

- **Defined**: a function or variable with a known address in this file.
- **Undefined**: referenced in this file, but defined elsewhere (another `.o` file or library).
- **Weak**: like a definition, but can be overridden by a strong definition from another file.

The symbol table maps symbol names to section offsets. The linker resolves undefined symbols by finding matching definitions in other object files.

### Relocation

When the compiler generates code, it doesn't know the final addresses of external symbols. It emits placeholder values and records **relocation entries** that tell the linker how to patch them.

Common relocation types on x86-64:

| Type | Meaning |
|---|---|
| `R_X86_64_PC32` | PC-relative 32-bit offset (for calls/jumps) |
| `R_X86_64_64` | Absolute 64-bit address |
| `R_X86_64_PLT32` | PC-relative offset through PLT (for shared libs) |
| `R_X86_64_GOTPCREL` | RIP-relative reference to GOT entry |

A relocation entry says: "at offset X in section Y, apply fixup for symbol Z using type T."

## Static Linking

Static linking combines multiple `.o` files into a single executable:

1. **Parse** all object files and extract sections, symbols, and relocations.
2. **Resolve symbols**: for each undefined symbol, find a definition in another file. If a symbol is multiply-defined or never defined, emit an error.
3. **Assign addresses**: lay out all `.text` sections contiguously, then `.data`, then `.bss`. Each symbol gets a final virtual address.
4. **Apply relocations**: walk every relocation entry, compute the final address, and patch the code or data.
5. **Emit** the output executable with program headers that describe how the loader should map it into memory.

Static linking produces self-contained binaries — no runtime dependencies — but large file sizes.

### Linker Scripts

Real linkers use **linker scripts** (`.ld` files) to control section placement, define symbols, and set the entry point. For example, the default linker script places `.text` at `0x400000` on x86-64. Embedded systems use custom scripts to place code in flash and data in RAM.

## Dynamic Linking

Most modern programs use **shared libraries** (`.so` on Linux, `.dll` on Windows, `.dylib` on macOS). These are linked at **load time** or **run time**, not at build time.

### PLT and GOT

To call a function in a shared library, the compiler emits an indirect call through the **Procedure Linkage Table** (PLT). The PLT is a small stub that jumps through a pointer in the **Global Offset Table** (GOT).

```
# PLT stub for printf
printf@plt:
    jmp *GOT[printf]     # lazy: first call jumps to resolver
    push index            # resolver identifies which symbol
    jmp resolver

# After resolution, GOT[printf] points to actual printf
```

The GOT holds the actual addresses of shared library symbols. On first call, the dynamic linker (`ld-linux.so`) resolves the symbol and patches the GOT entry. Subsequent calls jump directly — this is **lazy binding**.

### Position-Independent Code (PIC)

Shared libraries are loaded at arbitrary addresses. All code must be **position-independent**: no absolute addresses in `.text`. Instead, the code accesses data through the GOT and calls functions through the PLT. On x86-64, RIP-relative addressing makes this efficient.

## The Loader

The **loader** is the OS component (or `execve` syscall) that reads an executable into memory and starts it:

1. **Read the ELF header** and program headers.
2. **Map segments** into the process address space: `.text` (read-execute), `.data`/`.bss` (read-write), stack, heap.
3. **Load dynamic linker** if the executable depends on shared libraries (`ld-linux.so` is listed in `.interp`).
4. **Set up the stack**: push `argc`, `argv`, `envp`, and auxiliary vectors.
5. **Jump to the entry point** (`_start`), which calls `__libc_start_main`, which calls `main`.

The loader uses **memory-mapped files** (`mmap`) to map executable segments directly from the file into the process address space. This is efficient — the OS can page in code on demand rather than reading the entire file upfront.

### ASLR and Relocation

Modern operating systems use **Address Space Layout Randomization** (ASLR) — loading executables and shared libraries at random base addresses to defeat buffer overflow attacks. This requires all shared library code to be position-independent. The loader patches GOT entries and applies base relocations at load time.

## Build It

Examine real object files with tools: `objdump` disassembles code, `nm` lists symbols, `readelf` shows ELF structure, and `ldd` lists shared library dependencies. Then implement a simplified linker that resolves symbols and applies relocations.

## Use It

The standard linkers are **ld** (GNU), **lld** (LLVM — faster, cross-platform), and **gold** (GNU, multi-threaded). On most systems, `gcc` or `clang` invokes the linker behind the scenes. To see the linker command, run with `-v`.

```bash
gcc -v main.o utils.o -o program   # shows the ld invocation
ldd program                         # list shared library deps
readelf -h program                  # ELF header
nm program                          # symbol table
objdump -d program                  # disassembly
```

## Ship It: Linker Demo

A simplified linker demonstrates the core concepts: reading object files, resolving symbols, applying relocations, and producing a combined output. Full linkers handle thousands of edge cases — weak symbols, COMMON blocks, TLS relocations, linker scripts — but the principles are the same.

## Exercises

**Level 1 — Understand**: Run `objdump -d` on a compiled C program that calls `printf`. Find the PLT stub for `printf` and trace how the call flows through the PLT and GOT. What is the difference between the first call and subsequent calls?

**Level 2 — Implement**: Extend the simplified linker to handle **multiple relocation types** — at minimum, `R_X86_64_PC32` (PC-relative) and `R_X86_64_64` (absolute). Test by linking two object files where one calls a function defined in the other.

**Level 3 — Optimize**: Implement **link-time optimization** (LTO) at a basic level: instead of linking object files with opaque machine code, read LLVM/bitcode-like IR from each file, inline small functions across translation units, then emit a single optimized object file. Measure the performance improvement on a program with many small cross-file function calls.
