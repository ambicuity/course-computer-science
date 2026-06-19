"""
Out-of-Order Execution & Tomasulo's Algorithm Simulator
Phase 06 — Digital Logic & Computer Architecture

A complete Tomasulo simulator with reservation stations, common data bus
broadcast, and reorder buffer for in-order commit of out-of-order results.
"""

from dataclasses import dataclass
from enum import Enum, auto
from typing import Optional


# ---------------------------------------------------------------------------
# Data types
# ---------------------------------------------------------------------------

class Op(Enum):
    ADD = auto()
    SUB = auto()
    MUL = auto()
    LD = auto()


class ROBState(Enum):
    ISSUE = auto()
    EXECUTE = auto()
    WRITEBACK = auto()
    COMMIT = auto()


LATENCY = {
    Op.ADD: 1,
    Op.SUB: 1,
    Op.MUL: 3,
    Op.LD: 3,
}


@dataclass
class Instruction:
    op: Op
    dest: str       # destination register
    src1: str       # first source register
    src2: str = ""  # second source (empty for LD)
    latency: int = 0

    def __post_init__(self):
        if self.latency == 0:
            self.latency = LATENCY[self.op]

    def __repr__(self):
        if self.op == Op.LD:
            return f"LD  {self.dest}, {self.src1}"
        return f"{self.op.name:3s} {self.dest}, {self.src1}, {self.src2}"


@dataclass
class ReservationStation:
    name: str
    op: Optional[Op] = None
    vj: Optional[int] = None
    vk: Optional[int] = None
    qj: Optional[str] = None   # tag of RS that will produce j
    qk: Optional[str] = None   # tag of RS that will produce k
    dest: Optional[str] = None  # ROB entry name
    busy: bool = False
    cycles_left: int = 0

    def ready(self) -> bool:
        return (
            self.busy
            and self.qj is None
            and self.qk is None
            and self.cycles_left > 0
        )

    def clear(self):
        self.op = None
        self.vj = self.vk = None
        self.qj = self.qk = None
        self.dest = None
        self.busy = False
        self.cycles_left = 0


@dataclass
class ROBEntry:
    name: str
    instruction: Optional[Instruction] = None
    state: ROBState = ROBState.ISSUE
    dest: Optional[str] = None
    value: Optional[int] = None
    ready_to_commit: bool = False

    def clear(self):
        self.instruction = None
        self.state = ROBState.ISSUE
        self.dest = None
        self.value = None
        self.ready_to_commit = False


# ---------------------------------------------------------------------------
# Tomasulo Simulator
# ---------------------------------------------------------------------------

class TomasuloSimulator:
    """Full Tomasulo algorithm with reservation stations, CDB, and ROB."""

    def __init__(self, num_alu: int = 2, num_ld: int = 1, num_mul: int = 1):
        self.cycle = 0
        self.pc = 0
        self.program: list[Instruction] = []

        # Register file and status
        self.registers: dict[str, int] = {f"F{i}": 0 for i in range(16)}
        self.reg_status: dict[str, Optional[str]] = {
            f"F{i}": None for i in range(16)
        }

        # Reservation stations
        self.rs: list[ReservationStation] = []
        for i in range(num_alu):
            self.rs.append(ReservationStation(name=f"RS_ALU{i}"))
        for i in range(num_ld):
            self.rs.append(ReservationStation(name=f"RS_LD{i}"))
        for i in range(num_mul):
            self.rs.append(ReservationStation(name=f"RS_MUL{i}"))

        # Reorder buffer (circular)
        self.rob_size = 8
        self.rob: list[ROBEntry] = [
            ROBEntry(name=f"ROB{i}") for i in range(self.rob_size)
        ]
        self.rob_head = 0
        self.rob_tail = 0
        self.rob_count = 0

        self.log: list[str] = []

    # ---- internal helpers ------------------------------------------------

    def _rs_for_op(self, op: Op) -> Optional[ReservationStation]:
        for rs in self.rs:
            if rs.busy:
                continue
            if op in (Op.ADD, Op.SUB) and "ALU" in rs.name:
                return rs
            if op == Op.LD and "LD" in rs.name:
                return rs
            if op == Op.MUL and "MUL" in rs.name:
                return rs
        return None

    def _rob_full(self) -> bool:
        return self.rob_count >= self.rob_size

    def _rob_alloc(self) -> Optional[ROBEntry]:
        if self._rob_full():
            return None
        entry = self.rob[self.rob_tail]
        self.rob_tail = (self.rob_tail + 1) % self.rob_size
        self.rob_count += 1
        return entry

    def _resolve_operand(self, reg: str):
        """Return (value, tag).  If tag is not None the value is pending."""
        # Registers not in status table (e.g. R1 address regs) are always ready
        if reg not in self.reg_status:
            return self.registers.get(reg, 0), None
        tag = self.reg_status[reg]
        if tag is not None:
            # Check if the ROB entry already wrote back
            for entry in self.rob:
                if entry.name == tag and entry.state == ROBState.WRITEBACK:
                    return entry.value, None
            return None, tag
        return self.registers.get(reg, 0), None

    # ---- phases ----------------------------------------------------------

    def issue(self) -> bool:
        """Phase 1: Issue — allocate RS + ROB entry."""
        if self.pc >= len(self.program):
            return False
        if self._rob_full():
            return False

        instr = self.program[self.pc]
        rs = self._rs_for_op(instr.op)
        if rs is None:
            return False

        rob_entry = self._rob_alloc()
        assert rob_entry is not None

        rob_entry.instruction = instr
        rob_entry.state = ROBState.ISSUE
        rob_entry.dest = instr.dest

        rs.op = instr.op
        rs.busy = True
        rs.dest = rob_entry.name
        rs.cycles_left = instr.latency

        val, tag = self._resolve_operand(instr.src1)
        if tag is not None:
            rs.qj = tag
        else:
            rs.vj = val

        if instr.src2:
            val, tag = self._resolve_operand(instr.src2)
            if tag is not None:
                rs.qk = tag
            else:
                rs.vk = val

        if instr.dest in self.reg_status:
            self.reg_status[instr.dest] = rob_entry.name
        self.log.append(
            f"Cycle {self.cycle:2d}: ISSUE    {instr!s:20s} -> {rs.name}, {rob_entry.name}"
        )
        self.pc += 1
        return True

    def execute(self):
        """Phase 2: Execute — countdown ready instructions, broadcast on CDB."""
        for rs in self.rs:
            if not rs.ready():
                continue
            rs.cycles_left -= 1
            if rs.cycles_left == 0:
                vj = rs.vj or 0
                vk = rs.vk or 0
                if rs.op == Op.ADD:
                    result = vj + vk
                elif rs.op == Op.SUB:
                    result = vj - vk
                elif rs.op == Op.MUL:
                    result = vj * vk
                elif rs.op == Op.LD:
                    result = vj + 100  # simulated memory value
                else:
                    result = 0

                tag = rs.dest

                # Update ROB entry
                for entry in self.rob:
                    if entry.name == tag:
                        entry.state = ROBState.WRITEBACK
                        entry.value = result
                        entry.ready_to_commit = True
                        break

                # CDB broadcast: resolve all dependent reservation stations
                for other in self.rs:
                    if other.busy and other.name != rs.name:
                        if other.qj == tag:
                            other.vj = result
                            other.qj = None
                        if other.qk == tag:
                            other.vk = result
                            other.qk = None

                # Free this reservation station
                rs.clear()

                self.log.append(
                    f"Cycle {self.cycle:2d}: EXECUTE  {tag} -> {result}"
                )

    def writeback(self):
        """Phase 3: Writeback — no-op (CDB broadcast happens in execute)."""
        pass

    def commit(self):
        """Phase 4: Commit — retire in order from ROB head."""
        while self.rob_count > 0:
            entry = self.rob[self.rob_head]
            if entry.state != ROBState.WRITEBACK:
                break
            if entry.dest and entry.value is not None:
                self.registers[entry.dest] = entry.value
            if entry.dest in self.reg_status:
                self.reg_status[entry.dest] = None
            self.log.append(
                f"Cycle {self.cycle:2d}: COMMIT   {entry.name} {entry.dest} = {entry.value}"
            )
            entry.clear()
            self.rob_head = (self.rob_head + 1) % self.rob_size
            self.rob_count -= 1

    # ---- driver ----------------------------------------------------------

    def run(self, program: list[Instruction], max_cycles: int = 50) -> int:
        self.program = program
        self.pc = 0
        self.cycle = 0
        self.log.clear()

        while self.cycle < max_cycles:
            self.cycle += 1
            self.log.append(f"--- Cycle {self.cycle} ---")

            self.commit()
            self.writeback()
            self.execute()
            self.issue()

            if self.pc >= len(self.program) and self.rob_count == 0:
                break

        return self.cycle

    def print_state(self):
        print("\n=== Reservation Stations ===")
        print(
            f"{'Name':<10} {'Busy':<6} {'Op':<6} {'Vj':<6} "
            f"{'Vk':<6} {'Qj':<8} {'Qk':<8} {'Dest':<8}"
        )
        for rs in self.rs:
            print(
                f"{rs.name:<10} {str(rs.busy):<6} "
                f"{(rs.op.name if rs.op else '-'): <6} "
                f"{str(rs.vj or '-'): <6} {str(rs.vk or '-'): <6} "
                f"{(rs.qj or '-'): <8} {(rs.qk or '-'): <8} "
                f"{(rs.dest or '-'): <8}"
            )

        print("\n=== Reorder Buffer ===")
        print(f"{'Name':<8} {'State':<12} {'Dest':<6} {'Value':<6}")
        idx = self.rob_head
        for _ in range(self.rob_count):
            e = self.rob[idx]
            print(
                f"{e.name:<8} {e.state.name:<12} "
                f"{(e.dest or '-'): <6} {str(e.value or '-'): <6}"
            )
            idx = (idx + 1) % self.rob_size

        print("\n=== Register Status (occupied) ===")
        occupied = {k: v for k, v in self.reg_status.items() if v is not None}
        if occupied:
            for reg, tag in sorted(occupied.items()):
                print(f"  {reg} -> {tag}")
        else:
            print("  (all clear)")

        print("\n=== Log ===")
        for line in self.log:
            print(line)


# ---------------------------------------------------------------------------
# Demo programs
# ---------------------------------------------------------------------------

def demo_dependency_chain():
    """Load → dependent add → dependent mul, with independent sub sneaking in."""
    program = [
        Instruction(Op.LD,  "F1",  "R1"),
        Instruction(Op.ADD, "F2",  "F1", "F3"),
        Instruction(Op.MUL, "F4",  "F2", "F5"),
        Instruction(Op.SUB, "F6",  "F7", "F8"),
        Instruction(Op.ADD, "F9",  "F10", "F11"),
    ]

    sim = TomasuloSimulator(num_alu=2, num_ld=1, num_mul=1)
    sim.registers["F3"] = 10
    sim.registers["F5"] = 5
    sim.registers["F7"] = 20
    sim.registers["F8"] = 7
    sim.registers["F10"] = 3
    sim.registers["F11"] = 4
    sim.registers["R1"] = 100

    print("=" * 60)
    print("Demo 1: Dependency Chain — OoO hides load latency")
    print("=" * 60)
    for instr in program:
        print(f"  {instr}")

    total = sim.run(program)
    print(f"\nFinished in {total} cycles")
    sim.print_state()


def demo_independent_burst():
    """Five independent ALU ops — maximum parallelism."""
    program = [
        Instruction(Op.ADD, "F1",  "F2", "F3"),
        Instruction(Op.SUB, "F4",  "F5", "F6"),
        Instruction(Op.ADD, "F7",  "F8", "F9"),
        Instruction(Op.SUB, "F10", "F11", "F12"),
        Instruction(Op.ADD, "F13", "F14", "F15"),
    ]

    sim = TomasuloSimulator(num_alu=2)
    for i in range(2, 16):
        sim.registers[f"F{i}"] = i

    print("\n" + "=" * 60)
    print("Demo 2: Independent Burst — OoO overlaps everything")
    print("=" * 60)
    for instr in program:
        print(f"  {instr}")

    total = sim.run(program)
    print(f"\nFinished in {total} cycles (in-order would take 5)")
    sim.print_state()


def demo_waw_hazard():
    """Two writes to F1 — ROB prevents WAW hazard."""
    program = [
        Instruction(Op.MUL, "F1", "F2", "F3"),   # slow, issues first
        Instruction(Op.ADD, "F1", "F4", "F5"),   # fast, issues second
    ]

    sim = TomasuloSimulator(num_alu=1, num_mul=1)
    sim.registers["F2"] = 6
    sim.registers["F3"] = 7
    sim.registers["F4"] = 10
    sim.registers["F5"] = 20

    print("\n" + "=" * 60)
    print("Demo 3: WAW Hazard — ROB commits in order")
    print("=" * 60)
    for instr in program:
        print(f"  {instr}")

    total = sim.run(program)
    print(f"\nFinished in {total} cycles")
    print(f"F1 = {sim.registers['F1']}  (correct: 30 from ADD, last to commit in program order)")
    sim.print_state()


def main():
    demo_dependency_chain()
    demo_independent_burst()
    demo_waw_hazard()


if __name__ == "__main__":
    main()
