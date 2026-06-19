# Linear Algebra Foundations

> Vectors and matrices are the language of graphics, ML, and physics simulations. Without them, you can't rotate a triangle, train a neural network, or solve a system of equations.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 01 Lessons 01-04
**Time:** ~90 minutes

## Learning Objectives

- Perform vector arithmetic (add, scale, dot product, cross product) and explain geometric meaning.
- Multiply matrices, compute determinants, and find inverses — by hand and in code.
- Solve systems of linear equations using Gaussian elimination and LU decomposition.
- Explain eigenvalues and eigenvectors intuitively and compute them for small matrices.
- Connect linear algebra to CS applications: transformations (graphics), PCA (ML), PageRank (networks).

## The Problem

Every time you rotate a 3D model, train a neural network, or compute PageRank, you're doing linear algebra. Graphics pipelines multiply 4×4 matrices for every vertex. Backpropagation is matrix calculus. Google's original PageRank is an eigenvector computation.

Without linear algebra, you can't:
- Understand how GPUs achieve parallelism (they're matrix multiplication engines)
- Debug a model that won't converge (eigenvalue decomposition reveals conditioning)
- Implement a physics engine (forces are vectors, rotations are matrices)
- Comprehend modern ML papers (everything is tensors and matrix operations)

This lesson builds the mathematical machinery that Phase 14 (Graphics), Phase 19 (ML Framework capstone), and any physics simulation depend on.

## The Concept

### Vectors

A vector is an ordered list of numbers. Geometrically, it's an arrow from the origin.

```
v = [3, 4]     magnitude = √(3² + 4²) = 5

        ↑ y
        |     • (3,4)
        |    /
        |   /  v = [3,4]
        |  /
        | /
        +------→ x
```

**Vector operations:**

| Operation | Formula | Geometric Meaning |
|-----------|---------|-------------------|
| Addition | [a,b] + [c,d] = [a+c, b+d] | Translate by another vector |
| Scalar multiply | k·[a,b] = [ka, kb] | Scale length, keep direction |
| Dot product | [a,b]·[c,d] = ac + bd | Projection; cos θ = (u·v)/(\|u\|\|v\|) |
| Cross product (3D) | u × v = [u₂v₃-u₃v₂, ...] | Perpendicular vector; area of parallelogram |

The dot product is the most important: if u·v = 0, the vectors are orthogonal. This is how you check if two directions are perpendicular.

### Matrices

A matrix is a 2D array of numbers. Matrices represent linear transformations — functions that preserve addition and scalar multiplication.

```
Rotation by θ:          Scaling by 2:
[cos θ  -sin θ]         [2  0]
[sin θ   cos θ]         [0  2]
```

**Matrix multiplication:**

```
A (m×n) × B (n×p) = C (m×p)

C[i][j] = Σ_k A[i][k] * B[k][j]
```

Matrix multiplication is NOT commutative (A×B ≠ B×A in general). This matters: rotating then translating gives a different result than translating then rotating.

**Determinant:** det(A) tells you the scaling factor of the transformation. det = 0 means the transformation collapses space (singular matrix, no inverse).

```
det([a b; c d]) = ad - bc

det([2 0; 0 3]) = 6  (area scales by 6)
det([1 1; 1 1]) = 0  (collapses to a line)
```

**Inverse:** A⁻¹ exists iff det(A) ≠ 0. A⁻¹A = I (identity matrix). Solving Ax = b becomes x = A⁻¹b.

### Gaussian Elimination

To solve Ax = b, augment [A|b] and row-reduce:

```
[2  1 | 5]     R2 = R2 - (3/2)R1    [2  1 | 5]
[3  4 | 11]    ──────────────────→   [0  5/2 | 1/2]

Back-substitute: 5/2 · x₂ = 1/2  →  x₂ = 1/5
                 2x₁ + 1/5 = 5   →  x₁ = 12/5
```

This is O(n³) for an n×n system. LU decomposition factors A = LU (lower × upper triangular) so you can solve multiple right-hand sides efficiently.

### Eigenvalues and Eigenvectors

An eigenvector v of matrix A satisfies Av = λv — the transformation only scales v, doesn't change its direction. λ is the eigenvalue.

```
A = [2 1]    eigenvectors: [1,1] with λ=3
    [0 3]                   [1,0] with λ=2

A·[1,1] = [3,3] = 3·[1,1]  ✓
A·[1,0] = [2,0] = 2·[1,0]  ✓
```

**Computing eigenvalues:** solve det(A - λI) = 0.

For 2×2: det([a-λ, b; c, d-λ]) = (a-λ)(d-λ) - bc = 0 → quadratic in λ.

Eigenvalues reveal:
- **Stability:** if all |λ| < 1, repeated application of A converges to zero
- **PageRank:** the dominant eigenvector of the web graph's transition matrix
- **PCA:** eigenvectors of the covariance matrix are the principal components
- **Vibration:** eigenvalues are natural frequencies of a mechanical system

### Connection to CS

| CS Application | Linear Algebra Used |
|----------------|---------------------|
| 3D Graphics | 4×4 matrices for projection, rotation, translation |
| Machine Learning | Weight matrices, gradient computation, PCA |
| PageRank | Dominant eigenvector of web graph |
| Physics Simulation | Force vectors, rotation matrices, inertia tensors |
| Cryptography | Matrix-based ciphers (Hill cipher) |
| Compression | SVD (singular value decomposition) for image/video |

## Build It

### Step 1: Vector Operations

```python
import math

def dot(u, v):
    return sum(ui * vi for ui, vi in zip(u, v))

def magnitude(v):
    return math.sqrt(dot(v, v))

def normalize(v):
    m = magnitude(v)
    return [vi / m for vi in v]

def cross(u, v):
    return [
        u[1]*v[2] - u[2]*v[1],
        u[2]*v[0] - u[0]*v[2],
        u[0]*v[1] - u[1]*v[0]
    ]

# Angle between vectors
u = [1, 0]
v = [1, 1]
cos_theta = dot(u, v) / (magnitude(u) * magnitude(v))
angle = math.acos(cos_theta)
print(f"Angle: {math.degrees(angle):.1f}°")  # 45.0°
```

### Step 2: Matrix Multiplication

```python
def matmul(A, B):
    m, n = len(A), len(A[0])
    n2, p = len(B), len(B[0])
    assert n == n2
    C = [[0]*p for _ in range(m)]
    for i in range(m):
        for j in range(p):
            for k in range(n):
                C[i][j] += A[i][k] * B[k][j]
    return C

# Rotation by 45 degrees
theta = math.pi / 4
R = [[math.cos(theta), -math.sin(theta)],
     [math.sin(theta),  math.cos(theta)]]

v = [[1], [0]]  # column vector
rotated = matmul(R, v)
print(f"Rotated: [{rotated[0][0]:.3f}, {rotated[1][0]:.3f}]")
# [0.707, 0.707]
```

### Step 3: Gaussian Elimination

```python
def gaussian_eliminate(A, b):
    n = len(A)
    # Augment
    M = [row[:] + [bi] for row, bi in zip(A, b)]

    # Forward elimination
    for col in range(n):
        # Partial pivoting
        max_row = max(range(col, n), key=lambda r: abs(M[r][col]))
        M[col], M[max_row] = M[max_row], M[col]

        for row in range(col + 1, n):
            factor = M[row][col] / M[col][col]
            for j in range(col, n + 1):
                M[row][j] -= factor * M[col][j]

    # Back substitution
    x = [0] * n
    for i in range(n - 1, -1, -1):
        x[i] = (M[i][n] - sum(M[i][j] * x[j] for j in range(i+1, n))) / M[i][i]
    return x

A = [[2, 1], [3, 4]]
b = [5, 11]
x = gaussian_eliminate(A, b)
print(f"Solution: x = {x}")  # [2.4, 0.2]  (12/5, 1/5)
```

### Step 4: Eigenvalues (2×2)

```python
def eigenvalues_2x2(A):
    a, b = A[0][0], A[0][1]
    c, d = A[1][0], A[1][1]
    # Characteristic polynomial: λ² - (a+d)λ + (ad-bc) = 0
    trace = a + d
    det = a * d - b * c
    disc = math.sqrt(trace**2 - 4 * det)
    return [(trace + disc) / 2, (trace - disc) / 2]

A = [[2, 1], [0, 3]]
lambdas = eigenvalues_2x2(A)
print(f"Eigenvalues: {lambdas}")  # [3.0, 2.0]
```

## Use It

Production linear algebra uses optimized BLAS (Basic Linear Algebra Subprograms) libraries:
- **OpenBLAS** — open-source, used by NumPy/SciPy
- **Intel MKL** — optimized for Intel CPUs
- **cuBLAS** — GPU-accelerated (NVIDIA)

NumPy's `np.linalg.solve()` calls LAPACK's `dgesv` for general systems, `np.linalg.eig()` for eigenvalues. For large sparse matrices (PageRank, FEM), use iterative methods (Lanczos, Arnoldi) — not Gaussian elimination.

## Read the Source

- `numpy/linalg/linalg.py` — Python wrapper around LAPACK
- `scipy.sparse.linalg` — sparse matrix solvers (eigsh, gmres)
- 3Blue1Brown's "Essence of Linear Algebra" video series — best visual intuition

## Ship It

- `code/main.py`: vector/matrix operations, Gaussian elimination, eigenvalue solver
- `outputs/README.md`: linear algebra cheat sheet

## Exercises

1. **Easy:** Implement matrix transpose and verify (AB)ᵀ = BᵀAᵀ.
2. **Medium:** Implement LU decomposition and solve Ax = b using forward/back substitution.
3. **Hard:** Implement power iteration to find the dominant eigenvector of a 3×3 matrix. Apply it to a toy PageRank problem.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Vector | "A list of numbers" | An element of a vector space; geometrically an arrow with magnitude and direction |
| Dot product | "Multiply and add" | Projection of one vector onto another; u·v = \|u\|\|v\|cos θ |
| Determinant | "A number from a matrix" | Signed scaling factor of the linear transformation; 0 means singular |
| Eigenvector | "Special vector" | Direction preserved by the transformation (only scaled, not rotated) |
| Singular | "Can't invert" | det = 0; transformation collapses dimension; no unique solution exists |

## Further Reading

- [3Blue1Brown: Essence of Linear Algebra](https://www.3blue1brown.com/topics/linear-algebra) — visual intuition
- [MIT OCW 18.06](https://ocw.mit.edu/courses/18-06-linear-algebra-spring-2010/) — Gilbert Strang's legendary course
- [Linear Algebra Done Right](https://linear.axler.net/) — Sheldon Axler, proof-based
