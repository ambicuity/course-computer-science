# Build a Quantum Circuit Simulator

> Quantum computing is linear algebra on complex vectors with tensor products.

**Type:** Build
**Languages:** Python, Rust
**Prerequisites:** Phase 19 lessons 01-15
**Time:** ~600 minutes

## Learning Objectives

- Represent n-qubit states as complex vectors of size 2^n.
- Apply single-qubit and two-qubit gates via matrix multiplication.
- Implement measurement with probabilistic collapse.
- Understand why quantum simulation is exponentially expensive.

## The Problem

Quantum SDKs abstract details behind high-level APIs. You call `circuit.h(0)` and a Hadamard gate is applied, but you don't see what happens to the state vector. Real understanding requires implementing the mechanics yourself: how a gate transforms amplitudes, how entanglement arises from tensor products, and how measurement collapses superposition.

A state-vector simulator stores the full quantum state as a complex vector. For n qubits, the vector has 2^n entries. Each entry is the amplitude of a basis state. Gates are unitary matrices that transform the amplitudes. Measurement samples from the probability distribution given by |amplitude|^2.

This build gives you the core mechanics behind Qiskit/Cirq simulators. Production simulators optimize with sparse operations, tensor networks, or GPU kernels, but the semantics are identical: repeated unitary transforms followed by measurement.

## The Concept

A quantum circuit simulator has three components:

```
Quantum circuit (sequence of gates)
        │
        ▼
┌───────────────┐
│ 1. State       │  Initialize |00...0> as [1, 0, 0, ...]
│  Vector        │  Complex array of size 2^n
└───────────────┘
        │
        ▼
┌───────────────┐
│ 2. Gate        │  Apply unitary matrices to state
│  Application   │  Single-qubit: 2x2 matrix
│                │  Two-qubit: 4x4 matrix (CNOT, CZ)
└───────────────┘
        │
        ▼
┌───────────────┐
│ 3. Measurement │  Sample from |amp|^2 distribution
│                │  Collapse state to measured basis
└───────────────┘
```

Key quantum gates:

```
Pauli-X (NOT):    Hadamard:         CNOT:
[0 1]             1/√2 [1  1]      [1 0 0 0]
[1 0]                  [1 -1]      [0 1 0 0]
                                   [0 0 0 1]
                                   [0 0 1 0]
```

Gate application on qubit q: pair indices differing only on bit q, apply matrix to each pair.

## Build It

### Step 1: State Vector and Gate Application (Python)

```python
import numpy as np
from typing import List, Tuple

class QuantumState:
    """State-vector quantum simulator for n qubits."""

    def __init__(self, n_qubits: int):
        self.n_qubits = n_qubits
        self.n_states = 2 ** n_qubits
        # Initialize to |00...0>
        self.amplitudes = np.zeros(self.n_states, dtype=complex)
        self.amplitudes[0] = 1.0

    def apply_single_qubit_gate(self, gate: np.ndarray, target: int):
        """Apply a 2x2 gate to the target qubit."""
        assert gate.shape == (2, 2), "Gate must be 2x2"
        assert 0 <= target < self.n_qubits, "Target qubit out of range"

        # For each pair of states differing only in bit 'target'
        for i in range(self.n_states):
            if (i >> target) & 1 == 0:  # Only process the |0> side of the pair
                j = i | (1 << target)   # The |1> side
                a0 = self.amplitudes[i]
                a1 = self.amplitudes[j]
                # Apply gate matrix
                self.amplitudes[i] = gate[0, 0] * a0 + gate[0, 1] * a1
                self.amplitudes[j] = gate[1, 0] * a0 + gate[1, 1] * a1

    def apply_two_qubit_gate(self, gate: np.ndarray, qubit1: int, qubit2: int):
        """Apply a 4x4 gate to two qubits."""
        assert gate.shape == (4, 4), "Gate must be 4x4"

        for i in range(self.n_states):
            # Extract bits at qubit1 and qubit2
            b1 = (i >> qubit1) & 1
            b2 = (i >> qubit2) & 1
            # Only process when both bits are 0 (to avoid double-processing)
            if b1 == 0 and b2 == 0:
                # The four basis states: |00>, |01>, |10>, |11>
                idx00 = i
                idx01 = i | (1 << qubit2)
                idx10 = i | (1 << qubit1)
                idx11 = i | (1 << qubit1) | (1 << qubit2)

                a = [self.amplitudes[idx00], self.amplitudes[idx01],
                     self.amplitudes[idx10], self.amplitudes[idx11]]

                # Apply 4x4 matrix
                for row in range(4):
                    target_idx = [idx00, idx01, idx10, idx11][row]
                    self.amplitudes[target_idx] = sum(
                        gate[row, col] * a[col] for col in range(4)
                    )

    def measure(self) -> int:
        """Measure all qubits, returning the basis state index."""
        probabilities = np.abs(self.amplitudes) ** 2
        # Sample from the probability distribution
        result = np.random.choice(self.n_states, p=probabilities)
        # Collapse the state
        self.amplitudes = np.zeros(self.n_states, dtype=complex)
        self.amplitudes[result] = 1.0
        return result

    def probabilities(self) -> np.ndarray:
        """Return the probability of each basis state."""
        return np.abs(self.amplitudes) ** 2

    def __repr__(self):
        lines = []
        for i in range(self.n_states):
            prob = abs(self.amplitudes[i]) ** 2
            if prob > 1e-10:
                basis = format(i, f'0{self.n_qubits}b')
                lines.append(f"  |{basis}>: amplitude={self.amplitudes[i]:.4f}, prob={prob:.4f}")
        return "\n".join(lines)
```

### Step 2: Standard Gates

```python
# Standard quantum gates
I = np.array([[1, 0], [0, 1]], dtype=complex)  # Identity
X = np.array([[0, 1], [1, 0]], dtype=complex)  # Pauli-X (NOT)
Y = np.array([[0, -1j], [1j, 0]], dtype=complex)  # Pauli-Y
Z = np.array([[1, 0], [0, -1]], dtype=complex)  # Pauli-Z
H = np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2)  # Hadamard

# CNOT gate (control=qubit0, target=qubit1)
CNOT = np.array([
    [1, 0, 0, 0],
    [0, 1, 0, 0],
    [0, 0, 0, 1],
    [0, 0, 1, 0],
], dtype=complex)

# Phase gate
def phase_gate(theta: float) -> np.ndarray:
    return np.array([[1, 0], [0, np.exp(1j * theta)]], dtype=complex)

# Rotation gates
def rx(theta: float) -> np.ndarray:
    return np.array([
        [np.cos(theta/2), -1j*np.sin(theta/2)],
        [-1j*np.sin(theta/2), np.cos(theta/2)]
    ], dtype=complex)

def rz(theta: float) -> np.ndarray:
    return np.array([
        [np.exp(-1j*theta/2), 0],
        [0, np.exp(1j*theta/2)]
    ], dtype=complex)
```

### Step 3: Circuit Builder and Demo

```python
class QuantumCircuit:
    """High-level circuit builder."""

    def __init__(self, n_qubits: int):
        self.n_qubits = n_qubits
        self.gates = []

    def h(self, target: int):
        self.gates.append(('h', target))
        return self

    def x(self, target: int):
        self.gates.append(('x', target))
        return self

    def cnot(self, control: int, target: int):
        self.gates.append(('cnot', control, target))
        return self

    def measure(self):
        self.gates.append(('measure',))
        return self

    def run(self, shots: int = 1024) -> dict:
        """Execute the circuit and return measurement counts."""
        counts = {}
        for _ in range(shots):
            state = QuantumState(self.n_qubits)
            for gate in self.gates:
                if gate[0] == 'h':
                    state.apply_single_qubit_gate(H, gate[1])
                elif gate[0] == 'x':
                    state.apply_single_qubit_gate(X, gate[1])
                elif gate[0] == 'cnot':
                    state.apply_two_qubit_gate(CNOT, gate[1], gate[2])
                elif gate[0] == 'measure':
                    result = state.measure()
                    basis = format(result, f'0{self.n_qubits}b')
                    counts[basis] = counts.get(basis, 0) + 1
        return counts


def main():
    print("=== Bell State Circuit ===")
    # Create Bell state: H on qubit 0, then CNOT(0,1)
    # Result: (|00> + |11>) / sqrt(2)
    circuit = QuantumCircuit(2)
    circuit.h(0)
    circuit.cnot(0, 1)
    circuit.measure()

    counts = circuit.run(shots=1000)
    print("Measurement counts (1000 shots):")
    for basis, count in sorted(counts.items()):
        print(f"  |{basis}>: {count} ({count/10:.1f}%)")

    print("\n=== GHZ State (3 qubits) ===")
    # GHZ state: H on qubit 0, CNOT(0,1), CNOT(0,2)
    # Result: (|000> + |111>) / sqrt(2)
    circuit2 = QuantumCircuit(3)
    circuit2.h(0)
    circuit2.cnot(0, 1)
    circuit2.cnot(0, 2)
    circuit2.measure()

    counts2 = circuit2.run(shots=1000)
    print("Measurement counts (1000 shots):")
    for basis, count in sorted(counts2.items()):
        print(f"  |{basis}>: {count} ({count/10:.1f}%)")


if __name__ == "__main__":
    main()
```

Expected output:

```
=== Bell State Circuit ===
Measurement counts (1000 shots):
  |00>: 502 (50.2%)
  |11>: 498 (49.8%)

=== GHZ State (3 qubits) ===
Measurement counts (1000 shots):
  |000>: 497 (49.7%)
  |111>: 503 (50.3%)
```

## Use It

Production simulators optimize with sparse operations, tensor networks, or GPU kernels, but the semantics are identical:

- **Qiskit Aer**: IBM's quantum simulator. Uses state-vector, density matrix, and tensor network methods. The state-vector simulator is optimized with SIMD and multi-threading.
- **Cirq**: Google's quantum framework. The simulator uses the same state-vector approach with NumPy arrays.
- **QuEST**: A high-performance quantum simulator using OpenMP and GPU acceleration. Simulates up to ~30 qubits on a single machine.

The key production lesson: **2^n memory growth is the fundamental limit**. A 30-qubit state vector needs 2^30 × 16 bytes = 16 GB. A 40-qubit state needs 16 TB. Beyond 40 qubits, you need either a distributed simulator, a tensor network simulator (which exploits circuit structure), or actual quantum hardware.

## Read the Source

- [Qiskit Aer](https://github.com/Qiskit/qiskit-aer) — IBM's quantum simulator with state-vector, density matrix, and tensor network backends.
- [Cirq simulators](https://quantumai.google/cirq/simulate) — Google's simulator implementations and gate protocol code.
- [Quantum Computation and Quantum Information](https://www.cambridge.org/core/books/quantum-computation-and-quantum-information/01E10196D0A682A6AEFFEA52D53BE9AE) — Nielsen and Chuang. The standard textbook.

## Ship It

- `code/main.py`: state-vector simulator with single/two-qubit gates, measurement, and Bell/GHZ demos.
- `outputs/README.md`: sample Bell-state circuit with probability table and scalability analysis.

## Exercises

1. **Easy** — Add parameterized rotation gates (Rx, Rz). Implement `rx(theta)` and `rz(theta)` as 2x2 unitary matrices. Demonstrate that `rz(pi)` is equivalent to the Pauli-Z gate (up to global phase).
2. **Medium** — Add repeated-shot sampling. Run the circuit 1000 times, collect measurement outcomes, and plot a histogram. Show that the Bell state produces approximately 50% |00> and 50% |11>.
3. **Hard** — Add a simple circuit parser text format. Parse lines like `H 0`, `CNOT 0 1`, `MEASURE` from a text file and execute them. Support comments (lines starting with #).

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Qubit | "quantum bit" | A 2-dimensional complex state vector: alpha|0> + beta|1> where |alpha|^2 + |beta|^2 = 1. Unlike classical bits, qubits can be in superposition. |
| Amplitude | "complex coefficient" | The complex number alpha in alpha|0> + beta|1>. The probability of measuring |0> is |alpha|^2. Amplitudes can interfere constructively or destructively. |
| Unitary | "reversible transform" | A matrix U where U*U^dagger = I. Quantum gates are unitary: they preserve total probability (norm). This means quantum computation is reversible. |
| Entanglement | "correlated quantum state" | A joint state that cannot be factored into per-qubit states. The Bell state (|00> + |11>)/sqrt(2) is entangled: measuring one qubit instantly determines the other. |
| Measurement | "collapse/readout" | The process of extracting a classical bit from a qubit. Measurement collapses the superposition to a basis state with probability |amplitude|^2. It is irreversible and probabilistic. |

## Further Reading

- [Qiskit textbook](https://qiskit.org/learn/) — IBM's quantum computing curriculum with interactive notebooks.
- [Quantum Country](https://quantum.country/) — Andy Matuschak and Michael Nielsen's spaced-repetition introduction to quantum computing.
- [Nielsen and Chuang](https://www.cambridge.org/core/books/quantum-computation-and-quantum-information/) — The standard textbook on quantum computing.
