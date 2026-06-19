"""What Counts as Computation? — Vending Machine as a Finite-State Machine.

Demonstrates the finite automaton model of computation:
- States, alphabet, transition function, start state.
- Deterministic execution: one symbol at a time, one next state.
- The limit: a finite automaton cannot count arbitrarily high.
"""
from __future__ import annotations
from dataclasses import dataclass, field


@dataclass
class VendingMachine:
    """A vending machine modeled as a deterministic finite automaton.

    States are strings like 'idle', 'credit_5', 'credit_10', ...
    Alphabet: coin denominations as integers (5, 10, 25).
    The machine dispenses a product when accumulated credit >= price.
    """

    price: int = 25
    max_credit: int = 100
    state: str = "idle"
    credit: int = 0
    dispensed: int = 0
    history: list[str] = field(default_factory=list)

    def __post_init__(self) -> None:
        self._build_transitions()

    def _build_transitions(self) -> None:
        """Build the transition table: (state, input) -> next_state.

        States: 'idle', 'credit_{n}' for n in [5, max_credit) stepping by 5.
        Inputs: 5, 10, 25 (coin denominations).
        'dispense' is an output event, not a state — the machine returns to idle.
        """
        self.transitions: dict[tuple[str, int], str] = {}
        step = 5
        states = ["idle"] + [
            f"credit_{c}" for c in range(step, self.max_credit, step)
        ]
        for s in states:
            current_credit = 0 if s == "idle" else int(s.split("_")[1])
            for coin in (5, 10, 25):
                new_credit = min(current_credit + coin, self.max_credit - step)
                if current_credit + coin >= self.price:
                    # Dispense and reset — any overage is lost (simplified model)
                    self.transitions[(s, coin)] = "idle"
                else:
                    self.transitions[(s, coin)] = f"credit_{new_credit}"

    def step(self, coin: int) -> str | None:
        """Process one input symbol (a coin). Returns 'DISPENSE' or None."""
        if coin not in (5, 10, 25):
            raise ValueError(f"Invalid coin: {coin}. Accepted: 5, 10, 25")

        key = (self.state, coin)
        if key not in self.transitions:
            raise RuntimeError(f"No transition for state={self.state}, coin={coin}")

        prev_state = self.state
        self.state = self.transitions[key]

        if self.state == "idle" and prev_state != "idle":
            self.credit = 0
            self.dispensed += 1
            event = f"Inserted {coin}¢ → DISPENSE (product #{self.dispensed})"
            self.history.append(event)
            return "DISPENSE"
        else:
            self.credit = int(self.state.split("_")[1]) if self.state != "idle" else 0
            event = f"Inserted {coin}¢ → {self.state} (credit={self.credit}¢)"
            self.history.append(event)
            return None

    def insert(self, coin: int) -> str | None:
        """Convenience alias for step()."""
        return self.step(coin)

    def reset(self) -> None:
        """Return to idle state, credit=0, clear history."""
        self.state = "idle"
        self.credit = 0
        self.history.clear()

    def state_count(self) -> int:
        """Return the total number of states in this automaton."""
        all_states = set()
        for (s, _), ns in self.transitions.items():
            all_states.add(s)
            all_states.add(ns)
        return len(all_states)

    def display_history(self) -> None:
        """Print the sequence of transitions."""
        for i, event in enumerate(self.history, 1):
            print(f"  Step {i}: {event}")


def demonstrate_vending_machine() -> None:
    print("=== Vending Machine — Finite Automaton Demo ===\n")

    vm = VendingMachine(price=25)
    print(f"Price: {vm.price}¢  |  States: {vm.state_count()}\n")

    # Scenario 1: exact change
    print("Scenario 1: Exact change (10 + 10 + 5)")
    for coin in (10, 10, 5):
        result = vm.insert(coin)
    vm.display_history()
    vm.reset()

    # Scenario 2: overpay
    print("\nScenario 2: Overpay (25¢ coin)")
    result = vm.insert(25)
    vm.display_history()
    vm.reset()

    # Scenario 3: multiple purchases
    print("\nScenario 3: Two purchases in a row")
    for coin in (10, 10, 5, 25, 10, 10, 5):
        vm.insert(coin)
    vm.display_history()

    print(f"\nTotal dispensed: {vm.dispensed}")
    print(f"Total states: {vm.state_count()}")


def demonstrate_limits() -> None:
    """Show what a finite automaton CANNOT do."""
    print("\n=== What Finite Automata Cannot Compute ===\n")

    print("A finite automaton CANNOT recognize:")
    print("  L = {0ⁿ1ⁿ | n ≥ 0}  (equal 0s then 1s)")
    print()
    print("Why? To check that the number of 0s equals the number of 1s,")
    print("you must COUNT — and counting to arbitrary n requires unbounded memory.")
    print("A finite automaton has only finitely many states.")
    print("By the pigeonhole principle, on inputs longer than |Q|,")
    print("the machine must revisit a state and 'forget' the count.")
    print()
    print("This is the fundamental divide: finite memory → limited power.")
    print("Adding a stack (PDA) lets you count. Adding a tape (TM) lets you do anything.")


def comparison_table() -> None:
    """Print the computation model hierarchy."""
    print("\n=== Computation Model Hierarchy ===\n")
    header = f"{'Model':<25} {'Memory':<30} {'Recognizes':<30} {'Limitation'}"
    print(header)
    print("-" * 115)
    rows = [
        ("Finite Automaton", "Finitely many states only", "Regular languages", "Cannot count arbitrarily"),
        ("Pushdown Automaton", "States + unbounded stack", "Context-free languages", "Cannot cross-check two counts"),
        ("Turing Machine", "States + unbounded r/w tape", "Recursively enumerable", "Halting problem is undecidable"),
    ]
    for model, memory, recognizes, limitation in rows:
        print(f"{model:<25} {memory:<30} {recognizes:<30} {limitation}")


def main() -> None:
    demonstrate_vending_machine()
    demonstrate_limits()
    comparison_table()


if __name__ == "__main__":
    main()
