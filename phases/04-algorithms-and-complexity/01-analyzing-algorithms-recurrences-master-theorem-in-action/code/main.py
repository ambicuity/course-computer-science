import math


def recursion_tree(a, b, f_n, n, depth=None, max_depth=None):
    """Generate and print an ASCII recursion tree for T(n) = aT(n/b) + f(n).

    Args:
        a: number of subproblems
        b: size reduction factor
        f_n: work function (callable, takes subproblem size)
        n: initial problem size
        depth: current depth (internal use)
        max_depth: maximum depth to display
    """
    if depth is None:
        depth = 0
    if max_depth is None:
        max_depth = int(math.log(n, b)) + 1 if n > 1 and b > 1 else 4
    if depth > max_depth or n < 1:
        return

    indent = "  " * depth
    work = f_n(n)
    print(f"{indent}T({n}) [work={work}]")

    if n > 1:
        subproblem_size = n // b
        if subproblem_size < 1:
            return
        for _ in range(a):
            recursion_tree(a, b, f_n, subproblem_size, depth + 1, max_depth)


def compute_tree_work(a, b, f_n, n):
    """Compute total work across all levels of the recursion tree.

    Returns a list of (level, work_per_level, num_subproblems, subproblem_size).
    """
    levels = []
    level = 0
    size = n
    while size >= 1:
        num_nodes = a ** level
        work = num_nodes * f_n(size)
        levels.append((level, work, num_nodes, size))
        size = size // b
        level += 1
    return levels


def master_theorem(a, b, f_descriptor):
    """Classify T(n) = aT(n/b) + f(n) using the Master Theorem.

    Args:
        a: number of subproblems
        b: size reduction factor
        f_descriptor: string describing f(n), e.g., "n", "1", "n^2", "n*log(n)"

    Returns:
        dict with case number, solution, and explanation.
    """
    log_b_a = math.log(a, b)

    case, solution, explanation = _classify(f_descriptor, log_b_a, a, b)

    return {
        "a": a,
        "b": b,
        "f_n": f_descriptor,
        "log_b_a": log_b_a,
        "case": case,
        "solution": solution,
        "explanation": explanation,
    }


def _classify(f_desc, log_b_a, a, b):
    """Determine which Master Theorem case applies based on f(n) descriptor."""
    exponent = _parse_exponent(f_desc)

    if exponent is not None:
        if exponent < log_b_a - 1e-9:
            return (
                1,
                f"Theta(n^{log_b_a:.4f})",
                f"f(n) = {f_desc} grows slower than n^{log_b_a:.4f} = n^log_{b}({a})",
            )
        elif abs(exponent - log_b_a) < 1e-9:
            log_power = _parse_log_power(f_desc)
            total_log = log_power + 1
            return (
                2,
                f"Theta(n^{log_b_a:.4f} * log^{total_log} n)",
                f"f(n) = {f_desc} matches n^{log_b_a:.4f} up to log factors",
            )
        else:
            return (
                3,
                f"Theta({f_desc})",
                f"f(n) = {f_desc} grows faster than n^{log_b_a:.4f}",
            )

    return (None, "Cannot classify", f"f(n) = {f_desc} does not fit standard forms")


def _parse_exponent(f_desc):
    """Extract the polynomial exponent from f(n) descriptor.

    Handles patterns like "n", "n^2", "n*log(n)", "n*log(n)^2", "1" (returns 0).
    """
    f_desc = f_desc.strip()

    if f_desc == "1":
        return 0.0

    if "log" in f_desc or "logn" in f_desc.replace(" ", ""):
        parts = f_desc.replace(" ", "").split("*")
        for part in parts:
            if "log" not in part:
                if part == "n":
                    return 1.0
                elif part.startswith("n^"):
                    try:
                        return float(part[2:])
                    except ValueError:
                        return None
                elif part == "n":
                    return 1.0
        if f_desc.replace(" ", "") in ("log(n)", "logn"):
            return 0.0
        if f_desc.replace(" ", "").startswith("n*log"):
            return 1.0

    if f_desc == "n":
        return 1.0
    if f_desc.startswith("n^"):
        try:
            return float(f_desc[2:])
        except ValueError:
            return None

    return None


def _parse_log_power(f_desc):
    """Extract the power of log n from the descriptor."""
    cleaned = f_desc.replace(" ", "")
    if "log" in cleaned:
        if "^" in cleaned:
            idx = cleaned.index("log")
            power_part = cleaned[idx + 3:]
            if "^" in power_part:
                try:
                    return float(power_part.split("^")[1].split("*")[0].split(")")[0])
                except (ValueError, IndexError):
                    return 1
        return 0
    return 0


def substitution_skeleton(guess, recurrence_desc):
    """Generate a substitution method proof skeleton.

    Args:
        guess: guessed bound, e.g., "O(n log n)"
        recurrence_desc: description of the recurrence

    Returns:
        list of proof steps as strings.
    """
    steps = []

    steps.append(f"Goal: Prove T(n) = {guess}")
    steps.append(f"Recurrence: {recurrence_desc}")
    steps.append("")
    steps.append("Step 1 — Base case:")
    steps.append("  Show T(1) = c for some constant c. Verify the guess holds for small n.")
    steps.append("")
    steps.append("Step 2 — Inductive hypothesis:")
    steps.append("  Assume T(k) <= c * g(k) for all k < n, where g(n) is the guessed bound.")
    steps.append("")
    steps.append("Step 3 — Inductive step:")
    steps.append(f"  Substitute the inductive hypothesis into {recurrence_desc}:")
    steps.append("  T(n) = a*T(n/b) + f(n)")
    steps.append("       <= a * c * g(n/b) + f(n)      [by IH]")
    steps.append("       <= c * g(n)                    [need to show this]")
    steps.append("")
    steps.append("Step 4 — Choose constants:")
    steps.append("  Find c > 0 and n_0 >= 1 such that the inequality holds for all n >= n_0.")
    steps.append("")
    steps.append("Step 5 — Verify:")
    steps.append("  Expand the algebra, pick c large enough to absorb lower-order terms,")
    steps.append("  and confirm the inequality holds.")

    return steps


def main():
    print("=" * 60)
    print("RECURRENCE ANALYSIS TOOLKIT")
    print("=" * 60)

    # --- Recursion Tree Visualizer ---
    print("\n--- Recursion Tree: T(n) = 2T(n/2) + n, n=16 ---\n")
    recursion_tree(2, 2, lambda n: n, 16, max_depth=4)

    print("\n--- Work per level ---\n")
    levels = compute_tree_work(2, 2, lambda n: n, 16)
    total = 0
    for level, work, nodes, size in levels:
        print(f"  Level {level}: {nodes} nodes of size {size}, work = {work}")
        total += work
    print(f"  Total work: {total}")

    # --- Master Theorem Solver ---
    print("\n" + "=" * 60)
    print("MASTER THEOREM SOLVER")
    print("=" * 60)

    recurrences = [
        (2, 2, "n", "Merge sort"),
        (1, 2, "1", "Binary search"),
        (2, 2, "n^2", "Divide-and-conquer with quadratic merge"),
        (4, 2, "n", "Strassen-like (simplified)"),
        (3, 4, "n*log(n)", "Fast matrix multiply variant"),
    ]

    for a, b, f_desc, label in recurrences:
        result = master_theorem(a, b, f_desc)
        print(f"\n  {label}: T(n) = {a}T(n/{b}) + {f_desc}")
        print(f"    log_{b}({a}) = {result['log_b_a']:.4f}")
        print(f"    Case {result['case']}: {result['solution']}")
        print(f"    Why: {result['explanation']}")

    # --- Substitution Method ---
    print("\n" + "=" * 60)
    print("SUBSTITUTION METHOD PROOF SKELETON")
    print("=" * 60)

    skeleton = substitution_skeleton("O(n log n)", "T(n) = 2T(n/2) + n")
    print()
    for line in skeleton:
        print(f"  {line}")

    print("\n--- Second example ---\n")
    skeleton2 = substitution_skeleton("Theta(n log^2 n)", "T(n) = 2T(n/2) + n log n")
    for line in skeleton2:
        print(f"  {line}")


if __name__ == "__main__":
    main()
