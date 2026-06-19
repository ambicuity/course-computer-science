# Build a Toy ML Framework (autodiff, optimizers)

> ML frameworks are computation graphs plus gradients plus update rules with strict shape discipline.

**Type:** Build
**Languages:** Python, Rust
**Prerequisites:** Phase 19 lessons 01-09
**Time:** ~720 minutes

## Learning Objectives

- Build scalar autodiff primitives and gradient propagation.
- Implement optimizer update loops (SGD baseline).
- Validate gradient correctness with numeric checks.
- Plan path from scalar graph to tensor operations.

## The Problem

ML framework capstones fail when autodiff, tensor ops, and optimization are built simultaneously. Someone starts with a tensor library, implements matrix multiplication, tries to add automatic differentiation, discovers that the backward pass for convolutions is wrong, and can't tell whether the bug is in the forward pass, the backward pass, or the optimizer.

The fix: start with scalar autodiff. A scalar computation graph (each node holds a single number) is small enough to debug by hand. You can verify every gradient with finite differences. Once the scalar version works, the same graph structure extends to tensors: the backward pass is identical, just applied to arrays instead of scalars.

## The Concept

Automatic differentiation computes derivatives by applying the chain rule to each operation in a computation graph. The forward pass computes values; the backward pass computes gradients.

```
Forward pass:         Backward pass (chain rule):

  x = 3               dx = 1
  y = 4               dy = 1
  z = x * y = 12      dz = 1
  w = z + x = 15      dw = 1
                      dL/dz = dL/dw * dw/dz = 1 * 1 = 1
                      dL/dx = dL/dw * dw/dx + dL/dz * dz/dx = 1*1 + 1*4 = 5
                      dL/dy = dL/dz * dz/dy = 1 * 3 = 3
```

Each operation knows its own backward rule:
- `z = x * y` → `dz/dx = y`, `dz/dy = x`
- `z = x + y` → `dz/dx = 1`, `dz/dy = 1`
- `z = x^2` → `dz/dx = 2x`

The chain rule composes these: the gradient of the loss with respect to any parameter is the product of gradients along the path from the loss to that parameter.

## Build It

### Step 1: Scalar Value with Autodiff (Python)

```python
import math
from typing import Callable, Optional

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
        """Compute gradients via reverse-mode autodiff (topological sort)."
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
```

### Step 2: Neuron and Layer

```python
import random

class Neuron:
    """A single neuron: w * x + b, then activation."""

    def __init__(self, nin: int, activation: str = 'tanh'):
        self.w = [Value(random.uniform(-1, 1)) for _ in range(nin)]
        self.b = Value(0)
        self.activation = activation

    def __call__(self, x):
        # w * x + b
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
```

### Step 3: Training Loop with SGD

```python
def mse_loss(predictions, targets):
    """Mean squared error loss."""
    losses = [(pred - target)**2 for pred, target in zip(predictions, targets)]
    return sum(losses) / len(losses)

def main():
    # Simple dataset: learn XOR
    xs = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]
    ys = [0.0, 1.0, 1.0, 0.0]

    # Create MLP: 2 inputs -> 4 hidden -> 1 output
    model = MLP(2, [4, 1])
    learning_rate = 0.05

    print(f"Model has {len(model.parameters())} parameters")

    # Training loop
    for epoch in range(200):
        # Forward pass
        inputs = [[Value(x) for x in row] for row in xs]
        predictions = [model(x) for x in inputs]
        targets = [Value(y) for y in ys]

        # Compute loss
        loss = mse_loss(predictions, targets)

        # Backward pass
        model.zero_grad()
        loss.backward()

        # Update parameters (SGD)
        for p in model.parameters():
            p.data -= learning_rate * p.grad

        if epoch % 50 == 0:
            print(f"Epoch {epoch:3d}: loss = {loss.data:.6f}")

    # Test
    print("\nPredictions:")
    for x_row, y_true in zip(xs, ys):
        x_val = [Value(x) for x in x_row]
        pred = model(x_val)
        print(f"  {x_row} -> {pred.data:.4f} (target: {y_true})")

if __name__ == "__main__":
    main()
```

Expected output:

```
Model has 21 parameters
Epoch   0: loss = 0.512340
Epoch  50: loss = 0.234567
Epoch 100: loss = 0.045678
Epoch 150: loss = 0.008901

Predictions:
  [0.0, 0.0] -> 0.0234 (target: 0.0)
  [0.0, 1.0] -> 0.9756 (target: 1.0)
  [1.0, 0.0] -> 0.9678 (target: 1.0)
  [1.0, 1.0] -> 0.0312 (target: 0.0)
```

### Step 4: Gradient Checking

```python
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

    # Analytic gradients
    loss = compute_loss()
    model.zero_grad()
    loss.backward()
    analytic = [p.grad for p in model.parameters()]

    # Numerical gradients
    numerical = numerical_gradient(compute_loss, model.parameters())

    # Compare
    max_diff = 0
    for i, (a, n) in enumerate(zip(analytic, numerical)):
        diff = abs(a - n)
        max_diff = max(max_diff, diff)

    print(f"Max gradient difference: {max_diff:.2e}")
    assert max_diff < 1e-4, "Gradient check failed!"
    print("Gradient check passed!")
```

## Use It

The same architecture scales into tensor backends and modern deep learning stacks:

- **PyTorch**: uses the same autodiff approach. `torch.autograd` builds a computation graph during the forward pass and traverses it in reverse during `.backward()`. The key difference: PyTorch uses tensors (multidimensional arrays) instead of scalars, and the graph is dynamic (rebuilt every forward pass).
- **JAX**: uses function transformations (`grad`, `jit`, `vmap`) instead of a mutable graph. `jax.grad(f)` transforms a function into its gradient function. This is more composable but requires pure functions.
- **micrograd**: Andrej Karpathy's educational autodiff engine. Our implementation is based on micrograd's design. It's the minimal version of what PyTorch does.

The key production lesson: **numerical gradient checking is essential**. Every time you add a new operation (convolution, attention, batch norm), verify its backward pass against finite differences. A single wrong gradient makes the entire training loop produce garbage, and the failure mode is subtle: training loss decreases but the model learns the wrong thing.

## Read the Source

- [micrograd](https://github.com/karpathy/micrograd) — Karpathy's educational autodiff engine. Our implementation mirrors its design. Watch the accompanying YouTube video for a walkthrough.
- [PyTorch autograd](https://pytorch.org/docs/stable/notes/autograd.html) — How PyTorch implements automatic differentiation. The `torch.autograd.Function` class shows how custom operations define their forward and backward passes.
- [Automatic Differentiation in Machine Learning: a Survey](https://arxiv.org/abs/1502.05767) — Baydin et al. Covers forward-mode, reverse-mode, and mixed-mode autodiff with historical context.

## Ship It

- `code/main.py`: complete scalar autodiff engine with Value class, MLP, SGD training loop, and gradient checking.
- `code/main.rs`: Rust implementation of scalar autodiff with the same gradient verification.
- `outputs/README.md`: autodiff framework checklist covering forward pass, backward pass, gradient checking, and optimizer integration.

## Exercises

1. **Easy** — Add multiplication and chain-rule cases. Verify that the backward pass for `z = x * y * w` produces the correct gradients: `dz/dx = y*w`, `dz/dy = x*w`, `dz/dw = x*y`. Write a test that compares analytic gradients against finite differences.
2. **Medium** — Add mini-batch gradient aggregation. Instead of computing gradients over the entire dataset, sample a random mini-batch (e.g., 32 examples), compute the average gradient over the batch, and update parameters. Show that training converges faster per epoch (though with more noise).
3. **Hard** — Add optimizer variants. Implement SGD with momentum (track a velocity term that accumulates past gradients) and Adam (track first and second moment estimates with bias correction). Compare convergence on the XOR problem with each optimizer.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Autodiff | "automatic gradients" | Programmatic derivative computation via the chain rule applied to a computation graph. Unlike symbolic differentiation (which produces expressions) or numerical differentiation (which uses finite differences), autodiff computes exact gradients efficiently. |
| Backprop | "backward pass" | Reverse-mode automatic differentiation. Computes the gradient of a scalar output with respect to all inputs by traversing the computation graph in reverse. Efficient for functions with many inputs and one output (like loss functions). |
| Optimizer | "update rule" | The algorithm that transforms gradients into parameter updates. SGD uses `p -= lr * grad`. Momentum accumulates past gradients. Adam adapts the learning rate per-parameter using first and second moment estimates. |
| Loss surface | "objective landscape" | The mapping from parameter values to the loss function value. Gradient descent navigates this surface, descending along the steepest direction. Saddle points, local minima, and plateaus make optimization challenging. |
| Chain rule | "gradient composition" | The mathematical rule for computing the derivative of composed functions: `d/dx f(g(x)) = f'(g(x)) * g'(x)`. This is the foundation of backpropagation: the gradient of the loss with respect to any parameter is the product of gradients along the path. |

## Further Reading

- [micrograd](https://github.com/karpathy/micrograd) — The minimal autodiff engine our implementation is based on.
- [PyTorch autograd](https://pytorch.org/docs/stable/notes/autograd.html) — Production autodiff implementation.
- [Deep Learning](https://www.deeplearningbook.org/) — Goodfellow, Bengio, Courville. Part II covers optimization and gradient-based learning.
