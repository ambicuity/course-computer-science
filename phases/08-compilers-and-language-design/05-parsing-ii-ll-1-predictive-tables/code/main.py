"""
Lesson 05 — Parsing II: LL(1) and Predictive Tables
LL(1) parser generator: computes FIRST sets, FOLLOW sets,
builds a predictive parsing table, and parses input.
"""

from collections import defaultdict

EPSILON = "ε"
ENDMARKER = "$"

# ── Grammar Representation ──────────────────────────────────

def example_grammar():
    """Arithmetic expression grammar in LL(1) form (already left-factored)."""
    return {
        "E":  [["T", "E'"]],
        "E'": [["+", "T", "E'"], [EPSILON]],
        "T":  [["F", "T'"]],
        "T'": [["*", "F", "T'"], [EPSILON]],
        "F":  [["(", "E", ")"], ["id"]],
    }

def is_terminal(symbol, grammar):
    """A symbol is a terminal if it does not appear on the left side of any production."""
    return symbol not in grammar

# ── FIRST Sets ──────────────────────────────────────────────

def compute_first(grammar):
    """Compute FIRST sets for all grammar symbols."""
    first = defaultdict(set)

    changed = True
    while changed:
        changed = False
        for nonterminal, productions in grammar.items():
            for production in productions:
                # Compute FIRST of this production
                for i, symbol in enumerate(production):
                    if is_terminal(symbol, grammar):
                        if symbol not in first[nonterminal]:
                            first[nonterminal].add(symbol)
                            changed = True
                        break  # terminal always stops

                    # symbol is a non-terminal
                    before = len(first[nonterminal])
                    first[nonterminal].update(first[symbol] - {EPSILON})
                    if len(first[nonterminal]) > before:
                        changed = True
                    if EPSILON not in first[symbol]:
                        break  # cannot derive epsilon, stop
                else:
                    # All symbols can derive epsilon
                    if EPSILON not in first[nonterminal]:
                        first[nonterminal].add(EPSILON)
                        changed = True

    return dict(first)


def first_of_sequence(symbols, first, grammar):
    """Compute FIRST of a sequence of symbols."""
    result = set()
    for i, sym in enumerate(symbols):
        if is_terminal(sym, grammar):
            result.add(sym)
            return result
        result.update(first.get(sym, set()) - {EPSILON})
        if EPSILON not in first.get(sym, set()):
            return result
    # All symbols can derive epsilon
    result.add(EPSILON)
    return result

# ── FOLLOW Sets ─────────────────────────────────────────────

def compute_follow(grammar, first):
    """Compute FOLLOW sets for all non-terminals."""
    follow = defaultdict(set)
    start = list(grammar.keys())[0]
    follow[start].add(ENDMARKER)

    changed = True
    while changed:
        changed = False
        for nonterminal, productions in grammar.items():
            for production in productions:
                for i, symbol in enumerate(production):
                    if is_terminal(symbol, grammar):
                        continue

                    # symbol is a non-terminal B
                    rest = production[i + 1:]
                    if rest:
                        first_rest = first_of_sequence(rest, first, grammar)
                        before = len(follow[symbol])
                        follow[symbol].update(first_rest - {EPSILON})
                        if EPSILON in first_rest:
                            follow[symbol].update(follow[nonterminal])
                        if len(follow[symbol]) > before:
                            changed = True
                    else:
                        # B is at the end: add FOLLOW(A)
                        before = len(follow[symbol])
                        follow[symbol].update(follow[nonterminal])
                        if len(follow[symbol]) > before:
                            changed = True

    return dict(follow)

# ── Parsing Table ───────────────────────────────────────────

def build_table(grammar, first, follow):
    """Build the LL(1) predictive parsing table.
    Returns a dict: table[nonterminal][terminal] = production (list of symbols).
    """
    table = defaultdict(dict)

    for nonterminal, productions in grammar.items():
        for production in productions:
            first_prod = first_of_sequence(production, first, grammar)

            for terminal in first_prod - {EPSILON}:
                if terminal in table[nonterminal]:
                    # Conflict!
                    table[nonterminal][terminal] = "__CONFLICT__"
                else:
                    table[nonterminal][terminal] = production

            if EPSILON in first_prod:
                for terminal in follow[nonterminal]:
                    if terminal in table[nonterminal]:
                        table[nonterminal][terminal] = "__CONFLICT__"
                    else:
                        table[nonterminal][terminal] = production

    return dict(table)


def is_ll1(table):
    """Check if the parsing table has any conflicts."""
    for nonterminal, entries in table.items():
        for terminal, production in entries.items():
            if production == "__CONFLICT__":
                return False
    return True

# ── LL(1) Parser ────────────────────────────────────────────

def ll1_parse(table, tokens, start_symbol="E"):
    """Parse a list of tokens using the LL(1) predictive parsing algorithm.
    Returns the leftmost derivation as a list of productions applied.
    """
    stack = [ENDMARKER, start_symbol]
    tokens = tokens + [ENDMARKER]
    ip = 0  # input pointer
    derivation = []

    while stack[-1] != ENDMARKER:
        top = stack[-1]
        current = tokens[ip]

        if is_terminal(top, {k: None for k in table}):  # treat table keys as grammar
            if top == current:
                stack.pop()
                ip += 1
            else:
                raise SyntaxError(
                    f"Expected '{top}', found '{current}' at token position {ip}"
                )
        else:
            entry = table.get(top, {}).get(current, None)
            if entry is None:
                raise SyntaxError(
                    f"No table entry for [{top}, {current}] at token position {ip}"
                )
            if entry == "__CONFLICT__":
                raise SyntaxError(
                    f"LL(1) conflict at [{top}, {current}] — grammar is not LL(1)"
                )
            stack.pop()
            derivation.append((top, entry))
            # Push production in reverse (skip epsilon)
            if entry != [EPSILON]:
                for symbol in reversed(entry):
                    stack.append(symbol)

    if tokens[ip] == ENDMARKER:
        return derivation
    else:
        raise SyntaxError(f"Unexpected token '{tokens[ip]}' after parsing completed")

# ── Demo ────────────────────────────────────────────────────

def print_set_table(sets_dict, title):
    """Pretty-print a dict of sets."""
    print(f"\n{'=' * 40}")
    print(f"  {title}")
    print(f"{'=' * 40}")
    for symbol in sorted(sets_dict.keys()):
        members = ", ".join(sorted(sets_dict[symbol]))
        print(f"  {symbol:6s} = {{ {members} }}")


def print_parsing_table(table):
    """Pretty-print the parsing table."""
    all_terminals = sorted({t for row in table.values() for t in row if t != "__CONFLICT__"})
    print(f"\n{'=' * 60}")
    print("  Predictive Parsing Table")
    print(f"{'=' * 60}")
    header = f"  {'':6s} | " + " | ".join(f"{t:6s}" for t in all_terminals)
    print(header)
    print("  " + "-" * (len(header) - 2))
    for nt in sorted(table.keys()):
        row = f"  {nt:6s} | "
        cells = []
        for t in all_terminals:
            entry = table[nt].get(t, "")
            if entry == "__CONFLICT__":
                cells.append(" CONFL ")
            elif entry:
                rhs = " ".join(entry)
                cells.append(f"{rhs:6s}")
            else:
                cells.append("  -   ")
        row += " | ".join(cells)
        print(row)


def main():
    grammar = example_grammar()
    start = list(grammar.keys())[0]

    # 1. Compute FIRST sets
    first = compute_first(grammar)
    print_set_table(first, "FIRST Sets")

    # 2. Compute FOLLOW sets
    follow = compute_follow(grammar, first)
    print_set_table(follow, "FOLLOW Sets")

    # 3. Build parsing table
    table = build_table(grammar, first, follow)
    print_parsing_table(table)

    # 4. Check LL(1) condition
    print(f"\nGrammar is LL(1): {is_ll1(table)}")

    # 5. Parse examples
    test_inputs = [
        ["id", "+", "id", "*", "id"],
        ["(", "id", "+", "id", ")", "*", "id"],
        ["id"],
    ]

    for tokens in test_inputs:
        print(f"\n--- Parsing: {' '.join(tokens)} ---")
        try:
            derivation = ll1_parse(table, tokens, start)
            for i, (nt, prod) in enumerate(derivation):
                rhs = " ".join(prod)
                print(f"  {i + 1}. {nt} → {rhs}")
            print("  ✓ Accepted")
        except SyntaxError as e:
            print(f"  ✗ Error: {e}")


if __name__ == "__main__":
    main()
