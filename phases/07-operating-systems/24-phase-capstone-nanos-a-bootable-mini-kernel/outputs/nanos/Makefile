# Makefile for nanos — a bootable RISC-V mini-kernel
#
# Requires: riscv64-unknown-elf-gcc, qemu-system-riscv64

CC      = riscv64-unknown-elf-gcc
OBJCOPY = riscv64-unknown-elf-objcopy

CFLAGS  = -march=rv64im -mabi=lp64 -mcmodel=medany \
          -ffreestanding -O2 -Wall -Wextra -Werror
LDFLAGS = -T linker.ld -nostdlib

OBJS    = boot.o kernel.o context.o

.PHONY: all run clean

all: nanos.elf

nanos.elf: $(OBJS)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^

boot.o: boot.s
	$(CC) $(CFLAGS) -c -o $@ $<

kernel.o: kernel.c
	$(CC) $(CFLAGS) -c -o $@ $<

context.o: context.s
	$(CC) $(CFLAGS) -c -o $@ $<

run: nanos.elf
	qemu-system-riscv64 -machine virt -nographic -bios none -kernel nanos.elf

clean:
	rm -f $(OBJS) nanos.elf
