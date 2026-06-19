"""
Pushdown Automata & CFG Equivalence
Phase 05 — Theory of Computation, Lesson 08

PDA class with step simulation, cfg_to_pda and pda_to_cfg conversions.
Demonstrates PDAs for {aⁿbⁿ} and balanced parentheses.
"""

from __future__ import annotations
from collections import defaultdict
from dataclasses import dataclass
from typing import Optional


@dataclass(frozen=True)
class PDATransition:
    next_state: str
    stack_push: tuple[str, ...]


class PDA:
    def __init__(self):
        self.states: set[str] = set()
        self.input_alphabet: set[str] = set()
        self.stack_alphabet: set[str] = set()
        self.transitions: dict[tuple[str, str, str], list[PDATransition]] = {}
        self.start_state: str = "q0"
        self.start_stack: str = "Z"
        self.accept_states: set[str] = set()

    def add_transition(self, state: str, input_sym: str, stack_top: str,
                       next_state: str, stack_push: tuple[str, ...]) -> None:
        self.states.update([state, next_state])
        if input_sym != "":
            self.input_alphabet.add(input_sym)
        self.stack_alphabet.update(stack_push)
        self.stack_alphabet.add(stack_top)
        key = (state, input_sym, stack_top)
        if key not in self.transitions:
            self.transitions[key] = []
        self.transitions[key].append(PDATransition(next_state, stack_push))

    def accepts(self, input_string: str, max_steps: int = 50000) -> bool:
        """Accept by state (if accept_states is non-trivial) or by empty stack."""
        accept_by_empty_stack = len(self.accept_states) == 1 and \
            self.start_state in self.accept_states
        configs = [(self.start_state, 0, [self.start_stack])]
        visited = set()
        steps = 0
        while configs and steps < max_steps:
            state, pos, stack = configs.pop()
            steps += 1
            config_key = (state, pos, tuple(stack))
            if config_key in visited:
                continue
            visited.add(config_key)
            if pos == len(input_string):
                if accept_by_empty_stack and not stack:
                    return True
                if not accept_by_empty_stack and state in self.accept_states:
                    return True
            stack_top = stack[-1] if stack else ""
            # Input-consuming transitions
            if pos < len(input_string):
                for tr in self.transitions.get((state, input_string[pos], stack_top), []):
                    new_stack = list(stack[:-1])
                    for s in reversed(tr.stack_push):
                        if s != "":
                            new_stack.append(s)
                    configs.append((tr.next_state, pos + 1, new_stack))
            # Epsilon-input transitions
            for tr in self.transitions.get((state, "", stack_top), []):
                new_stack = list(stack[:-1])
                for s in reversed(tr.stack_push):
                    if s != "":
                        new_stack.append(s)
                configs.append((tr.next_state, pos, new_stack))
        return False

    def simulate(self, input_string: str, max_steps: int = 5000) -> list[dict]:
        trace = []
        configs = [(self.start_state, 0, [self.start_stack])]
        visited = set()
        steps = 0
        while configs and steps < max_steps:
            state, pos, stack = configs.pop()
            steps += 1
            config_key = (state, pos, tuple(stack))
            if config_key in visited:
                continue
            visited.add(config_key)
            trace.append({
                "step": steps,
                "state": state,
                "position": pos,
                "remaining": input_string[pos:],
                "stack": list(reversed(stack)),
            })
            if pos == len(input_string) and state in self.accept_states:
                trace[-1]["result"] = "ACCEPT"
                return trace
            stack_top = stack[-1] if stack else ""
            if pos < len(input_string):
                for tr in self.transitions.get((state, input_string[pos], stack_top), []):
                    new_stack = list(stack[:-1])
                    for s in reversed(tr.stack_push):
                        if s != "":
                            new_stack.append(s)
                    configs.append((tr.next_state, pos + 1, new_stack))
            for tr in self.transitions.get((state, "", stack_top), []):
                new_stack = list(stack[:-1])
                for s in reversed(tr.stack_push):
                    if s != "":
                        new_stack.append(s)
                configs.append((tr.next_state, pos, new_stack))
        trace.append({"result": "REJECT (max steps)"})
        return trace

    def __repr__(self) -> str:
        lines = [f"PDA(states={sorted(self.states)}, "
                 f"start={self.start_state}, "
                 f"accept={sorted(self.accept_states)})"]
        for key in sorted(self.transitions):
            state, inp, stack_top = key
            for tr in self.transitions[key]:
                inp_str = inp if inp else "ε"
                push_str = "".join(tr.stack_push) if tr.stack_push else "ε"
                lines.append(f"  δ({state}, {inp_str}, {stack_top}) = "
                             f"({tr.next_state}, {push_str})")
        return "\n".join(lines)


def pda_anbn() -> PDA:
    pda = PDA()
    pda.start_state = "q0"
    pda.start_stack = "Z"
    pda.accept_states = {"qf"}
    # Read a, push A
    pda.add_transition("q0", "a", "Z", "q0", ("A", "Z"))
    pda.add_transition("q0", "a", "A", "q0", ("A", "A"))
    # Read b, pop A
    pda.add_transition("q0", "b", "A", "q1", ())
    # Continue popping b's
    pda.add_transition("q1", "b", "A", "q1", ())
    # Accept when only Z remains
    pda.add_transition("q1", "", "Z", "qf", ("Z",))
    # Also accept empty string
    pda.add_transition("q0", "", "Z", "qf", ("Z",))
    return pda


def pda_balanced_parens() -> PDA:
    pda = PDA()
    pda.start_state = "q0"
    pda.start_stack = "Z"
    pda.accept_states = {"qf"}
    # Push on '('
    pda.add_transition("q0", "(", "Z", "q0", ("P", "Z"))
    pda.add_transition("q0", "(", "P", "q0", ("P", "P"))
    # Pop on ')'
    pda.add_transition("q0", ")", "P", "q0", ())
    # Accept when only Z remains
    pda.add_transition("q0", "", "Z", "qf", ("Z",))
    return pda


def cfg_to_pda(variables: set[str], terminals: set[str],
               rules: dict[str, list[list[str]]], start: str) -> PDA:
    """Convert CFG to PDA that accepts by empty stack.
    
    Standard construction (accepts by empty stack):
    - δ(q, ε, Z₀) = (q, SZ₀)  — push start symbol on top of bottom marker
    - δ(q, ε, A)  = (q, α)     — for each rule A → α  
    - δ(q, a, a)  = (q, ε)     — for each terminal a
    - Accept: input consumed AND stack is empty
    """
    pda = PDA()
    pda.start_state = "q0"
    pda.accept_states = {"q0"}  # accept by empty stack
    pda.start_stack = start     # start with just the start symbol
    pda.states = {"q0"}
    # For each production A → α, add δ(q0, ε, A) = (q0, α)
    for head, bodies in rules.items():
        for body in bodies:
            push = tuple(sym for sym in body if sym != "") or ()
            pda.add_transition("q0", "", head, "q0", push)
    # For each terminal a, add δ(q0, a, a) = (q0, ε)
    for term in terminals:
        pda.add_transition("q0", term, term, "q0", ())
    return pda


def pda_to_cfg(pda: PDA) -> tuple[str, set[str], dict[str, list[list[str]]]]:
    variables: set[str] = set()
    rules: dict[str, list[list[str]]] = defaultdict(list)
    state_list = sorted(pda.states)

    for p in state_list:
        for q in state_list:
            for A in pda.stack_alphabet:
                variables.add(f"[{p},{A},{q}]")

    start_var = f"[{pda.start_state},{pda.start_stack}," \
                f"{next(iter(pda.accept_states))}]"

    for (state, inp, stack_top), trans_list in pda.transitions.items():
        for tr in trans_list:
            r = tr.next_state
            if len(tr.stack_push) == 0:
                for q in state_list:
                    var = f"[{state},{stack_top},{q}]"
                    rhs = ([inp] if inp else []) + [f"[{r},{stack_top},{q}]"]
                    if not rhs:
                        rhs = [f"[{r},{stack_top},{q}]"]
                    rules[var].append(rhs)
            elif len(tr.stack_push) == 1:
                B = tr.stack_push[0]
                for q in state_list:
                    var = f"[{state},{stack_top},{q}]"
                    rhs = ([inp] if inp else []) + [f"[{r},{B},{q}]"]
                    rules[var].append(rhs)
            else:
                k = len(tr.stack_push)
                for q_last in state_list:
                    var = f"[{state},{stack_top},{q_last}]"
                    rhs = ([inp] if inp else [])
                    for idx, B in enumerate(tr.stack_push):
                        q_curr = r if idx == 0 else f"q_mid_{idx}"
                        q_next = q_last if idx == k - 1 else f"q_mid_{idx + 1}"
                        if idx == 0:
                            rhs.append(f"[{r},{B},{q_next}]")
                        elif idx == k - 1:
                            rhs.append(f"[{q_curr},{B},{q_last}]")
                        else:
                            rhs.append(f"[{q_curr},{B},{q_next}]")
                    rules[var].append(rhs)

    for A in pda.stack_alphabet:
        for f in pda.accept_states:
            var = f"[{f},{A},{f}]"
            rules[var].append([])

    return start_var, variables, dict(rules)


def main() -> None:
    print("=" * 60)
    print("Lesson 08: Pushdown Automata & CFG Equivalence")
    print("=" * 60)

    # --- PDA for {aⁿbⁿ} ---
    print("\n1. PDA for {aⁿbⁿ | n ≥ 0}")
    pda1 = pda_anbn()
    print(pda1)
    for n in range(5):
        s = "a" * n + "b" * n
        print(f"  accepts '{s}'? {pda1.accepts(s)}")
    print(f"  accepts 'abab'? {pda1.accepts('abab')}")
    print(f"  accepts 'aaab'? {pda1.accepts('aaab')}")

    # --- PDA for balanced parentheses ---
    print("\n2. PDA for Balanced Parentheses")
    pda2 = pda_balanced_parens()
    print(pda2)
    print(f"  accepts '(())'? {pda2.accepts('(())')}")
    print(f"  accepts '(()'?  {pda2.accepts('(()')}")
    print(f"  accepts '())'?  {pda2.accepts('())')}")

    # --- Trace simulation ---
    print("\n3. Trace simulation of '(())'")
    trace = pda2.simulate("(())")
    for entry in trace:
        print(f"  {entry}")

    # --- CFG → PDA ---
    print("\n4. CFG → PDA Conversion for {aⁿbⁿ}")
    variables = {"S"}
    terminals = {"a", "b"}
    rules = {"S": [["a", "S", "b"], []]}
    pda3 = cfg_to_pda(variables, terminals, rules, "S")
    print(pda3)
    for n in range(4):
        s = "a" * n + "b" * n
        print(f"  accepts '{s}'? {pda3.accepts(s)}")

    # --- PDA → CFG ---
    print("\n5. PDA → CFG Conversion (simple aⁿbⁿ PDA)")
    start_var, cfg_vars, cfg_rules = pda_to_cfg(pda1)
    print(f"  Start variable: {start_var}")
    print(f"  Total variables: {len(cfg_vars)}")
    print(f"  Total rules: {sum(len(b) for b in cfg_rules.values())}")


if __name__ == "__main__":
    main()
