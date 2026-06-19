# Build a Toy ML Framework (autodiff, optimizers)
# Run: python3 main.py
#
# Architecture:
#   Value (computation graph) → Neuron → Layer → MLP → Loss → Backward → SGD
#
# Implements a complete scalar autodiff engine with reverse-mode differentiation,
# neural network building blocks, and gradient checking.

import math
import random
from typing import Callable, Optional

# =============================================================================
# Step 1: Value — Scalar Autodiff Engine
# =============================================================================

class Value:
    """A scalar value in a computation graph with automatic differentiation."""

    def __init__(self, data: float, _children=(), _op: str = '', label: str = ''):
        self.data = data
        self.grad = 0.0
        self._backward: Callable = lambda: None
        self._prev = set(_children)
        self._op = _op
        self.label = label

    def __repr__(self):
        return f"Value({self.data:.4f}, grad={self.grad:.4f})"

    def __add__(self, other):
        other = other if isinstance(other, Value) else Value(other)
        out = Value(self.data + other.data, (self, other), '+')
        def _backward():
            self.grad += 1.0 * out.grad
            other.grad += 1.0 * out.grad
        out._backward = _backward
        return out

    def __radd__(self, other):
        return self + other

    def __mul__(self, other):
        other = other if isinstance(other, Value) else Value(other)
        out = Value(self.data * other.data, (self, other), '*')
        def _backward():
            self.grad += other.data * out.grad
            other.grad += self.data * out.grad
        out._backward = _backward
        return out

    def __rmul__(self, other):
        return self * other

    def __pow__(self, other):
        assert isinstance(other, (int, float))
        out = Value(self.data ** other, (self,), f'**{other}')
        def _backward():
            self.grad += (other * self.data ** (other - 1)) * out.grad
        out._backward = _backward
        return out

    def __neg__(self):
        return self * -1

    def __sub__(self, other):
        return self + (-other)

    def __truediv__(self, other):
        return self * other**-1

    def tanh(self):
        t = math.tanh(self.data)
        out = Value(t, (self,), 'tanh')
        def _backward():
            self.grad += (1 - t**2) * out.grad
        out._backward = _backward
        return out

    def relu(self):
        out = Value(max(0, self.data), (self,), 'relu')
        def _backward():
            self.grad += (1.0 if self.data > 0 else 0.0) * out.grad
        out._backward = _backward
        return out

    def exp(self):
        x = self.data
        out = Value(math.exp(x), (self,), 'exp')
        def _backward():
            self.grad += out.data * out.grad
        out._backward = _backward
        return out

    def backward(self):
        """Compute gradients via reverse-mode autodiff (topological sort)."""
        topo = []
        visited = set()
        def build_topo(v):
            if v not in visited:
                visited.add(v)
                for child in v._prev:
                    build_topo(child)
                topo.append(v)
        build_topo(self)
        self.grad = 1.0
        for v in reversed(topo):
            v._backward()

# =============================================================================
# Step 2: Neural Network Building Blocks
# =============================================================================

class Neuron:
    """A single neuron: w * x + b, then activation."""

    def __init__(self, nin: int, activation: str = 'tanh'):
        self.w = [Value(random.uniform(-1, 1)) for _ in range(nin)]
        self.b = Value(0)
        self.activation = activation

    def __call__(self, x):
        act = sum((wi * xi for wi, xi in zip(self.w, x)), self.b)
        if self.activation == 'tanh':
            return act.tanh()
        elif self.activation == 'relu':
            return act.relu()
        return act

    def parameters(self):
        return self.w + [self.b]

class Layer:
    """A layer of neurons."""

    def __init__(self, nin: int, nout: int, activation: str = 'tanh'):
        self.neurons = [Neuron(nin, activation) for _ in range(nout)]

    def __call__(self, x):
        outs = [n(x) for n in self.neurons]
        return outs[0] if len(outs) == 1 else outs

    def parameters(self):
        return [p for n in self.neurons for p in n.parameters()]

class MLP:
    """Multi-layer perceptron."""

    def __init__(self, nin: int, nouts: list):
        sz = [nin] + nouts
        self.layers = [Layer(sz[i], sz[i+1]) for i in range(len(nouts))]

    def __call__(self, x):
        for layer in self.layers:
            x = layer(x)
        return x

    def parameters(self):
        return [p for layer in self.layers for p in layer.parameters()]

    def zero_grad(self):
        for p in self.parameters():
            p.grad = 0.0

# =============================================================================
# Step 3: Loss Function and Training Loop
# =============================================================================

def mse_loss(predictions, targets):
    """Mean squared error loss."""
    losses = [(pred - target)**2 for pred, target in zip(predictions, targets)]
    return sum(losses) / len(losses)

def main():
    # Simple dataset: learn XOR
    xs = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]
    ys = [0.0, 1.0, 1.0, 0.0]

    model = MLP(2, [4, 1])
    learning_rate = 0.05

    print(f"Model has {len(model.parameters())} parameters")

    for epoch in range(200):
        inputs = [[Value(x) for x in row] for row in xs]
        predictions = [model(x) for x in inputs]
        targets = [Value(y) for y in ys]

        loss = mse_loss(predictions, targets)

        model.zero_grad()
        loss.backward()

        for p in model.parameters():
            p.data -= learning_rate * p.grad

        if epoch % 50 == 0:
            print(f"Epoch {epoch:3d}: loss = {loss.data:.6f}")

    print("\nPredictions:")
    for x_row, y_true in zip(xs, ys):
        x_val = [Value(x) for x in x_row]
        pred = model(x_val)
        print(f"  {x_row} -> {pred.data:.4f} (target: {y_true})")

# =============================================================================
# Step 4: Gradient Checking
# =============================================================================

def numerical_gradient(f, params, eps=1e-5):
    """Compute numerical gradients using finite differences."""
    grads = []
    for p in params:
        old = p.data
        p.data = old + eps
        loss_plus = f().data
        p.data = old - eps
        loss_minus = f().data
        p.data = old
        grads.append((loss_plus - loss_minus) / (2 * eps))
    return grads

def check_gradients(model, xs, ys):
    """Verify analytic gradients against numerical gradients."""
    def compute_loss():
        inputs = [[Value(x) for x in row] for row in xs]
        predictions = [model(x) for x in inputs]
        targets = [Value(y) for y in ys]
        return mse_loss(predictions, targets)

    loss = compute_loss()
    model.zero_grad()
    loss.backward()
    analytic = [p.grad for p in model.parameters()]
    numerical = numerical_gradient(compute_loss, model.parameters())

    max_diff = 0
    for a, n in zip(analytic, numerical):
        max_diff = max(max_diff, abs(a - n))

    print(f"\nMax gradient difference: {max_diff:.2e}")
    assert max_diff < 1e-4, "Gradient check failed!"
    print("Gradient check passed!")

if __name__ == "__main__":
    main()
    # Uncomment to verify gradients:
    # model = MLP(2, [4, 1])
    # xs = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]
    # ys = [0.0, 1.0, 1.0, 0.0]
    # check_gradients(model, xs, ys)
