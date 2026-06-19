# C/C++ Toolchain — gcc, clang, ld, ar, make

> "Compile and run" is four programs in a trenchcoat. See each one and the trenchcoat falls off.

**Type:** Build
**Languages:** C, Makefile
**Prerequisites:** Phase 00, Lessons 01–03
**Time:** ~75 minutes

## Learning Objectives

- Run each stage of the C compilation pipeline by hand (`cpp` → `cc1` → `as` → `ld`) and inspect its output.
- Build a multi-file program with a hand-written `Makefile`, including incremental rebuilds via dependency tracking.
- Create and link a static archive (`libfoo.a`) with `ar`, and a shared library (`libfoo.so` / `.dylib`) with `gcc -shared`.
- Diagnose the three classes of error that the toolchain produces: preprocessor errors, compile errors, linker errors — and explain *which tool* produced each.

## The Problem

When you type `gcc hello.c -o hello`, four separate programs run in sequence. Most of the time you don't have to care, but the moment something goes wrong, error messages reference different tools:

```
hello.c:3:10: fatal error: stio.h: No such file or directory          ← preprocessor
hello.c:7:5: error: 'printf' undeclared (first use in this function)   ← compiler
/usr/bin/ld: /tmp/cc12abcd.o: undefined reference to `printf'          ← linker
```

Three errors, three tools. Knowing which tool produced which error tells you where to look:

- Preprocessor errors → check `#include` paths, macros.
- Compiler errors → check syntax, types, declarations.
- Linker errors → check missing libraries, mismatched function signatures across translation units.

This lesson takes the trenchcoat off `gcc` and shows you each program. You'll never again wonder "is this a compile error or a link error?"

## The Concept

### The C compilation pipeline

```
   hello.c
      │
      ▼  (1) preprocess: expand #include, expand #define, strip comments
   hello.i                                ─── cpp / gcc -E
      │
      ▼  (2) compile: parse C, type-check, emit assembly
   hello.s                                ─── cc1 / gcc -S
      │
      ▼  (3) assemble: convert assembly to machine code
   hello.o                                ─── as / gcc -c
      │
      ▼  (4) link: resolve symbols across .o files and libraries
   hello (or a.out)                       ─── ld / gcc (default)
```

A program of any size has many `.o` files. The linker's job is to splice them into one executable, resolving each external reference (`call printf`, e.g.) to the actual address of the function — either inside another `.o`, inside an archive (`.a`), or inside a shared library (`.so`/`.dylib`).

### Translation units, declarations, definitions

A **translation unit** is one `.c` file after preprocessing. It compiles into one `.o` file. The compiler only sees one translation unit at a time — it does NOT know about other `.c` files. That's why C requires you to:

- **Declare** functions and globals you'll use (usually via a header `.h` file).
- **Define** them exactly once across all translation units.

A "multiple definition" linker error means you defined a symbol in more than one `.o`. A "undefined reference" means you declared it but never defined it (or you forgot to link the `.o` that defines it).

### Static vs shared libraries

| | Static (`libfoo.a`) | Shared (`libfoo.so`, `libfoo.dylib`) |
|--|---------------------|--------------------------------------|
| What it is | An archive (`ar`) of `.o` files | A loadable binary the OS can map into memory |
| When code is included | At link time — copied into the executable | At load/run time — mapped from disk |
| Executable size | Larger (carries the code) | Smaller (just a reference) |
| Multiple processes sharing | Each process has its own copy | One copy in RAM, mapped into many processes |
| Upgradeable without recompiling | No — must relink the executable | Yes — replace the `.so` and restart |

The big-picture trade: static linking gives you a self-contained, hermetic binary; shared linking gives you smaller binaries, faster startup of well-used libraries, and the ability to ship security patches in one library that all dependents pick up.

### Make: incremental, dependency-tracked builds

`make` is a small declarative language: rules of the form

```make
target: prerequisites
<TAB>command
```

`make` rebuilds a target only when one of its prerequisites is newer. The build graph is just the rules: targets become prerequisites of other targets, and the whole DAG gets walked.

The most important Make features for C work:

- Pattern rules: `%.o: %.c\n\t$(CC) -c $< -o $@` — "any `.c` file becomes the corresponding `.o`."
- Automatic variables: `$@` (target), `$<` (first prereq), `$^` (all prereqs).
- Phony targets: `.PHONY: clean all test` — names that aren't files.
- Variables: `CC = gcc`, `CFLAGS = -Wall -O2`.

## Build It

We'll build a tiny library `libgreet.a` and an executable `greet` that links against it, walking each stage of the toolchain.

### Step 1: Run each stage of the pipeline by hand

The `code/` folder has `main.c`. Walk the pipeline:

```sh
cd code/

# (1) Preprocess only — expand includes and macros
gcc -E main.c -o main.i
wc -l main.c main.i              # main.i is much longer (stdio.h pulled in)
head -20 main.i                  # see what the preprocessor expanded

# (2) Compile to assembly — no .o yet
gcc -S main.c -o main.s
head -30 main.s                  # human-readable assembly

# (3) Assemble to object code — relocatable, not yet runnable
gcc -c main.c -o main.o
file main.o                      # "ELF 64-bit ... relocatable"

# (4) Link — produce the executable
gcc main.o -o main
file main                        # "executable" or "Mach-O 64-bit ..."
./main
```

Same result as `gcc main.c -o main`, but now you've seen each intermediate.

### Step 2: A multi-file program

In `code/` you have:

```
greet.h         <-- declaration of greet()
greet.c         <-- definition  of greet()
main.c          <-- uses greet()
Makefile        <-- builds it all
```

Build the executable from sources:

```sh
gcc -c greet.c -o greet.o        # one translation unit
gcc -c main.c  -o main.o         # another
gcc greet.o main.o -o greet      # link both into an executable
./greet
```

Now archive `greet.o` into a library and link against it:

```sh
ar rcs libgreet.a greet.o        # build a static library
gcc main.o -L. -lgreet -o greet  # link main.o against libgreet.a
./greet
```

`-L.` adds `.` to the library search path; `-lgreet` says "find `libgreet.{a,so,dylib}`."

### Step 3: Read the provided `Makefile`

The included Makefile demonstrates pattern rules, automatic variables, and `.PHONY` targets. Read it line by line — the comments explain the standard idioms.

```sh
make             # build everything
make clean       # remove artifacts
make             # rebuilds only what changed
touch greet.c    # mark greet.c as modified
make             # rebuild only greet.o and re-link; main.o untouched
```

### Step 4: Trigger and diagnose each class of error

```sh
# Preprocessor error — wrong header name
gcc -c -DBROKEN_INCLUDE main.c 2>&1 | head -3
# fatal error: ... No such file or directory

# Compile error — undeclared identifier
gcc -c -DBROKEN_CALL main.c 2>&1 | head -3
# error: implicit declaration of function ... / undefined

# Linker error — forgot to link greet.o
gcc main.o -o broken 2>&1 | head -3
# undefined reference to 'greet'
```

Note that each tool stamps its own format. Once you can spot the stamp ("fatal error" vs "error" vs "undefined reference"), you know which knob to turn.

### Step 5: Shared library version

```sh
gcc -fPIC -c greet.c -o greet.pic.o
gcc -shared greet.pic.o -o libgreet.so       # Linux
# macOS: gcc -dynamiclib greet.pic.o -o libgreet.dylib
gcc main.o -L. -lgreet -o greet_shared

# Run — needs to find libgreet.so at load time
LD_LIBRARY_PATH=. ./greet_shared              # Linux
# DYLD_LIBRARY_PATH=. ./greet_shared           # macOS
```

The shared library lives outside the executable. Replace it, restart the process, and the new code is picked up — that's how OS security patches work.

## Use It

Real C/C++ projects use exactly these tools, just hidden by a layer of build-system code-gen:

- **CMake** generates a Makefile (or Ninja file) that, line by line, looks like the one in `code/Makefile`.
- **Bazel** computes the dependency DAG the same way `make` does, but in a sandboxed, distributed environment.
- **`pkg-config --cflags --libs gtk+-3.0`** prints the `-I…` and `-l…` flags you'd otherwise type by hand.

Look at the build output of a sizable project (`cmake --build . -- -v` is verbose). Every line is `cc -c` on some translation unit, followed by `cc -o` linking the lot. No magic.

## Read the Source

- [GNU Make manual](https://www.gnu.org/software/make/manual/make.html) — chapters 4 (rules) and 10 (functions) are the working subset most people use.
- [LLVM's `clang/Driver`](https://github.com/llvm/llvm-project/tree/main/clang/lib/Driver) — clang is `cpp + cc1 + as + ld` in a trenchcoat, and the driver source code is the cleanest place to see exactly how it orchestrates them.
- [Linkers and Loaders](https://linker.iecc.com/) by John Levine — the book on linking. Free online.

## Ship It

This lesson ships **`outputs/Makefile.template`** — a generic, well-commented Makefile you can drop into any new C project to get pattern rules, dependency tracking, debug/release flags, and a `make test` target.

## Exercises

1. **Easy.** Take the included `main.c` and `greet.c`. Without using `make`, produce the executable by typing every step yourself. Verify each intermediate (`main.i`, `main.s`, `main.o`) exists between commands.
2. **Medium.** Modify the Makefile to support a `make debug` target that builds with `-O0 -g` and a `make release` target that builds with `-O3 -DNDEBUG`, into separate output directories (`build/debug/`, `build/release/`).
3. **Hard.** Convert the static library workflow to use auto-generated dependency files (`gcc -MMD -MP`) so that editing a header triggers exactly the right `.o` rebuilds. Show that touching `greet.h` causes only files that include it to rebuild.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Translation unit | "A `.c` file" | A `.c` file after preprocessing — one input to one invocation of the compiler |
| Symbol | "A name" | A named entry in an `.o` file's symbol table: a function or global, marked as defined or undefined |
| Static library | "A library" | An `ar` archive of `.o` files; the linker pulls in just the ones whose symbols are used |
| Shared library | "A `.so` / `.dylib`" | A loadable binary mapped into a process at load time; one copy serves many processes |

## Further Reading

- *Computer Systems: A Programmer's Perspective* (Bryant & O'Hallaron) — Chapter 7 on linking is the textbook treatment.
- [Eli Bendersky's "Position Independent Code"](https://eli.thegreenplace.net/2011/11/03/position-independent-code-pic-in-shared-libraries) — why `-fPIC` exists and what it changes.
- [GNU `binutils` docs](https://sourceware.org/binutils/docs/) — `ar`, `nm`, `objdump`, `readelf`, `strip`, all the tools you reach for when investigating an `.o` file.
