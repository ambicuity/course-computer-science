"""
Lesson 06 — Parsing III: LR, SLR, LALR, GLR
SLR parser: builds LR(0) automaton, constructs action/goto table,
and runs shift-reduce parsing.
"""

from collections import defaultdict, deque

EPSILON = "ε"
ENDMARKER = "$"
AUGMENTED_START = "S'"

# ── Grammar Utilities ───────────────────────────────────────

def augmented_grammar(grammar, start):
    """Add S' → start production."""
    aug = {AUGMENTED_START: [[start]]}
    aug.update(grammar)
    return aug


def all_symbols(grammar):
    """Return (nonterminals, terminals) for a grammar."""
    nonterminals = set(grammar.keys())
    terminals = set()
    for prods in grammar.values():
        for prod in prods:
            for sym in prod:
                if sym not in nonterminals:
                    terminals.add(sym)
    terminals.discard(EPSILON)
    return nonterminals, terminals


def is_terminal(sym, nonterminals):
    return sym not in nonterminals and sym != EPSILON


def productions_list(grammar):
    """Enumerate all productions as (lhs, rhs) tuples."""
    prods = []
    for lhs, rhss in grammar.items():
        for rhs in rhss:
            prods.append((lhs, rhs))
    return prods

# ── Items ───────────────────────────────────────────────────

class Item:
    """LR(0) item: (production index, dot position)."""
    def __init__(self, prod_index, dot):
        self.prod_index = prod_index
        self.dot = dot

    def __eq__(self, other):
        return isinstance(other, Item) and self.prod_index == other.prod_index and self.dot == other.dot

    def __hash__(self):
        return hash((self.prod_index, self.dot))

    def __repr__(self):
        return f"Item({self.prod_index}, {self.dot})"

    def __lt__(self, other):
        return (self.prod_index, self.dot) < (other.prod_index, other.dot)


def item_str(item, prods):
    """Pretty-print an item."""
    lhs, rhs = prods[item.prod_index]
    symbols = list(rhs) if rhs != [EPSILON] else []
    dotted = symbols[:item.dot] + ["·"] + symbols[item.dot:]
    return f"{lhs} → {' '.join(dotted)}"

# ── Closure and GOTO ────────────────────────────────────────

def compute_closure(items, prods, nonterminals):
    """Compute closure of a set of LR(0) items."""
    closure = set(items)
    changed = True
    while changed:
        changed = False
        for item in list(closure):
            lhs, rhs = prods[item.prod_index]
            symbols = list(rhs) if rhs != [EPSILON] else []
            if item.dot < len(symbols):
                B = symbols[item.dot]
                if B in nonterminals:
                    for i, (plhs, prhs) in enumerate(prods):
                        new_item = Item(i, 0)
                        if new_item not in closure:
                            closure.add(new_item)
                            changed = True
    return frozenset(closure)


def compute_goto(items, symbol, prods, nonterminals):
    """Compute GOTO(I, X): move dot past symbol, then take closure."""
    moved = set()
    for item in items:
        lhs, rhs = prods[item.prod_index]
        symbols = list(rhs) if rhs != [EPSILON] else []
        if item.dot < len(symbols) and symbols[item.dot] == symbol:
            moved.add(Item(item.prod_index, item.dot + 1))
    if not moved:
        return None
    return compute_closure(moved, prods, nonterminals)

# ── Build LR(0) Automaton ──────────────────────────────────

def build_lr0_states(grammar):
    """Build the LR(0) automaton: states and transitions.
    Returns (states, transitions, prods, nonterminals, terminals).
    """
    nonterminals, terminals = all_symbols(grammar)
    prods = productions_list(grammar)
    start_prod = prods[0]  # augmented start

    # Initial state: closure of {S' → · start}
    initial = compute_closure({Item(0, 0)}, prods, nonterminals)

    states = [initial]
    state_map = {initial: 0}
    transitions = {}  # (state_index, symbol) -> state_index
    queue = deque([initial])

    all_syms = nonterminals | terminals

    while queue:
        current = queue.popleft()
        i = state_map[current]
        for symbol in sorted(all_syms):
            goto = compute_goto(current, symbol, prods, nonterminals)
            if goto is not None:
                if goto not in state_map:
                    state_map[goto] = len(states)
                    states.append(goto)
                    queue.append(goto)
                transitions[(i, symbol)] = state_map[goto]

    return states, transitions, prods, nonterminals, terminals

# ── FIRST / FOLLOW for SLR ─────────────────────────────────

def compute_first_seq(symbols, first, nonterminals):
    """FIRST of a sequence of symbols."""
    result = set()
    for sym in symbols:
        if is_terminal(sym, nonterminals):
            result.add(sym)
            return result
        result.update(first[sym] - {EPSILON})
        if EPSILON not in first[sym]:
            return result
    result.add(EPSILON)
    return result


def compute_first(grammar, nonterminals):
    """Compute FIRST sets for non-terminals."""
    first = defaultdict(set)
    changed = True
    while changed:
        changed = False
        for nt, prods in grammar.items():
            for prod in prods:
                if prod == [EPSILON]:
                    if EPSILON not in first[nt]:
                        first[nt].add(EPSILON)
                        changed = True
                    continue
                for sym in prod:
                    if is_terminal(sym, nonterminals):
                        if sym not in first[nt]:
                            first[nt].add(sym)
                            changed = True
                        break
                    before = len(first[nt])
                    first[nt].update(first[sym] - {EPSILON})
                    if len(first[nt]) > before:
                        changed = True
                    if EPSILON not in first[sym]:
                        break
                else:
                    if EPSILON not in first[nt]:
                        first[nt].add(EPSILON)
                        changed = True
    return dict(first)


def compute_follow(grammar, nonterminals, first):
    """Compute FOLLOW sets for non-terminals."""
    start = list(grammar.keys())[0]
    follow = defaultdict(set)
    follow[start].add(ENDMARKER)

    changed = True
    while changed:
        changed = False
        for nt, prods in grammar.items():
            for prod in prods:
                for i, sym in enumerate(prod):
                    if is_terminal(sym, nonterminals):
                        continue
                    rest = prod[i + 1:]
                    if rest:
                        fr = compute_first_seq(rest, first, nonterminals)
                        before = len(follow[sym])
                        follow[sym].update(fr - {EPSILON})
                        if EPSILON in fr:
                            follow[sym].update(follow[nt])
                        if len(follow[sym]) > before:
                            changed = True
                    else:
                        before = len(follow[sym])
                        follow[sym].update(follow[nt])
                        if len(follow[sym]) > before:
                            changed = True
    return dict(follow)

# ── SLR Table ───────────────────────────────────────────────

def build_slr_table(states, transitions, prods, nonterminals, terminals, follow):
    """Build the SLR action/goto table.
    action[state][terminal] = ('shift', s) | ('reduce', prod_index) | 'accept'
    goto_table[state][nonterminal] = state_index
    """
    action = defaultdict(dict)
    goto_table = defaultdict(dict)

    for (i, sym), j in transitions.items():
        if is_terminal(sym, nonterminals):
            if sym in action[i]:
                action[i][sym] = "__CONFLICT__"
            else:
                action[i][sym] = ("shift", j)
        else:
            goto_table[i][sym] = j

    for i, state in enumerate(states):
        for item in state:
            lhs, rhs = prods[item.prod_index]
            symbols = list(rhs) if rhs != [EPSILON] else []
            if item.dot == len(symbols):
                # Complete item
                if lhs == AUGMENTED_START:
                    action[i][ENDMARKER] = "accept"
                else:
                    for a in follow.get(lhs, set()):
                        if a in action[i]:
                            action[i][a] = "__CONFLICT__"
                        else:
                            action[i][a] = ("reduce", item.prod_index)

    return dict(action), dict(goto_table)


def find_conflicts(action):
    """Find shift-reduce and reduce-reduce conflicts."""
    sr_conflicts = []
    rr_conflicts = []
    for state, entries in action.items():
        for terminal, act in entries.items():
            if act == "__CONFLICT__":
                sr_conflicts.append((state, terminal))
    return sr_conflicts, rr_conflicts

# ── SLR Parser ──────────────────────────────────────────────

def slr_parse(action, goto_table, prods, tokens):
    """Shift-reduce parse. Returns a list of actions taken."""
    stack = [(0, None)]  # (state, symbol)
    tokens = tokens + [ENDMARKER]
    ip = 0
    trace = []

    while True:
        state = stack[-1][0]
        lookahead = tokens[ip]

        act = action.get(state, {}).get(lookahead)
        if act is None:
            raise SyntaxError(
                f"No action for state {state}, lookahead '{lookahead}'"
            )
        if act == "__CONFLICT__":
            raise SyntaxError(
                f"Shift-reduce conflict in state {state} on '{lookahead}'"
            )

        if act == "accept":
            trace.append("accept")
            return trace

        if act[0] == "shift":
            _, next_state = act
            stack.append((next_state, lookahead))
            ip += 1
            trace.append(f"shift {lookahead}")
        elif act[0] == "reduce":
            _, prod_index = act
            lhs, rhs = prods[prod_index]
            symbols = list(rhs) if rhs != [EPSILON] else []
            # Pop |rhs| items
            for _ in range(len(symbols)):
                stack.pop()
            prev_state = stack[-1][0]
            next_state = goto_table.get(prev_state, {}).get(lhs)
            if next_state is None:
                raise SyntaxError(f"No GOTO for state {prev_state}, symbol {lhs}")
            stack.append((next_state, lhs))
            rhs_str = " ".join(symbols) if symbols else EPSILON
            trace.append(f"reduce {lhs} → {rhs_str}")
        else:
            raise SyntaxError(f"Unexpected action: {act}")

# ── Demo ────────────────────────────────────────────────────

def demo_grammar(name, grammar, start, tokens_list):
    """Run full SLR build + parse demonstration."""
    print(f"\n{'#' * 60}")
    print(f"  Grammar: {name}")
    print(f"{'#' * 60}")

    for lhs, rhss in grammar.items():
        for rhs in rhss:
            print(f"  {lhs} → {' '.join(rhs)}")

    aug = augmented_grammar(grammar, start)
    states, transitions, prods, nonterminals, terminals = build_lr0_states(aug)
    first = compute_first(aug, nonterminals)
    follow = compute_follow(aug, nonterminals, first)

    print(f"\n  LR(0) states: {len(states)}")
    for i, state in enumerate(states):
        items_str = ", ".join(sorted(item_str(it, prods) for it in state))
        print(f"  State {i}: {{{items_str}}}")

    print(f"\n  Transitions:")
    for (i, sym), j in sorted(transitions.items()):
        print(f"    GOTO({i}, {sym}) = {j}")

    print(f"\n  FOLLOW sets:")
    for nt in sorted(follow.keys()):
        if nt == AUGMENTED_START:
            continue
        members = ", ".join(sorted(follow[nt]))
        print(f"    FOLLOW({nt}) = {{ {members} }}")

    action, goto_table = build_slr_table(states, transitions, prods, nonterminals, terminals, follow)
    sr, rr = find_conflicts(action)
    if sr:
        print(f"\n  ⚠ Shift-reduce conflicts: {sr}")
    if rr:
        print(f"  ⚠ Reduce-reduce conflicts: {rr}")
    if not sr and not rr:
        print(f"\n  ✓ No conflicts — grammar is SLR(1)")

    # Print table
    term_list = sorted(terminals | {ENDMARKER})
    print(f"\n  Action table:")
    header = f"    {'ST':4s} | " + " | ".join(f"{t:10s}" for t in term_list)
    print(header)
    for s in range(len(states)):
        row = f"    {s:4d} | "
        cells = []
        for t in term_list:
            a = action.get(s, {}).get(t)
            if a is None:
                cells.append(f"{'':10s}")
            elif a == "__CONFLICT__":
                cells.append(f"{'CONFLICT':10s}")
            elif a == "accept":
                cells.append(f"{'acc':10s}")
            elif a[0] == "shift":
                cells.append(f"s{a[1]:9d}")
            elif a[0] == "reduce":
                lhs, rhs = prods[a[1]]
                r_str = f"{lhs}→{''.join(rhs)}"
                cells.append(f"{r_str:10s}")
        row += " | ".join(cells)
        print(row)

    for tokens in tokens_list:
        print(f"\n  Parsing: {' '.join(tokens)}")
        try:
            trace = slr_parse(action, goto_table, prods, tokens)
            for step in trace:
                print(f"    {step}")
            print("    ✓ Accepted")
        except SyntaxError as e:
            print(f"    ✗ Error: {e}")


def main():
    # Grammar 1: Expression grammar (left-recursive, not LL(1))
    expr_grammar = {
        "E": [["E", "+", "T"], ["T"]],
        "T": [["T", "*", "F"], ["F"]],
        "F": [["(", "E", ")"], ["id"]],
    }
    demo_grammar("Expression (E → E+T | T)", expr_grammar, "E", [
        ["id", "+", "id", "*", "id"],
        ["id"],
    ])

    # Grammar 2: Simple sequence
    seq_grammar = {
        "S": [["A", "B"]],
        "A": [["a"]],
        "B": [["b"]],
    }
    demo_grammar("Sequence (S → AB, A → a, B → b)", seq_grammar, "S", [
        ["a", "b"],
    ])

    # Grammar 3: Dangling else (has shift-reduce conflict)
    dangling_grammar = {
        "S": [["i", "E", "t", "S"], ["i", "E", "t", "S", "e", "S"], ["a"]],
        "E": [["b"]],
    }
    demo_grammar("Dangling Else (ambiguity demo)", dangling_grammar, "S", [
        ["i", "b", "t", "a"],
        ["i", "b", "t", "i", "b", "t", "a", "e", "a"],
    ])


if __name__ == "__main__":
    main()
