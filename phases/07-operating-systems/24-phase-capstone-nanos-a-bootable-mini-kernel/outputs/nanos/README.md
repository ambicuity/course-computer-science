# nanos — A Bootable Mini-Kernel

A minimal RISC-V kernel demonstrating core OS concepts: scheduling, context switching, memory allocation, and a shell.

## Requirements

- `riscv64-unknown-elf-gcc` (RISC-V cross-compiler toolchain)
- `qemu-system-riscv64` (RISC-V system emulator)

### Installing on macOS

```bash
brew tap riscv-software-src/riscv
brew install riscv-gnu-toolchain qemu
```

### Installing on Ubuntu/Debian

```bash
sudo apt install gcc-riscv64-unknown-elf qemu-system-riscv64
```

## Build

```bash
make
```

This produces `nanos.elf`.

## Run

```bash
make run
```

QEMU boots nanos and presents a shell prompt:

```
nanos: booting...
nanos: 3 background processes created

========================================
  nanos - a bootable mini-kernel
  Phase 07 Operating Systems Capstone
========================================

nanos>
```

## Shell Commands

| Command | Description |
|---------|-------------|
| `echo <text>` | Print text |
| `help` | List available commands |
| `ps` | Show process table |
| `meminfo` | Show memory usage |
| `counters` | Show background process counters |
| `halt` | Shut down QEMU |

## Architecture

```
boot.s       — Entry point, stack/BSS setup, calls kernel_main()
context.s    — Context switch (save/restore callee-saved registers)
kernel.c     — UART driver, bump allocator, scheduler, process table, shell
linker.ld    — Memory layout
Makefile     — Build rules
```

## Clean

```bash
make clean
```
