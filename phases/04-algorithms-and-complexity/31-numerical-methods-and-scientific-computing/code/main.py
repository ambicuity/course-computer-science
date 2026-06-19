"""Numerical methods: floating-point, root finding, ODE integration, stability."""
import math
import struct


def float_bits(f):
    """Show IEEE 754 representation of a float."""
    bits = struct.unpack('Q', struct.pack('d', f))[0]
    sign = (bits >> 63) & 1
    exponent = ((bits >> 52) & 0x7FF) - 1023
    mantissa = bits & 0xFFFFFFFFFFFFF
    return sign, exponent, mantissa


def bisection(f, a, b, tol=1e-12, max_iter=100):
    """Find root in [a,b] where f(a) and f(b) have opposite signs."""
    assert f(a) * f(b) < 0, "f(a) and f(b) must have opposite signs"
    for i in range(max_iter):
        mid = (a + b) / 2
        if abs(f(mid)) < tol or (b - a) / 2 < tol:
            return mid, i
        if f(mid) * f(a) < 0:
            b = mid
        else:
            a = mid
    return (a + b) / 2, max_iter


def newton(f, df, x0, tol=1e-12, max_iter=100):
    """Find root of f(x)=0 using Newton's method."""
    x = x0
    for i in range(max_iter):
        fx = f(x)
        if abs(fx) < tol:
            return x, i
        x = x - fx / df(x)
    return x, max_iter


def euler(f, y0, t0, tf, h):
    """Solve dy/dt = f(t,y) using Euler's method."""
    t, y = t0, y0
    results = [(t, y)]
    while t < tf - 1e-12:
        y += h * f(t, y)
        t += h
        results.append((t, y))
    return results


def rk4(f, y0, t0, tf, h):
    """Solve dy/dt = f(t,y) using Runge-Kutta 4th order."""
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


def verlet(f, x0, v0, t0, tf, h):
    """Symplectic Verlet integrator for d²x/dt² = f(t, x)."""
    t, x, v = t0, x0, v0
    results = [(t, x, v)]
    a = f(t, x)
    while t < tf - 1e-12:
        x_new = x + v * h + 0.5 * a * h * h
        a_new = f(t + h, x_new)
        v_new = v + 0.5 * (a + a_new) * h
        t += h
        x, v, a = x_new, v_new, a_new
        results.append((t, x, v))
    return results


if __name__ == "__main__":
    # Floating-point precision
    print("=== Floating-Point ===")
    print(f"0.1 + 0.2 == 0.3: {0.1 + 0.2 == 0.3}")
    print(f"0.1 + 0.2 = {0.1 + 0.2:.20f}")
    print(f"Machine epsilon: {2.220446049250313e-16}")

    # Root finding
    print("\n=== Root Finding: x³ - 2x - 5 = 0 ===")
    f = lambda x: x**3 - 2*x - 5
    df = lambda x: 3*x**2 - 2

    root_bisect, steps_bisect = bisection(f, 2, 3)
    root_newton, steps_newton = newton(f, df, 2.0)
    print(f"Bisection: {root_bisect:.12f} in {steps_bisect} steps")
    print(f"Newton:    {root_newton:.12f} in {steps_newton} steps")

    # ODE integration
    print("\n=== ODE: dy/dt = -2y, y(0) = 1 ===")
    f_ode = lambda t, y: -2 * y
    exact = math.exp(-2)

    euler_result = euler(f_ode, 1.0, 0, 1, 0.1)
    rk4_result = rk4(f_ode, 1.0, 0, 1, 0.1)

    print(f"Euler:  y(1) = {euler_result[-1][1]:.6f} (error: {abs(euler_result[-1][1] - exact):.2e})")
    print(f"RK4:    y(1) = {rk4_result[-1][1]:.6f} (error: {abs(rk4_result[-1][1] - exact):.2e})")
    print(f"Exact:  y(1) = {exact:.6f}")

    # Catastrophic cancellation
    print("\n=== Catastrophic Cancellation ===")
    x = 1e15
    bad = math.sqrt(x + 1) - math.sqrt(x)
    good = 1 / (math.sqrt(x + 1) + math.sqrt(x))
    print(f"Naive  √(x+1) - √x = {bad}")
    print(f"Stable 1/(√(x+1)+√x) = {good:.15e}")
