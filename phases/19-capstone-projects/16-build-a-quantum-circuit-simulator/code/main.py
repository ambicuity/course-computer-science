# Build a Quantum Circuit Simulator
# Run: python3 main.py
#
# Architecture:
#   QuantumState (state vector) → Gate Application (matrix multiply) → Measurement
#
# Implements a state-vector quantum simulator with single/two-qubit gate application,
# probabilistic measurement, and a high-level circuit builder with fluent API.

import numpy as np
from typing import List, Tuple

# =============================================================================
# Step 1: State Vector and Gate Application
# =============================================================================

class QuantumState:
    """State-vector quantum simulator for n qubits."""

    def __init__(self, n_qubits: int):
        self.n_qubits = n_qubits
        self.n_states = 2 ** n_qubits
        self.amplitudes = np.zeros(self.n_states, dtype=complex)
        self.amplitudes[0] = 1.0  # Initialize to |00...0>

    def apply_single_qubit_gate(self, gate: np.ndarray, target: int):
        """Apply a 2x2 gate to the target qubit."""
        assert gate.shape == (2, 2)
        assert 0 <= target < self.n_qubits
        for i in range(self.n_states):
            if (i >> target) & 1 == 0:  # Only process |0> side
                j = i | (1 << target)   # The |1> side
                a0, a1 = self.amplitudes[i], self.amplitudes[j]
                self.amplitudes[i] = gate[0, 0] * a0 + gate[0, 1] * a1
                self.amplitudes[j] = gate[1, 0] * a0 + gate[1, 1] * a1

    def apply_two_qubit_gate(self, gate: np.ndarray, qubit1: int, qubit2: int):
        """Apply a 4x4 gate to two qubits."""
        assert gate.shape == (4, 4)
        for i in range(self.n_states):
            b1 = (i >> qubit1) & 1
            b2 = (i >> qubit2) & 1
            if b1 == 0 and b2 == 0:
                idx00 = i
                idx01 = i | (1 << qubit2)
                idx10 = i | (1 << qubit1)
                idx11 = i | (1 << qubit1) | (1 << qubit2)
                a = [self.amplitudes[idx00], self.amplitudes[idx01],
                     self.amplitudes[idx10], self.amplitudes[idx11]]
                for row in range(4):
                    target_idx = [idx00, idx01, idx10, idx11][row]
                    self.amplitudes[target_idx] = sum(gate[row, col] * a[col] for col in range(4))

    def measure(self) -> int:
        """Measure all qubits, returning the basis state index."""
        probabilities = np.abs(self.amplitudes) ** 2
        result = np.random.choice(self.n_states, p=probabilities)
        self.amplitudes = np.zeros(self.n_states, dtype=complex)
        self.amplitudes[result] = 1.0
        return result

    def probabilities(self) -> np.ndarray:
        return np.abs(self.amplitudes) ** 2

    def __repr__(self):
        lines = []
        for i in range(self.n_states):
            prob = abs(self.amplitudes[i]) ** 2
            if prob > 1e-10:
                basis = format(i, f'0{self.n_qubits}b')
                lines.append(f"  |{basis}>: amplitude={self.amplitudes[i]:.4f}, prob={prob:.4f}")
        return "\n".join(lines)

# =============================================================================
# Step 2: Standard Gates
# =============================================================================

I = np.array([[1, 0], [0, 1]], dtype=complex)
X = np.array([[0, 1], [1, 0]], dtype=complex)
Y = np.array([[0, -1j], [1j, 0]], dtype=complex)
Z = np.array([[1, 0], [0, -1]], dtype=complex)
H = np.array([[1, 1], [1, -1]], dtype=complex) / np.sqrt(2)

CNOT = np.array([
    [1, 0, 0, 0],
    [0, 1, 0, 0],
    [0, 0, 0, 1],
    [0, 0, 1, 0],
], dtype=complex)

def phase_gate(theta: float) -> np.ndarray:
    return np.array([[1, 0], [0, np.exp(1j * theta)]], dtype=complex)

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

# =============================================================================
# Step 3: Circuit Builder and Demo
# =============================================================================

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
        counts = {}
        for _ in range(shots):
            state = QuantumState(self.n_qubits)
            for gate in self.gates:
                if gate[0] == 'h': state.apply_single_qubit_gate(H, gate[1])
                elif gate[0] == 'x': state.apply_single_qubit_gate(X, gate[1])
                elif gate[0] == 'cnot': state.apply_two_qubit_gate(CNOT, gate[1], gate[2])
                elif gate[0] == 'measure':
                    result = state.measure()
                    basis = format(result, f'0{self.n_qubits}b')
                    counts[basis] = counts.get(basis, 0) + 1
        return counts


def main():
    print("=== Bell State Circuit ===")
    circuit = QuantumCircuit(2)
    circuit.h(0).cnot(0, 1).measure()
    counts = circuit.run(shots=1000)
    print("Measurement counts (1000 shots):")
    for basis, count in sorted(counts.items()):
        print(f"  |{basis}>: {count} ({count/10:.1f}%)")

    print("\n=== GHZ State (3 qubits) ===")
    circuit2 = QuantumCircuit(3)
    circuit2.h(0).cnot(0, 1).cnot(0, 2).measure()
    counts2 = circuit2.run(shots=1000)
    print("Measurement counts (1000 shots):")
    for basis, count in sorted(counts2.items()):
        print(f"  |{basis}>: {count} ({count/10:.1f}%)")


if __name__ == "__main__":
    main()
