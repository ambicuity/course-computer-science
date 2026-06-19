CC = gcc
CFLAGS  = -Wall -Wextra -O0 -g
LDFLAGS =

.PHONY: all clean run-smash run-rop run-modern

all: stack-smash rop-demo modern

# ============================================================
# Stack Smash demo: NX disabled, stack executable
# Shellcode on the stack CAN execute.
# ============================================================
stack-smash: main.c
	$(CC) $(CFLAGS) -fno-stack-protector -no-pie -z execstack \
		$(LDFLAGS) -o $@ main.c

# ============================================================
# ROP demo: NX enabled, but no canary, no PIE
# Shellcode on the stack will NOT execute (requires ROP).
# ============================================================
rop-demo: main.c
	$(CC) $(CFLAGS) -fno-stack-protector -no-pie \
		$(LDFLAGS) -o $@ main.c

# ============================================================
# Modern demo: canary + PIE + NX + full RELRO
# Demonstrates defense-in-depth.
# ============================================================
modern: main.c
	$(CC) $(CFLAGS) -fstack-protector-strong -pie -fpie \
		-Wl,-z,relro,-z,now \
		$(LDFLAGS) -o $@ main.c

# ============================================================
# RISC-V target (requires cross-compiler)
# ============================================================
riscv: main.s
	riscv64-linux-gnu-gcc -static -o $@ main.s

# ============================================================
# Convenience targets
# ============================================================

# Run the stack-smash exploit.
# Pipe the generated payload into the vulnerable program.
run-smash: stack-smash
	@echo "=== Stack Smash ==="
	@printf "Payload for this binary:\n"
	@./stack-smash smash-payload 2>/dev/null | head -c80 > /tmp/smash_payload.bin
	@xxd /tmp/smash_payload.bin | head -5
	@printf "Running exploit...\n"
	@./stack-smash run-smash < /tmp/smash_payload.bin || true

# Run the ROP exploit (requires gadget addresses).
run-rop: rop-demo
	@echo "=== ROP Demo ==="
	@echo "Step 1: Find gadgets in the binary:"
	@echo "  ROPgadget --binary rop-demo | grep 'pop rdi ; ret$$'"
	@echo "Step 2: Run: ./rop-demo rop-payload <gadget_addr> <arg_addr>"
	@echo "Step 3: Pipe payload: ./rop-demo rop-payload ... | ./rop-demo run-rop"

run-modern: modern
	@echo "=== Modern (Protected) ==="
	@echo "This binary has canary + PIE + NX + RELRO."
	@echo "A buffer overflow will be caught by the canary:"
	@python3 -c "import sys; sys.stdout.buffer.write(b'A'*88 + b'BBBBBBBB')" \
		| ./modern run-smash 2>&1 || true
	@echo "(Expected: __stack_chk_fail or SIGABRT)"

# Find gadgets in the ROP binary.
gadgets: rop-demo
	@echo "=== Finding ROP Gadgets ==="
	@if command -v ROPgadget >/dev/null 2>&1; then \
		ROPgadget --binary rop-demo | grep "pop rdi ; ret$$" || \
		echo "No pop rdi; ret gadget found (binary too small)"; \
		ROPgadget --binary rop-demo | grep "pop rsi ; pop r15 ; ret$$" || \
		echo "No pop rsi; pop r15; ret gadget found"; \
	else \
		echo "ROPgadget not found (pip install ROPgadget)."; \
		echo "Falling back to objdump:"; \
		objdump -d rop-demo | grep -A2 'pop.*rdi' || echo "(none found)"; \
		objdump -d rop-demo | grep -B1 'ret$$' | grep 'pop' | head -10; \
	fi

# Compare binary protection status.
check:
	@echo "=== Stack Smash protections ==="
	@checksec --file=stack-smash 2>/dev/null || readelf -l stack-smash | grep -q GNU_STACK && echo "Stack: executable"
	@echo "=== ROP Demo protections ==="
	@checksec --file=rop-demo 2>/dev/null || readelf -l rop-demo | grep GNU_STACK
	@echo "=== Modern protections ==="
	@checksec --file=modern 2>/dev/null || readelf -l modern | grep GNU_STACK

clean:
	rm -f stack-smash rop-demo modern riscv
	rm -f /tmp/smash_payload.bin
