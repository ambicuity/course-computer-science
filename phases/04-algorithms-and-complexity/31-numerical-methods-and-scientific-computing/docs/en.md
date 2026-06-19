# Numerical Methods and Scientific Computing

> Exact answers are a luxury. Most real problems — weather prediction, fluid dynamics, structural analysis — require approximate solutions computed with finite precision. Numerical methods are the algorithms that make those approximations reliable.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 04 Lessons 01-06, Phase 01 Lessons 23-24 (Linear Algebra, Calculus)
**Time:** ~90 minutes

## Learning Objectives

- Explain floating-point representation (IEEE 754) and why 0.1 + 0.2 ≠ 0.3.
- Implement root-finding algorithms (bisection, Newton's method) and analyze convergence.
- Solve ODEs numerically using Euler and Runge-Kutta methods.
- Apply numerical stability principles to avoid catastrophic cancellation and overflow.
- Connect numerical methods to CS applications: physics simulation, optimization, signal processing.

## The Problem

Computers represent real numbers as floating-point — 64 bits with a sign bit, 11-bit exponent, and 52-bit mantissa. This introduces rounding errors that compound in long computations. Without understanding numerical methods, you can't:

- Write a physics engine that doesn't explode (unstable ODE integration)
- Implement a financial calculator that gives correct results (catastrophic cancellation)
- Build a signal processor that doesn't alias (discrete Fourier transform)
- Debug why your optimization diverges (ill-conditioned matrices)

This lesson builds the numerical computing foundations that physics simulations, scientific computing, and any system dealing with continuous mathematics depend on.

## The Concept

### Floating-Point (IEEE 754)

```
64-bit double: [sign][11-bit exponent][52-bit mantissa]

value = (-1)^sign × 1.mantissa × 2^(exponent - 1023)

Example: 0.1 in binary
0.1 = 0.0001100110011... (repeating)
Cannot be represented exactly!
0.1 + 0.2 = 0.30000000000000004 (not 0.3)
```

**Machine epsilon:** ε ≈ 2.22 × 10⁻¹⁶ — the smallest number where 1 + ε > 1 in double precision.

**Catastrophic cancellation:** subtracting nearly-equal numbers loses significant digits.

```
f(x) = √(x+1) - √x

At x = 10¹⁵:
  Naive: √(10¹⁵+1) - √(10¹⁵) = 0  (wrong!)
  Algebraic: 1 / (√(x+1) + √x) = 1.58e-8  (correct)
```

### Root Finding

**Bisection method:** bracket the root, halve the interval each step. Guaranteed convergence but linear (1 bit per step).

```
f(x) = x³ - 2x - 5    f(2) = -1, f(3) = 16

Step 1: mid = 2.5, f(2.5) = 5.625 > 0 → root in [2, 2.5]
Step 2: mid = 2.25, f(2.25) = 1.89 > 0 → root in [2, 2.25]
...converges to x ≈ 2.0946
```

**Newton's method:** use tangent line to jump to next guess. Quadratic convergence but requires derivative and can diverge.

```
x_{n+1} = x_n - f(x_n) / f'(x_n)

f(x) = x³ - 2x - 5, f'(x) = 3x² - 2

x₀ = 2: x₁ = 2 - (-1)/(10) = 2.1
x₁ = 2.1: x₂ = 2.1 - (0.061)/(11.23) = 2.0946
...converges in 3 steps
```

### ODE Integration

**Euler method:** simplest ODE solver. Uses tangent line to step forward.

```
dy/dt = f(t, y)
y_{n+1} = y_n + h · f(t_n, y_n)

Example: dy/dt = -2y, y(0) = 1
Exact: y(t) = e^(-2t)

h=0.1: y(1) ≈ 0.1074 (exact: 0.1353, error: 21%)
h=0.01: y(1) ≈ 0.1340 (error: 1%)
```

**Runge-Kutta (RK4):** four evaluations per step, fourth-order accuracy.

```
k₁ = h·f(t, y)
k₂ = h·f(t + h/2, y + k₁/2)
k₃ = h·f(t + h/2, y + k₂/2)
k₄ = h·f(t + h, y + k₃)
y_{n+1} = y_n + (k₁ + 2k₂ + 2k₃ + k₄)/6
```

RK4 with h=0.1 gives error < 0.001% — 1000× better than Euler with same step size.

### Numerical Stability

**Stable vs unstable:** a method is stable if small perturbations in input produce small perturbations in output.

```
Unstable: y_{n+1} = y_n + h·(-1000·y_n)
  If h > 0.002, the solution explodes!

Stable: implicit Euler: y_{n+1} = y_n / (1 + 1000h)
  Stable for any h > 0
```

**Condition number:** how sensitive the output is to input perturbations. κ(A) = ‖A‖·‖A⁻¹‖. If κ ≈ 10¹⁵, you lose all digits to rounding.

### Connection to CS

| CS Application | Numerical Methods Used |
|----------------|------------------------|
| Physics engines | ODE integration (Verlet, RK4), collision detection |
| Machine Learning | Gradient descent, Newton's method, line search |
| Computer Graphics | Ray tracing (ray-sphere intersection), interpolation |
| Signal Processing | FFT (numerical Fourier transform), filtering |
| Finance | Monte Carlo simulation, option pricing |
| Climate/Weather | PDE solvers (finite difference, finite element) |

## Build It

### Step 1: IEEE 754 Inspection

```python
import struct

def float_bits(f):
    """Show IEEE 754 representation of a float."""
    bits = struct.unpack('Q', struct.pack('d', f))[0]
    sign = (bits >> 63) & 1
    exponent = ((bits >> 52) & 0x7FF) - 1023
    mantissa = bits & 0xFFFFFFFFFFFFF
    return sign, exponent, mantissa

print(f"0.1: {float_bits(0.1)}")
print(f"0.1 + 0.2 == 0.3: {0.1 + 0.2 == 0.3}")  # False!
print(f"0.1 + 0.2 = {0.1 + 0.2:.20f}")
```

### Step 2: Newton's Method

```python
def newton(f, df, x0, tol=1e-12, max_iter=100):
    """Find root of f(x)=0 using Newton's method."""
    x = x0
    for i in range(max_iter):
        fx = f(x)
        if abs(fx) < tol:
            return x, i
        x = x - fx / df(x)
    return x, max_iter

# x³ - 2x - 5 = 0
f = lambda x: x**3 - 2*x - 5
df = lambda x: 3*x**2 - 2
root, steps = newton(f, df, 2.0)
print(f"Root: {root:.12f} in {steps} steps")  # 2.094551481698
```

### Step 3: Runge-Kutta (RK4)

```python
def rk4(f, y0, t0, tf, h):
    """Solve dy/dt = f(t,y) using RK4."""
    t, y = t0, y0
    results = [(t, y)]
    while t < tf - 1e-12:
        k1 = h * f(t, y)
        k2 = h * f(t + h/2, y + k1/2)
        k3 = h * f(t + h/2, y + k2/2)
        k4 = h * f(t + h, y + k3)
        y += (k1 + 2*k2 + 2*k3 + k4) / 6
        t += h
        results.append((t, y))
    return results

# dy/dt = -2y, y(0) = 1 → exact y(t) = e^(-2t)
import math
f = lambda t, y: -2 * y
results = rk4(f, 1.0, 0, 1, 0.1)
numerical = results[-1][1]
exact = math.exp(-2)
print(f"RK4:    y(1) = {numerical:.6f}")
print(f"Exact:  y(1) = {exact:.6f}")
print(f"Error:  {abs(numerical - exact):.2e}")
```

### Step 4: Catastrophic Cancellation

```python
def bad_sqrt_diff(x):
    """Naive: √(x+1) - √x — catastrophic cancellation for large x."""
    return math.sqrt(x + 1) - math.sqrt(x)

def good_sqrt_diff(x):
    """Algebraically equivalent but numerically stable."""
    return 1 / (math.sqrt(x + 1) + math.sqrt(x))

x = 1e15
print(f"Naive:  {bad_sqrt_diff(x)}")  # 0.0 (wrong!)
print(f"Stable: {good_sqrt_diff(x)}")  # 1.58e-8 (correct)
```

## Use It

Production numerical libraries:
- **NumPy/SciPy** — Python numerical computing (wraps LAPACK, BLAS)
- **Intel MKL** — optimized math kernels for Intel CPUs
- **GSL** — GNU Scientific Library (C)
- **Eigen** — C++ template library for linear algebra

Physics engines use symplectic integrators (Verlet, Leapfrog) that conserve energy — standard RK4 drifts over long simulations.

## Read the Source

- [Numerical Recipes](https://numerical.recipes/) — the classic reference
- [What Every Computer Scientist Should Know About Floating-Point Arithmetic](https://docs.oracle.com/cd/E19957-01/806-3568/ncg_goldberg.html)
- SciPy `scipy.integrate.solve_ivp` — production ODE solver

## Ship It

- `code/main.py`: IEEE 754 inspection, Newton's method, RK4 integrator, stability demo
- `outputs/README.md`: numerical methods cheat sheet

## Exercises

1. **Easy:** Implement bisection method and compare convergence speed with Newton's method on x³ - 2x - 5 = 0.
2. **Medium:** Implement the Verlet integrator for a simple harmonic oscillator and compare energy conservation with Euler and RK4.
3. **Hard:** Implement adaptive step-size RK45 (Runge-Kutta-Fehlberg) that automatically adjusts h to maintain a target error tolerance.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Floating-point | "Decimal numbers" | IEEE 754 binary representation with finite precision; 0.1 cannot be represented exactly |
| Machine epsilon | "Smallest precision" | Smallest ε where 1 + ε > 1 in floating-point; ≈ 2.22×10⁻¹⁶ for double |
| Catastrophic cancellation | "Subtraction error" | Subtracting nearly-equal numbers loses significant digits; rewrite algebraically to avoid |
| Condition number | "Sensitivity measure" | κ(A) = ‖A‖·‖A⁻¹‖; how much input errors amplify through computation |
| Symplectic integrator | "Energy-preserving" | ODE solver that preserves Hamiltonian structure; essential for long physics simulations |

## Further Reading

- [Numerical Recipes](https://numerical.recipes/) — comprehensive algorithms reference
- [What Every CS Should Know About Floating-Point](https://docs.oracle.com/cd/E19957-01/806-3568/ncg_goldberg.html) — Goldberg's classic paper
- [SciPy documentation](https://scipy.org/) — production numerical Python
