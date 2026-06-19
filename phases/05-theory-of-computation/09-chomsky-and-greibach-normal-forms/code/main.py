"""
Chomsky and Greibach Normal Forms
Phase 05 — Theory of Computation, Lesson 09

to_cnf() and to_gnf() converters. Demonstrates conversion on an arithmetic expression grammar.
Verifies that original and normal-form grammars derive the same strings.
"""

from __future__ import annotations
from collections import defaultdict


def to_cnf(variables: set[str], terminals: set[str],
           rules: dict[str, list[list[str]]],
           start: str) -> tuple[set[str], set[str], dict[str, list[list[str]]], str]:
    vars_copy = set(variables)
    terms_copy = set(terminals)
    rules_copy = {h: [list(b) for b in bodies] for h, bodies in rules.items()}
    fresh_counter = [0]

    def fresh_var(prefix: str = "X") -> str:
        name = f"{prefix}{fresh_counter[0]}"
        fresh_counter[0] += 1
        vars_copy.add(name)
        return name

    # Step 1: Eliminate epsilon productions
    nullable = set()
    changed = True
    while changed:
        changed = False
        for head, bodies in rules_copy.items():
            for body in bodies:
                if body == [] or all(sym in nullable for sym in body):
                    if head not in nullable:
                        nullable.add(head)
                        changed = True
    new_rules: dict[str, list[list[str]]] = defaultdict(list)
    for head, bodies in rules_copy.items():
        for body in bodies:
            if body == []:
                if head == start:
                    new_rules[head].append([])
                continue
            indices = [i for i, sym in enumerate(body) if sym in nullable]
            for mask in range(1 << len(indices)):
                new_body = list(body)
                for bit_pos, idx in enumerate(indices):
                    if mask & (1 << bit_pos):
                        new_body[idx] = None
                filtered = [s for s in new_body if s is not None]
                if filtered or head == start:
                    new_rules[head].append(filtered if filtered else [])
    rules_copy = dict(new_rules)

    # Step 2: Eliminate unit productions
    # Compute unit closure: for each A, find all B such that A ⇒* B via unit rules
    unit_closure: dict[str, set[str]] = {v: {v} for v in vars_copy}
    changed = True
    while changed:
        changed = False
        for head, bodies in rules_copy.items():
            for body in bodies:
                if len(body) == 1 and body[0] in vars_copy:
                    target = body[0]
                    # Everything that reaches 'head' also reaches 'target'
                    for v in list(unit_closure):
                        if head in unit_closure[v] and target not in unit_closure[v]:
                            unit_closure[v].add(target)
                            changed = True
    # Build new rules: for each A, add all non-unit rules of every B in closure(A)
    new_rules: dict[str, list[list[str]]] = defaultdict(list)
    for head in vars_copy:
        for target in unit_closure.get(head, {head}):
            for body in rules_copy.get(target, []):
                is_unit = len(body) == 1 and body[0] in vars_copy
                if not is_unit and body not in new_rules[head]:
                    new_rules[head].append(body)
    rules_copy = dict(new_rules)

    # Step 3: Eliminate useless symbols
    generating = set()
    changed = True
    while changed:
        changed = False
        for head, bodies in rules_copy.items():
            if head in generating:
                continue
            for body in bodies:
                if all(sym in terms_copy or sym in generating for sym in body):
                    generating.add(head)
                    changed = True
                    break
    new_rules = defaultdict(list)
    for head, bodies in rules_copy.items():
        if head not in generating:
            continue
        for body in bodies:
            if all(sym in terms_copy or sym in generating for sym in body):
                new_rules[head].append(body)
    rules_copy = dict(new_rules)
    vars_copy = vars_copy & generating

    reachable = {start}
    changed = True
    while changed:
        changed = False
        for head in list(reachable):
            for body in rules_copy.get(head, []):
                for sym in body:
                    if sym not in reachable:
                        reachable.add(sym)
                        changed = True
    new_rules = defaultdict(list)
    for head, bodies in rules_copy.items():
        if head not in reachable:
            continue
        for body in bodies:
            if all(sym in reachable for sym in body):
                new_rules[head].append(body)
    rules_copy = dict(new_rules)
    vars_copy = vars_copy & reachable
    terms_copy = terms_copy & reachable

    # Step 4: Convert long rules (n >= 3) to binary
    new_rules = defaultdict(list)
    for head, bodies in rules_copy.items():
        for body in bodies:
            if len(body) <= 2:
                new_rules[head].append(body)
            else:
                current_head = head
                for i in range(len(body) - 2):
                    x = fresh_var("X")
                    new_rules[current_head].append([body[i], x])
                    current_head = x
                new_rules[current_head].append([body[-2], body[-1]])
    rules_copy = dict(new_rules)

    # Step 5: Convert terminal-in-mixture rules
    term_map: dict[str, str] = {}
    new_rules = defaultdict(list)
    for head, bodies in rules_copy.items():
        for body in bodies:
            new_body = []
            for sym in body:
                if sym in terms_copy and len(body) > 1:
                    if sym not in term_map:
                        t = fresh_var("T")
                        term_map[sym] = t
                        new_rules[t].append([sym])
                    new_body.append(term_map[sym])
                else:
                    new_body.append(sym)
            new_rules[head].append(new_body)
    rules_copy = dict(new_rules)

    return vars_copy, terms_copy, rules_copy, start


def to_gnf(variables: set[str], terminals: set[str],
           rules: dict[str, list[list[str]]],
           start: str) -> tuple[set[str], set[str], dict[str, list[list[str]]], str]:
    vars_set = set(variables)
    terms_set = set(terminals)
    rules_dict = {h: [list(b) for b in bodies] for h, bodies in rules.items()}
    fresh_counter = [0]

    def fresh_var(prefix: str = "G") -> str:
        name = f"{prefix}{fresh_counter[0]}"
        fresh_counter[0] += 1
        vars_set.add(name)
        return name

    vars_set, terms_set, rules_dict, start = to_cnf(
        vars_set, terms_set, rules_dict, start
    )

    var_list = sorted(vars_set)
    for i, ai in enumerate(var_list):
        for j in range(i):
            aj = var_list[j]
            new_bodies = []
            for body in rules_dict.get(ai, []):
                if body and body[0] == aj:
                    for replacement in rules_dict.get(aj, []):
                        new_bodies.append(replacement + body[1:])
                else:
                    new_bodies.append(body)
            rules_dict[ai] = new_bodies

        recursive = []
        non_recursive = []
        for body in rules_dict.get(ai, []):
            if body and body[0] == ai:
                recursive.append(body[1:])
            else:
                non_recursive.append(body)
        if recursive:
            ai_prime = fresh_var(f"{ai}p")
            rules_dict[ai] = []
            for body in non_recursive:
                rules_dict[ai].append(body + [ai_prime])
            rules_dict.setdefault(ai_prime, []).clear()
            for body in recursive:
                rules_dict[ai_prime].append(body + [ai_prime])
            rules_dict[ai_prime].append([])

    return vars_set, terms_set, rules_dict, start


def check_grammar(variables: set[str], terminals: set[str],
                  rules: dict[str, list[list[str]]], start: str) -> dict:
    report = {
        "total_rules": sum(len(b) for b in rules.values()),
        "total_variables": len(variables),
        "has_epsilon_rules": False,
        "has_unit_rules": False,
        "has_long_rules": False,
        "has_terminal_mixed": False,
        "is_cnf": True,
    }
    for head, bodies in rules.items():
        for body in bodies:
            if body == []:
                report["has_epsilon_rules"] = True
                report["is_cnf"] = False
            elif len(body) == 1 and body[0] in variables:
                report["has_unit_rules"] = True
                report["is_cnf"] = False
            elif len(body) > 2:
                report["has_long_rules"] = True
                report["is_cnf"] = False
            elif len(body) == 2:
                if body[0] in terminals or body[1] in terminals:
                    report["has_terminal_mixed"] = True
                    report["is_cnf"] = False
    return report


def derives(variables: set[str], terminals: set[str],
            rules: dict[str, list[list[str]]], start: str,
            target: str, max_depth: int = 20) -> bool:
    """Check if grammar derives target string via leftmost derivation search."""
    def _check(sent: list[str], depth: int) -> bool:
        if depth > max_depth:
            return False
        # Early termination: count minimum terminal contribution
        min_len = 0
        for s in sent:
            if s in terminals:
                min_len += 1
            elif s == "":
                pass
            elif s in rules:
                pass  # could be epsilon
        if min_len > len(target):
            return False
        for i, sym in enumerate(sent):
            if sym in rules:
                for body in rules[sym]:
                    new_sent = sent[:i] + body + sent[i + 1:]
                    if _check(new_sent, depth + 1):
                        return True
                return False
        current = "".join(s for s in sent if s != "")
        return current == target
    return _check([start], 0)


def main() -> None:
    print("=" * 60)
    print("Lesson 09: Chomsky and Greibach Normal Forms")
    print("=" * 60)

    # --- Arithmetic expression grammar ---
    print("\n1. Arithmetic Expression Grammar (original)")
    variables = {"E", "T", "F"}
    terminals = {"+", "*", "(", ")", "i"}
    rules = {
        "E": [["E", "+", "T"], ["T"]],
        "T": [["T", "*", "F"], ["F"]],
        "F": [["(", "E", ")"], ["i"]],
    }
    print(f"  Variables: {variables}")
    print(f"  Terminals: {terminals}")
    for head in sorted(rules):
        for body in rules[head]:
            print(f"  {head} → {' '.join(body) if body else 'ε'}")
    report = check_grammar(variables, terminals, rules, "E")
    print(f"  In CNF? {report['is_cnf']}")
    print(f"  Issues: has_unit={report['has_unit_rules']}, "
          f"has_long={report['has_long_rules']}, "
          f"has_mixed={report['has_terminal_mixed']}")

    # --- Convert to CNF ---
    print("\n2. CNF Conversion")
    cnf_vars, cnf_terms, cnf_rules, cnf_start = to_cnf(
        variables, terminals, rules, "E"
    )
    print(f"  CNF variables ({len(cnf_vars)}): {sorted(cnf_vars)}")
    print(f"  CNF terminals ({len(cnf_terms)}): {sorted(cnf_terms)}")
    for head in sorted(cnf_rules):
        for body in cnf_rules[head]:
            print(f"  {head} → {' '.join(body) if body else 'ε'}")
    cnf_report = check_grammar(cnf_vars, cnf_terms, cnf_rules, cnf_start)
    print(f"  Is CNF? {cnf_report['is_cnf']}")
    print(f"  Total rules: {cnf_report['total_rules']}")

    # --- Verify same language ---
    print("\n3. Verify Original and CNF Derive Same Strings")
    test_strings = ["i", "i+i", "i*i", "i+i*i", "(i)", "(i+i)*i"]
    for s in test_strings:
        orig = derives(variables, terminals, rules, "E", s)
        cnf_d = derives(cnf_vars, cnf_terms, cnf_rules, cnf_start, s)
        match = "✓" if orig == cnf_d else "✗"
        print(f"  '{s}': original={orig}, cnf={cnf_d} {match}")

    # --- aⁿbⁿ grammar to CNF ---
    print("\n4. {aⁿbⁿ} Grammar → CNF")
    ab_vars = {"S"}
    ab_terms = {"a", "b"}
    ab_rules = {"S": [["a", "S", "b"], []]}
    print("  Original: S → aSb | ε")
    cnf_v, cnf_t, cnf_r, cnf_s = to_cnf(ab_vars, ab_terms, ab_rules, "S")
    print("  CNF:")
    for head in sorted(cnf_r):
        for body in cnf_r[head]:
            print(f"    {head} → {' '.join(body) if body else 'ε'}")
    for n in range(4):
        target = "a" * n + "b" * n
        orig = derives(ab_vars, ab_terms, ab_rules, "S", target)
        cnf_d = derives(cnf_v, cnf_t, cnf_r, cnf_s, target)
        match = "✓" if orig == cnf_d else "✗"
        print(f"  '{target}': original={orig}, cnf={cnf_d} {match}")

    # --- GNF Conversion ---
    print("\n5. GNF Conversion (on aⁿbⁿ)")
    gnf_v, gnf_t, gnf_r, gnf_s = to_gnf(
        {"S"}, {"a", "b"}, {"S": [["a", "S", "b"], []]}, "S"
    )
    print(f"  GNF variables ({len(gnf_v)}): {sorted(gnf_v)}")
    for head in sorted(gnf_r):
        for body in gnf_r[head]:
            print(f"    {head} → {' '.join(body) if body else 'ε'}")


if __name__ == "__main__":
    main()
