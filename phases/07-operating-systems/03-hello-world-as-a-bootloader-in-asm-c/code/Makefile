# Makefile — Build a minimal RISC-V kernel
#
# Targets:
#   make          Build kernel.bin
#   make run      Build and run in QEMU
#   make debug    Build and run with GDB server
#   make clean    Remove build artifacts

# Cross-compilation toolchain
# Adjust prefix if your toolchain differs
CC      = riscv64-unknown-elf-gcc
AS      = riscv64-unknown-elf-as
LD      = riscv64-unknown-elf-ld
OBJCOPY = riscv64-unknown-elf-objcopy
QEMU    = qemu-system-riscv64

# Flags
CFLAGS  = -march=rv64imac -mabi=lp64 -O2 -Wall -Wextra -nostdlib -ffreestanding
ASFLAGS = -march=rv64imac -mabi=lp64
LDFLAGS = -T linker.ld --no-relax

# QEMU settings
QEMU_FLAGS  = -nographic -machine virt -bios none -kernel kernel.bin
QEMU_DEBUG  = $(QEMU_FLAGS) -s -S

# Build targets
all: kernel.bin

# Assemble bootloader
boot.o: boot.s
	$(AS) $(ASFLAGS) -o $@ $<

# Compile kernel
kernel.o: kernel.c
	$(CC) $(CFLAGS) -c -o $@ $<

# Link into ELF
kernel.elf: boot.o kernel.o linker.ld
	$(LD) $(LDFLAGS) -o $@ boot.o kernel.o

# Convert ELF to flat binary (strip ELF headers, keep raw code/data)
kernel.bin: kernel.elf
	$(OBJCOPY) -O binary $< $@

# Run in QEMU (press Ctrl-A then X to exit)
run: kernel.bin
	$(QEMU) $(QEMU_FLAGS)

# Run with GDB server (connect with: target remote :1234)
debug: kernel.bin
	$(QEMU) $(QEMU_DEBUG)

clean:
	rm -f *.o *.elf *.bin

.PHONY: all run debug clean
