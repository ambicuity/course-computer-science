# Running the Pipelined RISC-V CPU

## Files

| File | Description |
|------|-------------|
| `../code/pipelined_cpu.sv` | Complete 5-stage pipelined CPU with testbench |
| `../code/programs.s` | RISC-V assembly test programs |

## Quick Start (Icarus Verilog)

```bash
# Install iverilog (macOS)
brew install icarus-verilog

# Compile and run (requires SV support — use Verilator or commercial sim)
iverilog -g2012 -o cpu_sim ../code/pipelined_cpu.sv
vvp cpu_sim
```

## Using Verilator (Recommended)

```bash
# Install Verilator
brew install verilator   # macOS
sudo apt install verilator  # Ubuntu/Debian

# Lint the design
verilator --lint-only -Wall ../code/pipelined_cpu.sv

# Build and run C++ simulation
verilator --cc --exe --build ../code/pipelined_cpu.sv \
          -o cpu_sim

# Run
./obj_dir/cpu_sim
```

## Using Commercial Simulators

### Synopsys VCS
```bash
vcs -sverilog ../code/pipelined_cpu.sv -o cpu_sim
./cpu_sim
```

### Cadence Xcelium
```bash
xrun -sv ../code/pipelined_cpu.sv
```

### Mentor Questa
```bash
vsim -c -do "run -all" work.tb_pipelined_cpu
```

## Preparing Test Programs

The instruction memory loads from `program.hex`. To generate it from assembly:

```bash
# Assemble
riscv64-unknown-elf-as -march=rv32i -mabi=ilp32 ../code/programs.s -o programs.o

# Link at address 0
riscv64-unknown-elf-ld -Ttext=0 programs.o -o programs.elf

# Convert to flat hex (one instruction per line, 8 hex digits)
riscv64-unknown-elf-objcopy -O binary programs.elf programs.bin
xxd -p -c 4 programs.bin > program.hex
```

Or use the included Python helper:

```python
import struct

# Manually encode a few test instructions
# ADDI x1, x0, 5   → 0x00500093
# ADDI x2, x0, 3   → 0x00300113
# ADD  x3, x1, x2  → 0x002081B3

instructions = [
    0x00500093,  # ADDI x1, x0, 5
    0x00300113,  # ADDI x2, x0, 3
    0x002081B3,  # ADD  x3, x1, x2
    0x00000013,  # NOP
]

with open("program.hex", "w") as f:
    for instr in instructions:
        f.write(f"{instr:08x}\n")
```

## Expected Output

After running a test program, the testbench dumps all 32 registers. For the Fibonacci program:

```
x10 = 55 (0x00000037)    # fib(10) = 55
```

For the bubble sort program:

```
x12 = 1                   # arr[0]
x13 = 2                   # arr[1]
x14 = 3                   # arr[2]
x15 = 5                   # arr[3]
x16 = 8                   # arr[4]
```

## Viewing Waveforms

```bash
# Generate VCD dump (built into testbench)
# Then open with GTKWave
gtkwave cpu.vcd
```

Look at:
- `if_pc` — PC trace shows instruction flow
- `stall` — should assert briefly on load-use hazards
- `branch_taken` / `flush_if_id` — shows branch penalty
- `forward_A` / `forward_B` — shows forwarding in action (10 = EX hazard, 01 = MEM hazard)
