# Memory-Safety Exploit Demonstration Suite

**Author:** Phase 12 — Cryptography & Security, Lesson 20

## What It Is

A suite of C and RISC-V assembly programs demonstrating stack buffer overflow exploitation and modern mitigations. Three build configurations show the evolution of memory-safety attacks:

- **`stack-smash`** — Classic stack buffer overflow with an executable stack (`-z execstack`). Overwrite the return address to call a target function.
- **`rop-demo`** — Same overflow, but with NX enabled and PIE disabled. Demonstrates the need for Return-Oriented Programming.
- **`modern`** — All protections enabled (stack canary + PIE + ASLR + NX + Full RELRO). Shows how defense-in-depth catches or prevents exploitation.
- **`riscv`** — RISC-V assembly program demonstrating buffer overflow overwriting the `ra` (return address) register on a RISC-V architecture.

## How to Build

```bash
cd code
make all          # Builds all three C targets
make riscv        # Build RISC-V target (requires riscv64-linux-gnu-gcc)
```

## How to Run

### Stack Smash Demo

```bash
# Show stack layout and generate payload
./stack-smash
# or: ./stack-smash smash-payload

# Pipe the payload into the vulnerable function
./stack-smash run-smash < /tmp/smash_payload.bin
```

### ROP Demo

```bash
# Find gadgets in the binary
make gadgets

# Generate ROP chain with your gadget addresses
./rop-demo rop-payload 0x401023 0x402010

# Test the exploit
./rop-demo run-rop < /tmp/rop_payload.bin
```

### Modern (Protected)

```bash
./modern
./modern run-smash < <(python3 -c 'import sys; sys.stdout.buffer.write(b"A"*88 + b"BBBBBBBB")')
# Expected: __stack_chk_fail
```

### RISC-V Demo

```bash
qemu-riscv64 ./riscv
# Or on RISC-V hardware: ./riscv
```

## What Each Demo Demonstrates

| Target | Canary | NX/DEP | ASLR | PIE | Demonstration |
|--------|--------|--------|------|-----|---------------|
| stack-smash | DISABLED | DISABLED | DISABLED | DISABLED | Classic return-address overwrite, shellcode injection |
| rop-demo | DISABLED | ENABLED | DISABLED | DISABLED | ROP chaining to bypass NX |
| modern | ENABLED | ENABLED | ENABLED | ENABLED | Canary catch, PIE+ASLR randomization |
| riscv | N/A | N/A | N/A | N/A | RISC-V return-address overwrite |

## Connection to the Capstone

The capstone (Phase 12, Lesson 24 — "A TLS 1.3 Library & A Mini-CTF") includes a CTF challenge where students exploit toy TLS implementations with memory-safety bugs. The techniques demonstrated here — calculating offsets, constructing ROP chains, bypassing NX — are directly applicable. The `outputs/` directory serves as a reference for the exploit-writing component of the CTF toolkit.

## Limitations

- The C demos target x86-64 Linux. They use x86-64-specific inline assembly (`mov %%rbp`), `gets()`, and `/proc/self/maps`. They will not compile or run on non-x86 architectures (except via emulation).
- The modern target will abort with `__stack_chk_fail` on overflow — that is by design. The demonstration is in observing *how* it fails.
- ASLR bypass requires a separate information-leak vulnerability, which is not included here.
- The RISC-V target requires a cross-compiler or native RISC-V toolchain.
- The programs disable all protections intentionally. Running them with uncontrolled input on a production system is dangerous.
