# Calculus and Continuous Math

> Discrete math gives you algorithms. Calculus gives you optimization, physics, and the ability to understand why gradient descent works.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 01 Lessons 01-04, Lesson 23 (Linear Algebra)
**Time:** ~90 minutes

## Learning Objectives

- Compute derivatives using the power, chain, and product rules.
- Explain the geometric meaning of derivatives (slope) and integrals (area).
- Apply gradient descent to minimize a function and understand why learning rate matters.
- Compute partial derivatives and gradients for multivariable functions.
- Connect calculus to CS: optimization (ML), physics simulation, signal processing.

## The Problem

Machine learning is calculus. Training a neural network means computing gradients of a loss function with respect to millions of weights, then updating those weights in the direction that reduces loss. Without calculus, you can't:

- Understand why backpropagation works (it's the chain rule applied recursively)
- Debug a model that diverges (gradient explosion — derivatives growing without bound)
- Implement physics simulations (velocity is the derivative of position; acceleration is the derivative of velocity)
- Optimize anything continuously (resource allocation, signal processing, control systems)

This lesson builds the calculus machinery that Phase 19's ML Framework capstone and any optimization problem depend on.

## The Concept

### Derivatives

The derivative of f(x) at point x is the instantaneous rate of change — the slope of the tangent line.

```
f(x) = x²

f'(x) = lim[h→0] (f(x+h) - f(x)) / h
      = lim[h→0] ((x+h)² - x²) / h
      = lim[h→0] (2xh + h²) / h
      = 2x

At x=3: f'(3) = 6  (slope of tangent line)
```

**Rules:**

| Rule | Formula | Example |
|------|---------|---------|
| Power | d/dx(xⁿ) = nxⁿ⁻¹ | d/dx(x³) = 3x² |
| Sum | d/dx(f+g) = f'+g' | d/dx(x²+x) = 2x+1 |
| Product | d/dx(fg) = f'g + fg' | d/dx(x·sin x) = sin x + x·cos x |
| Chain | d/dx(f(g(x))) = f'(g(x))·g'(x) | d/dx(sin(x²)) = cos(x²)·2x |

The chain rule is the foundation of backpropagation: the derivative of a composition of functions is the product of their individual derivatives.

### Integrals

The integral of f(x) from a to b is the signed area under the curve.

```
∫[a,b] f(x) dx = F(b) - F(a)   where F' = f

∫[0,3] x² dx = [x³/3]₀³ = 27/3 - 0 = 9
```

Integrals accumulate: distance is the integral of velocity; work is the integral of force over displacement.

### Gradients

For multivariable functions f(x, y, z, ...), the gradient ∇f is the vector of partial derivatives:

```
∇f = [∂f/∂x, ∂f/∂y, ∂f/∂z]

f(x,y) = x² + y²
∇f = [2x, 2y]

At (3,4): ∇f = [6, 8]
```

The gradient points in the direction of steepest ascent. Its magnitude tells you how steep. To minimize f, move in the direction of -∇f.

### Gradient Descent

```
x_new = x_old - learning_rate × ∇f(x_old)
```

Repeat until convergence. The learning rate (η) controls step size:
- Too large: overshoots, diverges
- Too small: converges painfully slowly
- Just right: approaches minimum efficiently

```
f(x) = x⁴ - 3x³ + 2     f'(x) = 4x³ - 9x²

Start at x=3, η=0.01:
x=3.00  f=2.00   f'=-27.00  → x=3.27
x=3.27  f=30.87  f'=-42.67  → x=3.70
...converges to local minimum near x=2.25
```

### Connection to CS

| CS Application | Calculus Used |
|----------------|---------------|
| Neural network training | Gradient descent, chain rule (backpropagation) |
| Physics simulation | Derivatives for velocity/acceleration, integrals for work |
| Signal processing | Fourier transform (integrals), convolution |
| Optimization | Lagrange multipliers, KKT conditions |
| Graphics | Parametric curves (derivatives give tangent vectors) |
| Control theory | Differential equations, PID controllers |

## Build It

### Step 1: Numerical Derivative

```python
def derivative(f, x, h=1e-8):
    """Numerical derivative using central difference."""
    return (f(x + h) - f(x - h)) / (2 * h)

# f(x) = x³, f'(x) = 3x²
f = lambda x: x**3
print(f"f'(2) = {derivative(f, 2):.6f}")  # 12.000000
print(f"Exact: {3 * 2**2}")                # 12
```

### Step 2: Numerical Integral

```python
def integrate(f, a, b, n=10000):
    """Numerical integration using Simpson's rule."""
    h = (b - a) / n
    result = f(a) + f(b)
    for i in range(1, n):
        coeff = 4 if i % 2 == 1 else 2
        result += coeff * f(a + i * h)
    return result * h / 3

# ∫[0,3] x² dx = 9
f = lambda x: x**2
print(f"∫x² dx [0,3] = {integrate(f, 0, 3):.6f}")  # 9.000000
```

### Step 3: Gradient Descent

```python
def gradient_descent(grad_f, x0, lr=0.01, iterations=1000):
    """Minimize f using gradient descent. grad_f returns gradient at x."""
    x = x0[:]
    history = [x[:]]
    for _ in range(iterations):
        g = grad_f(x)
        x = [xi - lr * gi for xi, gi in zip(x, g)]
        history.append(x[:])
    return x, history

# Minimize f(x,y) = (x-3)² + (y-5)²  →  minimum at (3,5)
grad_f = lambda x: [2*(x[0]-3), 2*(x[1]-5)]
minimum, history = gradient_descent(grad_f, [0.0, 0.0], lr=0.1, iterations=50)
print(f"Minimum found at ({minimum[0]:.4f}, {minimum[1]:.4f})")  # (3.0, 5.0)
```

### Step 4: Chain Rule (Backpropagation)

```python
def backprop_example():
    """Simple backprop: f = sin(x²), compute df/dx using chain rule."""
    import math

    x = 2.0
    # Forward pass
    x_squared = x ** 2          # u = x²
    f = math.sin(x_squared)     # f = sin(u)

    # Backward pass (chain rule)
    df_du = math.cos(x_squared) # df/du = cos(u)
    du_dx = 2 * x               # du/dx = 2x
    df_dx = df_du * du_dx       # df/dx = cos(x²) · 2x

    print(f"f = sin({x}²) = {f:.4f}")
    print(f"df/dx = cos({x}²)·{2*x} = {df_dx:.4f}")

    # Verify numerically
    h = 1e-8
    numerical = (math.sin((x+h)**2) - math.sin((x-h)**2)) / (2*h)
    print(f"Numerical: {numerical:.4f}")

backprop_example()
```

## Use It

Every ML framework (PyTorch, TensorFlow, JAX) implements automatic differentiation — the chain rule applied to computational graphs. When you call `loss.backward()`, PyTorch traverses the computation graph in reverse, multiplying local gradients (chain rule) to compute ∂loss/∂weight for every parameter.

Gradient descent variants:
- **SGD** — stochastic (minibatch) gradient descent
- **Adam** — adaptive learning rate with momentum
- **L-BFGS** — quasi-Newton method for smooth problems

## Read the Source

- PyTorch `torch.autograd` — automatic differentiation engine
- [3Blue1Brown: Essence of Calculus](https://www.3blue1brown.com/topics/calculus) — visual intuition
- [MIT OCW 18.01](https://ocw.mit.edu/courses/18-01sc-single-variable-calculus-fall-2010/) — single variable calculus

## Ship It

- `code/main.py`: numerical derivatives, integrals, gradient descent, chain rule demo
- `outputs/README.md`: calculus cheat sheet for ML

## Exercises

1. **Easy:** Compute the derivative of f(x) = x⁴ - 2x² + 1 at x=1 numerically and verify analytically.
2. **Medium:** Implement gradient descent for f(x,y) = (x-1)² + 10(y-2)². Why does it converge slowly along the y-axis? (Hint: condition number.)
3. **Hard:** Implement a simple neural network with one hidden layer and train it using backpropagation (chain rule) on the XOR problem.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Derivative | "Rate of change" | Instantaneous slope of the tangent line; lim[f(x+h)-f(x)]/h as h→0 |
| Integral | "Area under the curve" | Accumulation of f over [a,b]; antiderivative evaluated at endpoints |
| Gradient | "Direction of steepest ascent" | Vector of partial derivatives [∂f/∂x₁, ∂f/∂x₂, ...] |
| Chain rule | "Derivative of composition" | d/dx(f(g(x))) = f'(g(x))·g'(x); the basis of backpropagation |
| Learning rate | "Step size" | Scalar η controlling how far to move against the gradient in each update |

## Further Reading

- [3Blue1Brown: Essence of Calculus](https://www.3blue1brown.com/topics/calculus) — best visual introduction
- [MIT OCW 18.01](https://ocw.mit.edu/courses/18-01sc-single-variable-calculus-fall-2010/) — full course
- [The Matrix Calculus You Need For Deep Learning](https://explained.ai/matrix-calculus/) — practical ML calculus
