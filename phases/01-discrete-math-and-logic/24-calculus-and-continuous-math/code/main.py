"""Calculus foundations: derivatives, integrals, gradient descent, chain rule."""
import math


def derivative(f, x, h=1e-8):
    """Numerical derivative using central difference."""
    return (f(x + h) - f(x - h)) / (2 * h)


def integral(f, a, b, n=10000):
    """Numerical integration using Simpson's rule."""
    h = (b - a) / n
    result = f(a) + f(b)
    for i in range(1, n):
        coeff = 4 if i % 2 == 1 else 2
        result += coeff * f(a + i * h)
    return result * h / 3


def gradient_descent(grad_f, x0, lr=0.01, iterations=1000, tol=1e-8):
    """Minimize f using gradient descent. grad_f returns gradient vector at x."""
    x = x0[:]
    for _ in range(iterations):
        g = grad_f(x)
        if math.sqrt(sum(gi**2 for gi in g)) < tol:
            break
        x = [xi - lr * gi for xi, gi in zip(x, g)]
    return x


def chain_rule_demo():
    """Demonstrate chain rule: f = sin(x²), compute df/dx."""
    x = 2.0
    # Forward
    u = x ** 2
    f = math.sin(u)
    # Backward (chain rule)
    df_du = math.cos(u)
    du_dx = 2 * x
    df_dx = df_du * du_dx

    # Numerical verification
    h = 1e-8
    numerical = (math.sin((x + h) ** 2) - math.sin((x - h) ** 2)) / (2 * h)

    print(f"f = sin({x}²) = {f:.6f}")
    print(f"Chain rule: df/dx = cos({u})·{2*x} = {df_dx:.6f}")
    print(f"Numerical:  df/dx = {numerical:.6f}")
    print(f"Match: {abs(df_dx - numerical) < 1e-6}")


if __name__ == "__main__":
    # Derivatives
    f = lambda x: x ** 3
    print(f"d/dx(x³) at x=2: {derivative(f, 2):.6f} (exact: {3*4})")

    # Integrals
    f = lambda x: x ** 2
    print(f"\n∫x² dx [0,3] = {integral(f, 0, 3):.6f} (exact: 9)")

    f = lambda x: math.sin(x)
    print(f"∫sin(x) dx [0,π] = {integral(f, 0, math.pi):.6f} (exact: 2)")

    # Gradient descent
    print("\n--- Gradient Descent ---")
    grad_f = lambda x: [2 * (x[0] - 3), 2 * (x[1] - 5)]
    minimum = gradient_descent(grad_f, [0.0, 0.0], lr=0.1, iterations=100)
    print(f"Min of (x-3)²+(y-5)²: ({minimum[0]:.4f}, {minimum[1]:.4f})")

    # Chain rule
    print("\n--- Chain Rule (Backpropagation) ---")
    chain_rule_demo()
