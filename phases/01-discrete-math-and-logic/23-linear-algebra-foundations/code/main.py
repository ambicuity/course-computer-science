"""Linear algebra foundations: vectors, matrices, Gaussian elimination, eigenvalues."""
import math


def dot(u, v):
    """Dot product of two vectors."""
    return sum(ui * vi for ui, vi in zip(u, v))


def magnitude(v):
    """Euclidean norm of a vector."""
    return math.sqrt(dot(v, v))


def normalize(v):
    """Unit vector in the same direction."""
    m = magnitude(v)
    return [vi / m for vi in v]


def cross(u, v):
    """Cross product (3D vectors only)."""
    return [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]


def matmul(A, B):
    """Matrix multiplication A (m×n) × B (n×p) → C (m×p)."""
    m, n = len(A), len(A[0])
    n2, p = len(B), len(B[0])
    assert n == n2, f"Dimension mismatch: {m}×{n} times {n2}×{p}"
    C = [[0] * p for _ in range(m)]
    for i in range(m):
        for j in range(p):
            for k in range(n):
                C[i][j] += A[i][k] * B[k][j]
    return C


def transpose(A):
    """Matrix transpose."""
    return [list(row) for row in zip(*A)]


def det2x2(A):
    """Determinant of a 2×2 matrix."""
    return A[0][0] * A[1][1] - A[0][1] * A[1][0]


def gaussian_eliminate(A, b):
    """Solve Ax = b via Gaussian elimination with partial pivoting."""
    n = len(A)
    M = [row[:] + [bi] for row, bi in zip(A, b)]

    for col in range(n):
        max_row = max(range(col, n), key=lambda r: abs(M[r][col]))
        M[col], M[max_row] = M[max_row], M[col]

        for row in range(col + 1, n):
            factor = M[row][col] / M[col][col]
            for j in range(col, n + 1):
                M[row][j] -= factor * M[col][j]

    x = [0] * n
    for i in range(n - 1, -1, -1):
        x[i] = (M[i][n] - sum(M[i][j] * x[j] for j in range(i + 1, n))) / M[i][i]
    return x


def eigenvalues_2x2(A):
    """Eigenvalues of a 2×2 matrix via characteristic polynomial."""
    a, b = A[0][0], A[0][1]
    c, d = A[1][0], A[1][1]
    trace = a + d
    det = a * d - b * c
    disc = math.sqrt(max(0, trace**2 - 4 * det))
    return [(trace + disc) / 2, (trace - disc) / 2]


def power_iteration(A, iterations=100):
    """Find dominant eigenvector via power iteration."""
    n = len(A)
    b = [1.0] * n
    for _ in range(iterations):
        Ab = [sum(A[i][j] * b[j] for j in range(n)) for i in range(n)]
        norm = magnitude(Ab)
        b = [x / norm for x in Ab]
    eigenvalue = dot(b, [sum(A[i][j] * b[j] for j in range(n)) for i in range(n)])
    return eigenvalue, b


if __name__ == "__main__":
    # Vector operations
    u = [1, 0]
    v = [1, 1]
    print(f"dot({u}, {v}) = {dot(u, v)}")
    print(f"|{v}| = {magnitude(v):.4f}")
    print(f"normalize({v}) = [{normalize(v)[0]:.4f}, {normalize(v)[1]:.4f}]")

    cos_theta = dot(u, v) / (magnitude(u) * magnitude(v))
    print(f"angle between {u} and {v}: {math.degrees(math.acos(cos_theta)):.1f}°")

    # Matrix operations
    theta = math.pi / 4
    R = [[math.cos(theta), -math.sin(theta)],
         [math.sin(theta),  math.cos(theta)]]
    v_col = [[1], [0]]
    rotated = matmul(R, v_col)
    print(f"\nRotated [1,0] by 45°: [{rotated[0][0]:.4f}, {rotated[1][0]:.4f}]")

    # Gaussian elimination
    A = [[2, 1], [3, 4]]
    b = [5, 11]
    x = gaussian_eliminate(A, b)
    print(f"\nSolved Ax=b: x = [{x[0]:.4f}, {x[1]:.4f}]")

    # Eigenvalues
    A = [[2, 1], [0, 3]]
    lambdas = eigenvalues_2x2(A)
    print(f"\nEigenvalues of [[2,1],[0,3]]: {lambdas}")

    # Power iteration
    A = [[2, 1], [1, 3]]
    eigenvalue, eigenvector = power_iteration(A)
    print(f"Dominant eigenvalue: {eigenvalue:.4f}")
    print(f"Dominant eigenvector: [{eigenvector[0]:.4f}, {eigenvector[1]:.4f}]")
