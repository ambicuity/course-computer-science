# Memory-Safety Attacks — Stack Smash, ROP, ASLR

> One unchecked `gets()` call gives an attacker control of your program. Understanding why is the difference between writing code and writing secure code.

**Type:** Build
**Languages:** C, RISC-V Assembly
**Prerequisites:** Phase 12 lessons 01–19, Phase 02 (C pointers, stack), Phase 06 (RISC-V assembly, calling convention), Phase 07 (virtual memory, ASLR)
**Time:** ~90 minutes

## Learning Objectives

- Explain why C and C++ programs are susceptible to memory-safety vulnerabilities due to direct pointer access and manual memory management.
- Describe the stack layout during a function call: local variables, saved frame pointer, return address, and how a buffer overflow corrupts these.
- Implement a classic stack-smash attack: overflow a local buffer to overwrite a return address and redirect execution to a target function.
- Construct a Return-Oriented Programming (ROP) chain that bypasses a non-executable (NX) stack by reusing existing code gadgets.
- Explain what ASLR randomizes, why it raises the bar for exploitation, and the conditions under which it can be bypassed.
- Compare the four major mitigation layers (stack canaries, NX/DEP, ASLR/PIE, RELRO) and describe what each protects against and where it falls short.
- Read and modify RISC-V assembly that demonstrates return-address overwrite on a non-x86 architecture.

## The Problem

C and C++ give programmers direct memory access through pointers. There is no bounds checking on arrays, no automatic validation of string operations, and no protection against writing past the end of a stack-allocated buffer. This design — inherited from the systems-programming philosophy of "trust the programmer" — is the root cause of an entire class of vulnerabilities called **memory-safety bugs**.

A single stack buffer overflow can overwrite a function's **return address** — the pointer on the stack that tells `ret` where to jump when the function finishes. By controlling that address, an attacker redirects the CPU to execute arbitrary code.

The scale of the problem is staggering:
- ~70% of browser vulnerabilities are memory-safety bugs (Microsoft, 2019–2023).
- The majority of remote code execution (RCE) exploits start with a buffer overflow.
- Google's Project Zero estimates that memory-safety bugs account for the majority of "critical" CVEs across Chrome, Windows, iOS, and Android.
- EternalBlue (MS17-010), the exploit behind WannaCry and NotPetya, used a stack buffer overflow in Windows SMB.

The lesson that follows builds the intuition — from the raw stack layout to the modern mitigations that make exploitation exponentially harder — through three hands-on demos in C and RISC-V assembly.

## The Concept

Memory-safety exploitation rests on four pillars: how the stack works, how to hijack it, how to do so even when the stack is non-executable, and how modern defenses try (and sometimes fail) to stop all of the above.

### 1. Stack Buffer Overflows (Stack Smashing)

When a function is called in C on x86-64, the stack frame looks like this (low address at bottom):

```
Higher addresses
+---------------------------+
|     caller's stack frame  |
+---------------------------+
|     return address        |  ← rbp + 8    (where `ret` jumps to)
+---------------------------+
|     saved RBP             |  ← rbp        (restored on `leave`)
+---------------------------+
|     local variables       |  ← rbp - N
|     (e.g., char buf[64])  |
+---------------------------+
|     ...                   |  ← rsp
Lower addresses
```

The key insight: **the return address lives ABOVE the local variables in memory**. If a function writes past the end of a local buffer (through `gets()`, `strcpy()` without length checks, `scanf("%s", ...)`, etc.), it marches sequentially toward higher addresses — through the saved RBP and straight into the return address.

A classic attack overflows the buffer with:
1. **Padding** — enough data to fill the buffer and reach the return address.
2. **The new return address** — pointing to attacker-controlled code (shellcode) placed elsewhere in the payload.

This was documented definitively in Aleph One's "Smashing the Stack for Fun and Profit" (Phrack, 1996). The paper demonstrated that a carefully crafted input to a vulnerable program could spawn a shell — and the fundamental technique has not changed in nearly three decades.

### 2. Return-Oriented Programming (ROP)

After Aleph One's paper, the industry response was to make the stack **non-executable** — the NX bit (x86) or DEP (Windows). If the CPU cannot fetch and execute instructions from the stack, placing shellcode on the stack and jumping to it accomplishes nothing.

Enter Return-Oriented Programming, formalized by Shacham et al. in 2007. The observation: even though you cannot inject new code, the process's own executable memory (`.text` section, loaded libraries) contains thousands of instruction sequences ending in `ret`. Each such sequence is called a **gadget**.

Examples of gadgets:
```
pop rdi; ret          # pops a value into rdi, then returns
pop rsi; pop r15; ret # pops two values, then returns  
syscall; ret          # invokes a syscall, then returns
```

By overwriting the return address not with a single target, but with a **chain of return addresses** pointing to consecutive gadgets, the attacker can orchestrate arbitrary computation using only existing code. Each gadget does a small piece of work (load a register, write a value, call a function), then `ret` pops the next gadget address off the attacker-controlled stack.

The classic pattern to call `win(arg)`:
```
Stack (growing direction →)
+----------------------------+
| pop rdi; ret gadget        |  ← overwritten return address
+----------------------------+
| arg for rdi                |  ← value popped by `pop rdi`
+----------------------------+
| address of win()           |  ← what `ret` jumps to next
+----------------------------+
```

Finding gadgets is the main engineering challenge. Tools like `ROPgadget` (`pip install ROPgadget`) and `ropper` scan a binary for all instruction sequences ending in `ret`, `call`, or `jmp` (indirect branches are also useful). On a typical binary, thousands of gadgets exist.

### 3. ASLR (Address Space Layout Randomization)

If the attacker knows the exact address of every gadget and function, ROP is straightforward. **ASLR** randomizes the base addresses of the stack, heap, loaded libraries, and (with PIE) the executable itself. An attacker who cannot predict addresses cannot construct a working ROP chain.

The randomization works per-execution:
- **Stack:** randomized base (on x86-64, ~22 bits of entropy from a 47-bit user address space).
- **Heap:** randomized base.
- **Libraries (mmap):** randomized base (on x86-64, ~28 bits of entropy for libraries).
- **Executable:** randomized only if compiled as **PIE** (Position Independent Executable). Without PIE (a non-PIE binary is loaded at a fixed address like `0x400000`), the executable's `.text` section is at a predictable location.

ASLR bypass methods:
- **Information leak:** read a pointer value from the target to compute the base address of a library or the executable (e.g., via a format-string vulnerability, a use-after-free that leaks a vtable pointer, or a side channel).
- **Non-PIE executable:** if the binary is not compiled with `-pie`/`-fpie`, its code is at a fixed address — gadgets from the binary itself are always usable regardless of ASLR.
- **Return-to-PLT:** call functions through the PLT (Procedure Linkage Table), whose address is fixed relative to the binary. Even with ASLR, the PLT stubs are at known offsets in a non-PIE binary.
- **Heap spray:** fill the heap with many copies of shellcode or NOP sleds to increase the probability that a random jump lands on code. Less relevant with NX now standard.
- **Blind return address prediction:** on 32-bit systems, the stack entropy may be as low as 8 bits (256 possibilities). Brute-force is feasible.

ASLR on 32-bit systems is significantly weaker than on 64-bit. With only ~8 bits of randomization for the stack base, an attacker can try 256 offsets before crashing the program (and with `fork`-based servers, each child has the same stack layout, making repeated attempts trivial).

### 4. Mitigation Evolution

Each defense addresses a specific attack vector, but none is a silver bullet:

| Mitigation | What it does | Bypass |
|------------|-------------|--------|
| **Stack canary** (`-fstack-protector`) | A random value is placed before the return address on the stack. Before `ret`, the canary is checked — if modified, the program aborts. | Information leak of the canary value; overwrite the canary with its correct value (requires an adjacent read); or overwrite a different target (e.g., a function pointer before the canary). |
| **NX/DEP** (`-z noexecstack`) | Marks the stack as non-executable. Shellcode on the stack cannot run. | ROP — reuse existing executable code. |
| **ASLR + PIE** (`-pie -fpie`) | Randomizes the base address of all memory regions, including the executable. | Information leak; non-PIE code; 32-bit brute-force; return-to-PLT in non-PIE binaries. |
| **Full RELRO** (`-z now`) | Resolves all PLT entries at load time, then marks the GOT read-only. Prevents GOT overwrite attacks. | Requires targeting other writable function pointers (e.g., `__free_hook`, `__malloc_hook` on older glibc). |
| **CFI (Control Flow Integrity)** | Restricts indirect branch targets to a precomputed set of valid destinations. | Coarse-grained CFI (forward-edge only) can still be bypassed; fine-grained CFI (Clang CFI) is more robust but has performance cost. |
| **Shadow Stack (Intel CET)** | Hardware maintains a separate, protected copy of return addresses. `ret` compares the stack address to the shadow stack address; mismatch = fault. | Not yet widespread. Requires both CPU support and recompilation. |
| **PAC (ARM v8.3)** | Pointer Authentication Code: a cryptographic MAC is embedded in unused bits of pointer values. Tampering changes the MAC and causes a fault. | Requires information leak of the PAC key; brute-force of 7-bit PAC (can be viable locally). |
| **MTE (ARM v9)** | Memory Tagging Extension: assigns a 4-bit tag to each 16-byte memory region; pointer tags must match. Catches spatial and temporal memory safety errors at use time. | Tag collision probability is 1/16 per access; attacker may retry. Primarily a probabilistic defense. |

Modern hardening for a C binary looks like:
```bash
gcc -fstack-protector-strong -pie -fpie -Wl,-z,relro,-z,now -o program program.c
```

This enables canaries, PIE+ASLR, and full RELRO. Combined with kernel-level ASLR, it makes reliable exploitation significantly harder — but not impossible if an information leak exists.

## Build It

You will write and run three demonstrations. Each builds on the previous one and shows a different facet of memory-safety exploitation.

### Step 1: Classic Stack Smash (C)

Write a program with a vulnerable function that uses `gets()` to read into a 64-byte buffer. A harmless `win()` function displays a success message. The goal: provide input that overflows the buffer and overwrites the return address to point to `win()`.

Key compilation flags:
```bash
gcc -fno-stack-protector -no-pie -z execstack -o exploit main.c
```

- `-fno-stack-protector`: disable canaries (no stack integrity check).
- `-no-pie`: do not position the executable independently (fixed load address).
- `-z execstack`: mark the stack as executable (not needed for ROP, but needed if shellcode were placed on the stack).

Inside the program:
1. Print the address of the buffer, the saved frame pointer, and where the return address sits.
2. Compute the offset from the buffer start to the return address.
3. Construct a payload: `[72 bytes of 'A'] + [address of win()]`.
4. Feed it via stdin to `gets()` — observe control flow hijacking.

### Step 2: ROP Chain (C)

Same setup, but compile **without** `-z execstack` (NX is enabled):

```bash
gcc -fno-stack-protector -no-pie -o exploit main.c
```

Now shellcode on the stack causes a segmentation fault (the CPU refuses to fetch instructions from a non-executable page). Instead, we build a ROP chain:

1. Find a `pop rdi; ret` gadget in the binary:
   ```bash
   objdump -d exploit | grep -A1 "pop.*rdi"
   # Or:
   ROPgadget --binary exploit | grep ": pop rdi ; ret$"
   ```
2. Find the address of the `win()` function:
   ```bash
   nm exploit | grep win
   ```
3. Construct the ROP chain:
   ```
   [padding 72 bytes]
   [address of pop_rdi_ret gadget]
   [address of argument string for rdi]
   [address of win()]
   ```
4. Feed the payload — the `ret` from the vulnerable function pops the gadget address, `pop rdi; ret` loads the argument and returns to `win()`.

If `win()` takes no arguments, you can skip the `pop rdi` and just return directly to `win()`. But the full ROP chain demonstrates the pattern you would use to call any function with any arguments.

### Step 3: RISC-V Stack Overflow (Assembly)

The same concept on a different architecture. In RISC-V, the return address is stored in the `ra` register. The caller saves `ra` to the stack before calling a function; the callee restores it and executes `ret` (which is a pseudo-instruction for `jalr zero, ra, 0`).

If a function saves `ra` at `sp+72` and allocates a 64-byte buffer at `sp+0`, a buffer overflow overwrites the saved `ra`. When the function executes `ld ra, 72(sp); addi sp, sp, 80; ret`, it jumps to the attacker-controlled address.

The RISC-V demonstration:
```assembly
vulnerable:
    addi sp, sp, -80
    sd   ra, 72(sp)      # save return address
    sd   s0, 64(sp)      # save frame pointer
    addi s0, sp, 80      # set frame pointer
    addi a0, sp, 0       # buffer = sp+0
    jal  gets            # read into buffer (OVERFLOW!)
    ld   s0, 64(sp)
    ld   ra, 72(sp)      # restore ra from stack (OVERWRITTEN!)
    addi sp, sp, 80
    ret                  # jump to attacker address
```

Compile with:
```bash
riscv64-linux-gnu-gcc -static -o exploit main.s
```

Or, if you have QEMU user-mode emulation:
```bash
riscv64-linux-gnu-gcc -static -o exploit main.s
qemu-riscv64 ./exploit
```

Without a RISC-V toolchain, the source is still educational — the key takeaway is that the attack pattern is architecture-independent.

## Use It

These techniques are not academic. They are used in real-world exploit chains daily:

- **EternalBlue (MS17-010):** A stack buffer overflow in Windows SMBv1. The exploit uses a ROP chain to disable SMEP (Supervisor Mode Execution Prevention) and execute shellcode in kernel mode. Weaponized by WannaCry and NotPetya ransomware — estimated damages in the billions.
- **Stagefright (Android CVE-2015-1538):** A heap buffer overflow in Android's media library (libstagefright) triggered by a crafted MP4 file. The exploit used ROP chains to bypass NX and ASLR on Android devices. Demonstrated that memory-safety bugs in media parsers are a rich attack surface.
- **Heartbleed (CVE-2014-0160):** A buffer over-read (not overflow) in OpenSSL's TLS heartbeat extension. The attacker could read 64 KB of server memory beyond the heartbeat payload, leaking private keys, session cookies, and passwords. While not a control-flow hijack, it is a memory-safety bug of the same class.
- **Pwn2Own competitions:** Every year, contestants demonstrate stack and heap overflows in major browsers, kernels, and hypervisors. The winning exploits typically chain an information leak (to defeat ASLR) with a ROP chain (to defeat NX) and a final payload (to escalate privileges or escape a sandbox).
- **iPhone jailbreaks:** The most recent jailbreaks (e.g., checkm8, the bootrom exploit for A5–A11 chips) use buffer overflows in USB drivers — a class of attack that has existed since the 1990s.

Modern mitigations have raised the bar enormously, but they have not eliminated the class:
- **PAC (ARM v8.3)** adds cryptographic protection to pointers, making arbitrary code execution significantly harder on Apple Silicon and recent Android devices.
- **Intel CET (Control-flow Enforcement Technology)** adds hardware shadow stacks and indirect-branch tracking, making ROP impractical on supported hardware.
- **MTE (ARM v9)** tags every memory allocation and checks tags on each access, catching most spatial and temporal memory errors at runtime.

Despite this, C and C++ remain the languages for operating systems, browsers, and embedded systems — and memory-safety bugs continue to be discovered and exploited. The lesson is not that C is "dangerous" and should never be used. The lesson is that understanding exactly how these attacks work is essential for anyone who writes systems code.

## Read the Source

- **"Smashing the Stack for Fun and Profit" — Phrack 49 (Aleph One, 1996):** The canonical paper that introduced stack-smashing attacks to the public. Every concept in this lesson traces back to this paper. Read the original: `http://phrack.org/issues/49/14.html`.
- **"The Geometry of Innocent Flesh on the Bone: Return-into-libc without Function Calls" — Shacham et al. (CCS 2007):** The paper that formally defined Return-Oriented Programming. Explains how to find gadgets and compose them into Turing-complete computation.
- **Linux kernel `arch/x86/entry/entry_64.S`:** The syscall entry and exit path. Look at how `entry_SYSCALL_64` handles the swapgs, stack switch, and return via `sysret`. Understanding this code shows how the kernel uses the same stack mechanisms that attackers exploit — and how it protects itself.
- **GCC `gcc/gcc/stack-protector.*`:** The source code implementing `-fstack-protector`. The canary value is read from `__stack_chk_guard` (set by the dynamic linker from `/dev/urandom` or `AT_RANDOM` aux vector). The check is inserted in the function epilogue.
- **PaX/grsecurity documentation:** The PaX team pioneered ASLR and NX on Linux (as `CONFIG_PAX_ASLR` and `CONFIG_PAX_NOEXEC`). Their technical documentation explains the design decisions and bypass analysis in far more depth than any textbook.
- **Intel CET specification (Chapter 18 of Intel SDM):** Describes the hardware implementation of shadow stacks (`ssp` register, `call`/`ret` checking, `setssbsy`/`clrssbsy` instructions) and indirect-branch tracking (`ENDBRANCH` instruction markers).

## Ship It

The reusable artifact is a **Memory-Safety Exploit Demonstration Suite** — three programs (C stack smash, C ROP chain, RISC-V return-address overwrite) that live in `code/`. The Makefile builds all three configurations. The suite can be reused in the phase capstone (Lesson 24: mini-CTF toolkit) as reference material for CTF exploit challenges.

## Exercises

1. **Easy** — Compile the C stack-smash demo and compute the correct offset from the buffer to the return address (hint: objdump or GDB can help). Successfully hijack control to the `win()` function. Verify by checking that "WIN!" is printed.

2. **Medium** — Extend the ROP chain to not just call `win()`, but to call `win()` with a specific string argument. You will need to find a `pop rdi; ret` gadget (or equivalent) and place the argument address on the stack. Hint: store the string in a global variable whose address you can predict.

3. **Hard** — On the RISC-V assembly demo, calculate the exact offset from the buffer to the saved `ra`. Modify the input to overwrite `ra` so that the `vulnerable` function returns to `win()` instead of `main`. For a bonus challenge: use QEMU user-mode emulation to verify the exploit works on RISC-V without hardware.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Buffer overflow | Writing past the end of an array | Writing data beyond the allocated bounds of a buffer, corrupting adjacent memory (return address, saved frame pointer, adjacent variables). |
| Stack canary | A protection against stack smashing | A random value placed between local variables and the return address. Checked before `ret`; a mismatch means the buffer was overflowed. |
| NX / DEP (W^X) | Non-executable stack | A page-table permission bit that prevents the CPU from fetching instructions from stack (or heap) pages. Shellcode on the stack cannot execute. |
| ASLR | Address Space Layout Randomization | Randomizing the base address of stack, heap, libraries, and (with PIE) the executable at each run, making it hard to predict addresses for exploitation. |
| PIE | Position Independent Executable | An executable compiled with `-pie -fpie` so that its code can be loaded at any base address, enabling ASLR for the executable itself. |
| ROP | Return-Oriented Programming | An exploit technique that chains short instruction sequences (gadgets) ending in `ret` to execute arbitrary computation using only existing code. |
| Gadget | A short instruction sequence ending in ret | A few instructions found in the binary that perform a small operation (e.g., `pop rdi; ret`) and can be chained to build arbitrary behavior. |
| Return address | The address a function returns to | Saved on the stack during a `call` instruction. Overwriting it is the primary goal of stack-smashing attacks. |
| Shellcode | Machine code injected by an attacker | Executable payload (often spawning a shell) placed in the overflow buffer or elsewhere in memory. |
| CFI | Control Flow Integrity | A set of techniques that restrict indirect branch targets to a precomputed set of valid destinations, making ROP and JOP (Jump-Oriented Programming) harder. |
| Shadow stack | A hardware-protected copy of return addresses | A separate memory region (inaccessible to normal loads/stores) where `call` pushes and `ret` pops the return address. Mismatch = fault. |
| RELRO | Relocation Read-Only | A linker feature that makes the GOT (Global Offset Table) read-only after dynamic linking, preventing GOT-overwrite attacks. "Full RELRO" (`-z now`) resolves all symbols at load time. |
| GOT / PLT | Global Offset Table / Procedure Linkage Table | Indirection tables used for dynamic linking. The GOT holds function pointers; the PLT contains stub code that resolves symbols lazily. Overwriting a GOT entry is a common exploit target. |
| Heap spray | Filling the heap with attacker data | A technique to increase the probability that a corrupted pointer lands on attacker-controlled data (e.g., NOP sled + shellcode), used to bypass ASLR on the heap. |

## Further Reading

- "Smashing the Stack for Fun and Profit" — Aleph One, Phrack 49 (1996). The original paper. Still the best introduction to stack buffer overflows.
- "Return-Oriented Programming: Systems, Languages, and Applications" — Shacham et al. (2010). A comprehensive survey of ROP theory and practice.
- "Systematic Analysis of Defenses Against Return-Oriented Programming" — Carlini et al. (2014). An evaluation of CFI, shadow stacks, and other defenses against ROP.
- "ASLR Smack & Laugh Reference" — The PaX Team (2003). A detailed analysis of ASLR strengths and weaknesses on various architectures.
- Intel 64 and IA-32 Architectures Software Developer's Manual, Volume 3A, Chapter 18 — "Control-flow Enforcement Technology (CET)". The hardware specification for shadow stacks and indirect-branch tracking.
- "Anatomy of an Exploit: Inside the CVE-2015-1538 Stagefright Vulnerability" — Google Project Zero (2015). A deep dive into a real-world memory-safety exploit chain targeting Android's media library.
