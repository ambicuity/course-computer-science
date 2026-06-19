# Makefile for RISC-V assembly programs
# Requires: riscv64-unknown-elf-gcc (or riscv64-linux-gnu-gcc)
# Emulator: Spike (riscv-isa-sim) or QEMU
#
# Usage:
#   make all        — build every program
#   make test       — build and run all programs
#   make hello      — build hello.s
#   make clean      — remove build artifacts
#
# Adjust CROSS_COMPILE if your toolchain prefix differs.

CROSS_COMPILE ?= riscv64-unknown-elf-
CC     = $(CROSS_COMPILE)gcc
OBJDUMP = $(CROSS_COMPILE)objdump
CFLAGS = -march=rv32i -mabi=ilp32 -nostartfiles -T linker.ld

# Programs to build (without .s extension)
PROGRAMS = hello gcd bubblesort matrix_multiply palindrome

# ── Targets ──────────────────────────────────────────────────────

.PHONY: all test clean

all: $(PROGRAMS)

# Pattern rule: build .s → executable
%: %.s
	$(CC) $(CFLAGS) -o $@ $<

# Generate disassembly for study
%.dis: %
	$(OBJDUMP) -d $< > $@

# Run each program (Spike assumes pk is installed)
test: all
	@echo "─── Running programs ───"
	@for prog in $(PROGRAMS); do \
		echo "[RUN] $$prog"; \
		spike pk ./$$prog && echo "[PASS] $$prog" || echo "[FAIL] $$prog"; \
	done

clean:
	rm -f $(PROGRAMS) *.dis *.o
