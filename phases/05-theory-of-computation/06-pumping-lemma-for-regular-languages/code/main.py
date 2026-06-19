"""
Pumping Lemma Demonstration
============================
Demonstrates pumping on regular languages (where it works)
and shows it fails for non-regular languages (proving they are not regular).
"""

from typing import Callable


# ---------------------------------------------------------------------------
# Helper: DFA simulation for a*b*
# ---------------------------------------------------------------------------

def in_ab_star(s: str) -> bool:
    """Check if s ∈ a*b* using a simple DFA."""
    state = 0  # 0: reading a's, 1: reading b's, 2: reject
    for c in s:
        if state == 0:
            if c == 'a':
                pass
            elif c == 'b':
                state = 1
            else:
                state = 2
        elif state == 1:
            if c == 'b':
                pass
            else:
                state = 2
        elif state == 2:
            return False
    return True


# ---------------------------------------------------------------------------
# Pumping on a regular language: a*b*
# ---------------------------------------------------------------------------

def pumping_lemma_demo(language_func: Callable[[str], bool], p: int, w: str):
    """
    Show that pumping works for a regular language.
    Picks the first valid partition and tests xy^i z for i = 0, 1, 2, 3.
    """
    print(f"Pumping Lemma Demo for w = {w!r} (|w| = {len(w)}, p = {p})")
    print(f"  Language check: w ∈ L? {language_func(w)}")

    # Find a valid partition: |xy| ≤ p, |y| ≥ 1
    for y_len in range(1, p + 1):
        x_len = p - y_len  # |xy| = p, which is ≤ p
        if x_len + y_len > len(w):
            continue
        x = w[:x_len]
        y = w[x_len:x_len + y_len]
        z = w[x_len + y_len:]

        print(f"\n  Partition: x = {x!r}, y = {y!r}, z = {z!r}")
        print(f"    |x| = {len(x)}, |y| = {len(y)}, |z| = {len(z)}")
        print(f"    |xy| = {x_len + y_len} ≤ p = {p} ✓")

        all_in = True
        for i in range(4):
            pumped = x + y * i + z
            result = language_func(pumped)
            status = "✓" if result else "✗"
            print(f"    xy^{i}z = {pumped!r:20s} ∈ L? {result} {status}")
            if not result:
                all_in = False

        if all_in:
            print(f"\n  ✓ Pumping works for this partition!")
            return True

    print(f"\n  No valid partition found where all pumped strings stay in L.")
    return False


# ---------------------------------------------------------------------------
# Proving non-regularity: pumping FAILS
# ---------------------------------------------------------------------------

def prove_not_regular(
    language_description: str,
    language_func: Callable[[str], bool],
    p: int,
    w: str,
    explain: Callable[[str, str, str, int], str] | None = None,
):
    """
    Demonstrate that pumping fails for a non-regular language.
    Shows that for every valid partition w = xyz, some xy^i z ∉ L.
    """
    print(f"\n{'='*60}")
    print(f"Proving {language_description} is NOT regular")
    print(f"{'='*60}")
    print(f"Assume L is regular with pumping length p = {p}.")
    print(f"Choose w = {w!r} ∈ L, |w| = {len(w)} ≥ p.")

    any_partition_all_pass = False

    for y_len in range(1, p + 1):
        for x_len in range(0, p - y_len + 1):
            if x_len + y_len > len(w):
                continue
            if x_len + y_len > p:
                continue
            x = w[:x_len]
            y = w[x_len:x_len + y_len]
            z = w[x_len + y_len:]

            if len(y) == 0:
                continue

            print(f"\n  Partition: x = {x!r}, y = {y!r}, z = {z!r}")

            pumped_0 = x + z
            result_0 = language_func(pumped_0)
            pumped_2 = x + y * 2 + z
            result_2 = language_func(pumped_2)

            status_0 = "✓" if result_0 else "✗"
            status_2 = "✓" if result_2 else "✗"

            if explain:
                reason = explain(x, y, z, p)
                print(f"    xy^0z = {pumped_0!r} ∈ L? {result_0} {status_0}")
                print(f"    xy^2z = {pumped_2!r} ∈ L? {result_2} {status_2}")
                print(f"    Reason: {reason}")
            else:
                print(f"    xy^0z = {pumped_0!r} ∈ L? {result_0} {status_0}")
                print(f"    xy^2z = {pumped_2!r} ∈ L? {result_2} {status_2}")

            if result_0 and result_2:
                any_partition_all_pass = True

    if not any_partition_all_pass:
        print(f"\n  ∴ For every valid partition, pumping fails.")
        print(f"  ∴ By pumping lemma, L is NOT regular. ∎")
    else:
        # This means we need a different string choice
        print(f"\n  Note: Some partitions pump back into L.")
        print(f"  The pumping lemma requires ALL partitions to fail.")
        print(f"  A different string w may be needed for a clean proof.")


# ---------------------------------------------------------------------------
# Worked example 1: {a^n b^n | n ≥ 0} is NOT regular
# ---------------------------------------------------------------------------

def in_an_bn(s: str) -> bool:
    """Check if s ∈ {a^n b^n | n ≥ 0}."""
    n = len(s)
    i = 0
    while i < n and s[i] == 'a':
        i += 1
    j = i
    while j < n and s[j] == 'b':
        j += 1
    return j == n and i == n - i


def explain_an_bn(x: str, y: str, z: str, p: int) -> str:
    a_count_y = y.count('a')
    b_count_y = y.count('b')
    if a_count_y > 0 and b_count_y == 0:
        return (f"y = {a_count_y} a's. Pumping i=0 removes {a_count_y} a's "
                f"but keeps same b count → unbalanced.")
    elif b_count_y > 0:
        return (f"y contains {b_count_y} b's. Pumping disrupts structure.")
    return "Pumping disrupts the equal count."


# ---------------------------------------------------------------------------
# Worked example 2: {ww | w ∈ {a,b}*} is NOT regular
# ---------------------------------------------------------------------------

def in_ww(s: str) -> bool:
    """Check if s ∈ {ww | w ∈ {a,b}*}."""
    n = len(s)
    if n % 2 != 0:
        return False
    half = n // 2
    return s[:half] == s[half:]


def explain_ww(x: str, y: str, z: str, p: int) -> str:
    pumped_len = len(x) + len(z)
    if pumped_len % 2 != 0:
        return (f"xy^0z has length {pumped_len} (odd) → cannot be ww.")
    pumped_len_2 = len(x) + 2 * len(y) + len(z)
    if pumped_len_2 % 2 != 0:
        return (f"xy^2z has length {pumped_len_2} (odd) → cannot be ww.")
    return f"Structural mismatch after pumping."


# ---------------------------------------------------------------------------
# Worked example 2b: {ww | w ∈ {a,b,c}*} with a string where ALL partitions fail
# ---------------------------------------------------------------------------

def explain_ww_abc(x: str, y: str, z: str, p: int) -> str:
    pumped_len = len(x) + len(z)
    if pumped_len % 2 != 0:
        return f"xy^0z has length {pumped_len} (odd) → cannot be ww."
    pumped_len_2 = len(x) + 2 * len(y) + len(z)
    if pumped_len_2 % 2 != 0:
        return f"xy^2z has length {pumped_len_2} (odd) → cannot be ww."
    return f"Pumping disrupts the ww structure."


# ---------------------------------------------------------------------------
# Main demo
# ---------------------------------------------------------------------------

def main():
    print("=" * 60)
    print("  PUMPING LEMMA DEMONSTRATIONS")
    print("=" * 60)

    # --- Part 1: Pumping works for regular language a*b* ---
    print("\n--- Part 1: Pumping on REGULAR language a*b* ---\n")
    pumping_lemma_demo(in_ab_star, p=3, w="aaabbb")

    # --- Part 2: {a^n b^n} is NOT regular ---
    prove_not_regular(
        "{a^n b^n | n ≥ 0}",
        in_an_bn,
        p=4,
        w="aaaabbbb",
        explain=explain_an_bn,
    )

    # --- Part 3: {ww | w ∈ {a,b,c}*} is NOT regular ---
    # Using abcabc where |xy| ≤ 3 means y cannot span both halves.
    # Every partition of y within first 3 chars disrupts the ww structure
    # because the alphabet has 3 distinct chars (no single-char repeats work).
    prove_not_regular(
        "{ww | w ∈ {a,b,c}*}",
        in_ww,
        p=3,
        w="abcabc",  # "abc" + "abc"
        explain=explain_ww_abc,
    )

    # Verify abcabc partitions all fail:
    print("\n  Verification for abcabc with p=3:")
    w = "abcabc"
    for y_len in range(1, 4):
        for x_len in range(0, 4 - y_len):
            if x_len + y_len > 3:
                continue
            x = w[:x_len]
            y = w[x_len:x_len + y_len]
            z = w[x_len + y_len:]
            p0 = in_ww(x + z)
            p2 = in_ww(x + y * 2 + z)
            status = "FAIL ✓" if not (p0 and p2) else "PASS ✗"
            print(f"    x={x!r}, y={y!r}, z={z!r} → i=0:{p0}, i=2:{p2} → {status}")

    # --- Additional demonstration ---
    print("\n" + "=" * 60)
    print("  QUICK PUMPING CHECKS")
    print("=" * 60)

    print("\nRegular language a*b*:")
    for w in ["ab", "aaabb", "aaaabbbb"]:
        print(f"  {w!r} ∈ a*b*? {in_ab_star(w)}")
    print("  Pumping always works for regular languages.")

    print("\nNon-regular {a^n b^n}:")
    for w in ["ab", "aabb", "aaabbb", "aaaabbbb"]:
        print(f"  {w!r} ∈ {{a^n b^n}}? {in_an_bn(w)}")
    print("  Pumping fails → proves non-regularity.")

    print("\nNon-regular {ww}:")
    for w in ["aa", "abab", "abcabc"]:
        print(f"  {w!r} ∈ {{ww}}? {in_ww(w)}")
    print("  Pumping fails for carefully chosen strings → proves non-regularity.")


if __name__ == "__main__":
    main()
