# ABI Cheat Sheet — Calling Conventions

Side-by-side reference for three calling conventions you'll meet in this course.

## SysV AMD64 (Linux, macOS, BSD on x86_64)

| | Register |
|--|----------|
| Args 1-6 (integer / pointer) | `rdi, rsi, rdx, rcx, r8, r9` |
| Args 7+ | stack (right-to-left, before `call`) |
| Args (floating-point) 1-8 | `xmm0..xmm7` |
| Return value (int) | `rax` (+ `rdx` for 128-bit) |
| Return value (float) | `xmm0` |
| Callee-saved | `rbx, rbp, rsp, r12, r13, r14, r15` |
| Caller-saved | `rax, rcx, rdx, rsi, rdi, r8, r9, r10, r11`, all xmm |
| Stack alignment | 16-byte at `call` time |
| Red zone | 128 bytes below `rsp` callee may use without adjustment |

Prologue: `push rbp; mov rbp, rsp; sub rsp, N`
Epilogue: `leave; ret`

## Windows x64 (Microsoft x64 ABI)

| | Register |
|--|----------|
| Args 1-4 (integer / pointer) | `rcx, rdx, r8, r9` |
| Args 5+ | stack, with 32-byte "shadow space" reserved by caller |
| Args (floating-point) 1-4 | `xmm0..xmm3` (one slot used per argument position, shared with integer regs) |
| Return value (int) | `rax` |
| Callee-saved | `rbx, rbp, rdi, rsi, rsp, r12-r15`, `xmm6-xmm15` |
| Caller-saved | `rax, rcx, rdx, r8-r11`, `xmm0-xmm5` |
| Stack alignment | 16-byte at `call` time |
| Shadow space | 32 bytes always allocated by caller (even unused) |

## RISC-V (RV64I)

| | Register | Alias |
|--|----------|-------|
| Return address | `x1` | `ra` |
| Stack pointer | `x2` | `sp` |
| Global pointer | `x3` | `gp` |
| Thread pointer | `x4` | `tp` |
| Temps (caller-saved) | `x5-x7, x28-x31` | `t0-t6` |
| Saved (callee-saved) | `x8-x9, x18-x27` | `s0-s11` |
| Args / return | `x10-x17` | `a0-a7` (a0/a1 are return values) |

Prologue (typical): `addi sp, sp, -16; sd ra, 8(sp); sd s0, 0(sp)`
Epilogue: `ld ra, 8(sp); ld s0, 0(sp); addi sp, sp, 16; ret` (= `jalr x0, ra, 0`)

## ARM AArch64 (AAPCS64)

| | Register |
|--|----------|
| Args 1-8 (integer / pointer) | `x0..x7` |
| Args 1-8 (floating-point) | `v0..v7` |
| Return value (int) | `x0` (+ `x1` for 128-bit) |
| Frame pointer | `x29` (fp) |
| Link register | `x30` (lr) — equivalent of x86's saved return address |
| Stack pointer | `sp` |
| Callee-saved | `x19-x28`, `x29`, `x30`, certain SIMD regs |
| Caller-saved | `x0-x18`, low SIMD regs |
| Stack alignment | 16-byte; quad-word aligned for SP at function entry |

## Why the diversity?

Each ABI optimizes different things:
- **SysV AMD64**: more registers for args (6) → fewer stack accesses.
- **Windows x64**: shadow space simplifies debuggers and EH unwinding.
- **RISC-V**: 8 arg registers; simple uniform encoding.
- **AArch64**: 8 arg registers + dedicated frame/link regs; lock-step with iOS/Android.

Don't mix them. Code compiled for one ABI on the same hardware can crash when linked with code from another.
