#!/usr/bin/env python3
"""
Normalization Tool — Phase 10, Lesson 04
Closure, minimal cover, BCNF decomposition, 3NF synthesis, candidate keys.
"""
from typing import Set, List, Tuple, Optional
from itertools import combinations

Attribute = str
FD = Tuple[frozenset, frozenset]


def parse_fds(s: str) -> List[FD]:
    result = []
    for part in s.replace(" ", "").split(","):
        part = part.strip()
        if not part:
            continue
        lhs_str, rhs_str = part.split("\u2192")
        result.append((frozenset(lhs_str), frozenset(rhs_str)))
    return result


def compute_closure(attrs: Set[str], fds: List[FD]) -> Set[str]:
    closure = set(attrs)
    changed = True
    while changed:
        changed = False
        for lhs, rhs in fds:
            if lhs <= closure and not rhs <= closure:
                closure |= rhs
                changed = True
    return closure


def decompose_rhs(fds: List[FD]) -> List[FD]:
    result = []
    for lhs, rhs in fds:
        for a in rhs:
            result.append((lhs, frozenset({a})))
    return result


def extraneous_left(fds: List[FD], idx: int, attr: str) -> bool:
    lhs, rhs = fds[idx]
    reduced_lhs = frozenset(a for a in lhs if a != attr)
    if reduced_lhs == lhs:
        return False
    key_in = compute_closure(set(reduced_lhs), fds)
    return rhs <= key_in


def eliminate_extraneous_left(fds: List[FD]) -> List[FD]:
    result = list(fds)
    for i in range(len(result)):
        lhs, rhs = result[i]
        for a in list(lhs):
            if extraneous_left(result, i, a):
                new_lhs = frozenset(x for x in lhs if x != a)
                result[i] = (new_lhs, rhs)
    return result


def redundant_fd(fds: List[FD], idx: int) -> bool:
    target_lhs, target_rhs = fds[idx]
    reduced = [fd for j, fd in enumerate(fds) if j != idx]
    closure = compute_closure(set(target_lhs), reduced)
    return target_rhs <= closure


def eliminate_redundant(fds: List[FD]) -> List[FD]:
    result = list(fds)
    i = 0
    while i < len(result):
        if redundant_fd(result, i):
            result.pop(i)
        else:
            i += 1
    return result


def minimal_cover(fds: List[FD]) -> List[FD]:
    fds = decompose_rhs(fds)
    fds = eliminate_extraneous_left(fds)
    fds = eliminate_redundant(fds)
    return fds


def find_bcnf_violation(schema: Set[str], fds: List[FD]) -> Optional[FD]:
    for lhs, rhs in fds:
        if not (lhs <= schema and rhs <= schema):
            continue
        if rhs <= lhs:
            continue
        closure = compute_closure(set(lhs), fds) & schema
        if closure != schema:
            return (lhs, rhs)
    return None


def bcnf_decompose(schema: Set[str], fds: List[FD]) -> List[Set[str]]:
    result = [set(schema)]
    while True:
        changed = False
        new_result = []
        for rel in result:
            viol = find_bcnf_violation(rel, fds)
            if viol is not None:
                lhs, _ = viol
                closure = compute_closure(set(lhs), fds) & rel
                r1 = set(closure)
                r2 = (rel - r1) | set(lhs)
                if r1 and r2 and r1 != rel and r2 != rel:
                    new_result.append(r1)
                    new_result.append(r2)
                    changed = True
                else:
                    new_result.append(rel)
            else:
                new_result.append(rel)
        result = new_result
        if not changed:
            break
    return result


def candidate_keys(schema: Set[str], fds: List[FD]) -> List[Set[str]]:
    schema_set = set(schema)
    lhs_attrs: Set[str] = set()
    rhs_attrs: Set[str] = set()
    for lhs, rhs in fds:
        lhs_attrs |= set(lhs)
        rhs_attrs |= set(rhs)
    must_attrs = (lhs_attrs - rhs_attrs) | (schema_set - lhs_attrs - rhs_attrs)
    must_attrs &= schema_set
    maybe_attrs = (lhs_attrs & rhs_attrs) & schema_set
    if compute_closure(must_attrs, fds) == schema_set:
        return [must_attrs]
    keys: List[Set[str]] = []
    maybe_list = sorted(maybe_attrs)
    for r in range(len(maybe_list) + 1):
        for combo in combinations(maybe_list, r):
            test = must_attrs | set(combo)
            if any(k.issubset(test) for k in keys):
                continue
            if compute_closure(test, fds) == schema_set:
                keys.append(test)
        if keys:
            break
    return keys


def synth_3nf(schema: Set[str], fds: List[FD]) -> List[Set[str]]:
    g = minimal_cover(fds)
    groups: dict[frozenset, set] = {}
    for lhs, rhs in g:
        key = frozenset(lhs)
        if key not in groups:
            groups[key] = set(lhs)
        groups[key] |= set(rhs)
    result = list(groups.values())
    result = [r for r in result
              if not any(r != s and r <= s for s in result)]
    keys = candidate_keys(schema, fds)
    if not any(any(k <= r for k in keys) for r in result):
        result.append(keys[0])
    return result


def format_set(s: set) -> str:
    return "".join(sorted(s))


def show_section(title: str):
    print(f"\n{'=' * 60}")
    print(f"  {title}")
    print(f"{'=' * 60}")


def demo_school():
    attr_names = {
        "S": "StudentID", "N": "StudentName", "M": "Major",
        "A": "AdvisorID", "D": "AdvisorName",
        "C": "CourseCode", "T": "CourseTitle",
        "I": "InstructorID", "E": "InstructorName", "G": "Grade",
    }
    schema = set("SNMADCTIEG")
    fd_str = "S\u2192NMA, A\u2192D, C\u2192TI, I\u2192E, SC\u2192G"
    fds = parse_fds(fd_str)

    show_section("School Database — Original Schema & FDs")
    attr_list = ", ".join(f"{k}={v}" for k, v in sorted(attr_names.items()))
    print(f"  Attributes: {{{attr_list}}}")
    print(f"  Schema: {{{format_set(schema)}}}")
    print(f"  FDs: {fd_str}")

    show_section("Attribute Closure Examples")
    for attrs in ["S", "C", "SC", "A"]:
        cl = compute_closure(set(attrs), fds)
        print(f"  {attrs}\u207a = {{{format_set(cl)}}}")

    show_section("Minimal Cover")
    mc = minimal_cover(fds)
    for lhs, rhs in mc:
        print(f"  {format_set(lhs)} \u2192 {format_set(rhs)}")

    show_section("Candidate Keys")
    cks = candidate_keys(schema, fds)
    for ck in cks:
        print(f"  {{{format_set(ck)}}}")

    show_section("BCNF Decomposition (lossless)")
    bcnf_rels = bcnf_decompose(schema, fds)
    for i, rel in enumerate(bcnf_rels, 1):
        names = ", ".join(attr_names.get(a, a) for a in sorted(rel))
        print(f"  R{i}: {{{format_set(rel)}}}  \u2192  {names}")

    show_section("3NF Synthesis (dependency-preserving)")
    tnf_rels = synth_3nf(schema, fds)
    for i, rel in enumerate(tnf_rels, 1):
        names = ", ".join(attr_names.get(a, a) for a in sorted(rel))
        print(f"  R{i}: {{{format_set(rel)}}}  \u2192  {names}")


def demo_3nf_not_bcnf():
    show_section("3NF-but-not-BCNF Example")
    schema = set("ABC")
    fd_str = "AB\u2192C, C\u2192B"
    fds = parse_fds(fd_str)
    print(f"  Schema: {{{format_set(schema)}}}, FDs: {fd_str}")
    cks = candidate_keys(schema, fds)
    print("  Candidate keys:")
    for ck in cks:
        print(f"    {{{format_set(ck)}}}")

    bcnf_rels = bcnf_decompose(schema, fds)
    print("  BCNF decomposition:")
    for i, rel in enumerate(bcnf_rels, 1):
        print(f"    R{i}: {{{format_set(rel)}}}")
    print("    (loses FD AB\u2192C \u2014 can't enforce across decomposed tables)")

    tnf_rels = synth_3nf(schema, fds)
    print("  3NF synthesis:")
    for i, rel in enumerate(tnf_rels, 1):
        print(f"    R{i}: {{{format_set(rel)}}}")
    print("    (preserves all FDs)")


def main():
    demo_school()
    demo_3nf_not_bcnf()
    print()


if __name__ == "__main__":
    main()
