import random
from collections import defaultdict


class Message:
    def __init__(self, src, dst, round_num, value):
        self.src = src
        self.dst = dst
        self.round_num = round_num
        self.value = value

    def __repr__(self):
        return f"Msg({self.src}->{self.dst}, r{self.round_num}, v{self.value})"


class Node:
    def __init__(self, node_id, initial_value, n_nodes, f_max=1):
        self.id = node_id
        self.initial_value = initial_value
        self.n_nodes = n_nodes
        self.f_max = f_max
        self.estimate = initial_value
        self.decided = None
        self.round_num = 1

    def propose_messages(self, round_num):
        if self.decided is not None:
            return []
        self.round_num = round_num
        msgs = []
        for dst in range(self.n_nodes):
            if dst != self.id:
                msgs.append(Message(self.id, dst, round_num, self.estimate))
        return msgs

    def receive_messages(self, messages, round_num):
        if self.decided is not None:
            return None
        values = defaultdict(int)
        for m in messages:
            if m.round_num == round_num:
                values[m.value] += 1
        quorum = self.n_nodes - self.f_max
        for val, count in values.items():
            if count >= quorum:
                self.decided = val
                self.estimate = val
                return val
        if values:
            best = max(values, key=values.get)
            if values[best] > self.f_max:
                self.estimate = best
        return None


class AdversaryScheduler:
    def __init__(self, nodes, crashed_node=None):
        self.nodes = nodes
        self.crashed_node = crashed_node
        self.pending = []
        self.round_num = 1
        self.max_rounds = 20
        self.decisions_logged = []
        self.steps = 0

    def step(self):
        self.steps += 1
        if self.round_num > self.max_rounds:
            return False
        alive = [n for n in self.nodes if n.id != self.crashed_node]
        for node in alive:
            msgs = node.propose_messages(self.round_num)
            self.pending.extend(msgs)
        if not self.pending:
            still_running = [n for n in alive if n.decided is None]
            if not still_running:
                return False
            self.round_num += 1
            return True
        best_msg = None
        best_valence = -1
        for msg in self.pending:
            valence = self._assess_valence(msg)
            if valence > best_valence:
                best_valence = valence
                best_msg = msg
        if best_msg is None:
            best_msg = self.pending[0]
        self.pending.remove(best_msg)
        dst_node = self.nodes[best_msg.dst]
        result = dst_node.receive_messages([best_msg], self.round_num)
        if result is not None:
            self.decisions_logged.append(
                (dst_node.id, result, self.round_num)
            )
        has_decided = all(n.decided is not None for n in alive)
        no_more_msgs = len(self.pending) == 0
        if no_more_msgs and not has_decided:
            undecided = [n for n in alive if n.decided is None]
            if not undecided:
                return False
            self.round_num += 1
        if has_decided:
            return False
        return True

    def _assess_valence(self, candidate_msg):
        dst = self.nodes[candidate_msg.dst]
        current_estimate = dst.estimate
        values = defaultdict(int)
        values[current_estimate] += 1
        for m in self.pending:
            if m.dst == candidate_msg.dst and m is not candidate_msg:
                values[m.value] += 1
        return len(values)

    def run(self):
        for _ in range(self.max_rounds * len(self.nodes) * len(self.nodes) * 2):
            if not self.step():
                break
        return self.decisions_logged


class BenOrNode:
    def __init__(self, node_id, initial_value, n_nodes, f_max=1):
        self.id = node_id
        self.initial_value = initial_value
        self.n_nodes = n_nodes
        self.f_max = f_max
        self.estimate = initial_value
        self.decided = None
        self.round_num = 1

    def phase1_send(self, round_num):
        if self.decided is not None:
            return []
        self.round_num = round_num
        msgs = []
        for dst in range(self.n_nodes):
            if dst != self.id:
                msgs.append(
                    Message(self.id, dst, round_num, self.estimate)
                )
        return msgs

    def phase1_receive(self, messages, round_num):
        if self.decided is not None:
            return ("decided", self.decided)
        values = defaultdict(int)
        values[self.estimate] += 1
        for m in messages:
            if m.round_num == round_num and m.src != self.id:
                values[m.value] += 1
        quorum = self.n_nodes - self.f_max
        for val, count in values.items():
            if count >= quorum:
                self.decided = val
                self.estimate = val
                return ("decided", val)
        if values:
            best = max(values, key=values.get)
            if values[best] > self.f_max:
                self.estimate = best
                return ("propose", best)
        return ("coin_flip_needed", None)

    def coin_flip(self):
        self.estimate = random.randint(0, 1)
        return self.estimate


class BenOrRandomizedConsensus:
    def __init__(self, n_nodes, initial_values, f_max=1, crashed_node=None):
        self.nodes = [
            BenOrNode(i, initial_values[i], n_nodes, f_max)
            for i in range(n_nodes)
        ]
        self.n_nodes = n_nodes
        self.f_max = f_max
        self.crashed_node = crashed_node
        self.max_rounds = 50

    def run(self):
        for rnd in range(1, self.max_rounds + 1):
            phase1_msgs = []
            for node in self.nodes:
                if node.id != self.crashed_node:
                    phase1_msgs.extend(node.phase1_send(rnd))
            alive = [
                n for n in self.nodes if n.id != self.crashed_node
            ]
            for dst_node in alive:
                incoming = [
                    m for m in phase1_msgs if m.dst == dst_node.id
                ]
                result = dst_node.phase1_receive(incoming, rnd)
                if result and result[0] == "coin_flip_needed":
                    dst_node.coin_flip()
            all_decided = all(
                n.decided is not None or n.id == self.crashed_node
                for n in self.nodes
            )
            if all_decided:
                decisions = {
                    n.id: n.decided
                    for n in self.nodes
                    if n.id != self.crashed_node
                }
                return decisions, rnd
        decisions = {}
        for n in self.nodes:
            if n.id != self.crashed_node:
                decisions[n.id] = n.decided
        return decisions, self.max_rounds


def run_deterministic_with_adversary(scenarios):
    print("=" * 70)
    print("DETERMINISTIC CONSENSUS UNDER ADVERSARY SCHEDULER")
    print("=" * 70)
    for name, initial_values in scenarios:
        print(f"\n--- Scenario: {name} ---")
        print(f"    Initial values: {initial_values}")
        n = len(initial_values)
        nodes = [Node(i, initial_values[i], n) for i in range(n)]
        scheduler = AdversaryScheduler(nodes, crashed_node=None)
        results = scheduler.run()
        decided_nodes = [
            (node.id, node.decided)
            for node in nodes
            if node.decided is not None
        ]
        undecided_nodes = [
            (node.id, node.estimate)
            for node in nodes
            if node.decided is None
        ]
        if undecided_nodes:
            print(f"    RESULT: NO CONSENSUS REACHED")
            print(
                f"    Decided: {decided_nodes if decided_nodes else 'none'}"
            )
            print(f"    Stuck (undecided): {undecided_nodes}")
            print(
                f"    Messages still pending: {len(scheduler.pending)}"
            )
        else:
            print(f"    RESULT: Consensus reached: {decided_nodes}")


def run_deterministic_with_crash(scenarios):
    print("\n" + "=" * 70)
    print("DETERMINISTIC CONSENSUS WITH CRASHED NODE")
    print("=" * 70)
    for name, initial_values in scenarios:
        print(f"\n--- Scenario: {name}, Node 2 crashed ---")
        print(f"    Initial values: {initial_values}")
        n = len(initial_values)
        nodes = [Node(i, initial_values[i], n) for i in range(n)]
        scheduler = AdversaryScheduler(nodes, crashed_node=2)
        results = scheduler.run()
        decided_nodes = [
            (node.id, node.decided)
            for node in nodes
            if node.id != 2 and node.decided is not None
        ]
        undecided_nodes = [
            (node.id, node.estimate)
            for node in nodes
            if node.id != 2 and node.decided is None
        ]
        if undecided_nodes:
            print(f"    RESULT: NO CONSENSUS REACHED (FLP!)")
            print(
                f"    Decided: {decided_nodes if decided_nodes else 'none'}"
            )
            print(f"    Stuck (undecided): {undecided_nodes}")
        else:
            print(
                f"    RESULT: Consensus reached despite crash: {decided_nodes}"
            )


def run_randomized_trials(scenarios, trials=30):
    print("\n" + "=" * 70)
    print("BEN-OR RANDOMIZED CONSENSUS")
    print("=" * 70)
    for name, initial_values in scenarios:
        print(f"\n--- Scenario: {name} ---")
        print(f"    Initial values: {initial_values}")
        round_counts = []
        agreements = 0
        value_distributions = defaultdict(int)
        for trial in range(trials):
            random.seed(trial + 1000)
            n = len(initial_values)
            protocol = BenOrRandomizedConsensus(n, initial_values)
            decisions, rounds = protocol.run()
            round_counts.append(rounds)
            alive_decisions = {
                k: v for k, v in decisions.items() if v is not None
            }
            if len(alive_decisions) == n:
                values = list(alive_decisions.values())
                if all(v == values[0] for v in values):
                    agreements += 1
                    value_distributions[values[0]] += 1
        avg = sum(round_counts) / len(round_counts)
        print(f"    Trials: {trials}")
        print(f"    Consensus reached: {agreements}/{trials}")
        print(f"    Avg rounds to decide: {avg:.1f}")
        print(
            f"    Min rounds: {min(round_counts)}, "
            f"Max rounds: {max(round_counts)}"
        )
        print(f"    Decided values: {dict(value_distributions)}")


def run_randomized_with_crash(scenarios, trials=30):
    print("\n" + "=" * 70)
    print("BEN-OR RANDOMIZED CONSENSUS (WITH CRASHED NODE)")
    print("=" * 70)
    for name, initial_values in scenarios:
        print(f"\n--- Scenario: {name}, Node 2 crashed ---")
        print(f"    Initial values: {initial_values}")
        round_counts = []
        agreements = 0
        for trial in range(trials):
            random.seed(trial + 2000)
            n = len(initial_values)
            protocol = BenOrRandomizedConsensus(
                n, initial_values, crashed_node=2
            )
            decisions, rounds = protocol.run()
            round_counts.append(rounds)
            alive_decisions = {
                k: v for k, v in decisions.items() if v is not None
            }
            if len(alive_decisions) == n - 1:
                values = list(alive_decisions.values())
                if all(v == values[0] for v in values):
                    agreements += 1
        avg = sum(round_counts) / len(round_counts)
        print(f"    Trials: {trials}")
        print(f"    Consensus reached: {agreements}/{trials}")
        print(f"    Avg rounds to decide: {avg:.1f}")
        print(
            f"    Min rounds: {min(round_counts)}, "
            f"Max rounds: {max(round_counts)}"
        )


def main():
    random.seed(42)
    scenarios = [
        ("Split (0,1,0)", [0, 1, 0]),
        ("Split (1,1,0)", [1, 1, 0]),
        ("All same (0,0,0)", [0, 0, 0]),
        ("One vs two (1,0,0)", [1, 0, 0]),
    ]
    run_deterministic_with_adversary(scenarios[:3])
    run_deterministic_with_crash(scenarios[:3])
    print("\n\n" + "!" * 70)
    print("FLP IMPOSSIBILITY RESULT: The adversary scheduler can ALWAYS")
    print("prevent consensus in a deterministic, fully asynchronous protocol")
    print("with even ONE potential crash failure.")
    print("!" * 70)
    run_randomized_trials(scenarios, trials=30)
    run_randomized_with_crash(scenarios, trials=30)
    print("\n" + "*" * 70)
    print("RANDOMIZATION BREAKS FLP: Coin flips deny the adversary")
    print("foreknowledge. Consensus succeeds with probability 1.")
    print()
    print("Key insights:")
    print("  1. Deterministic + async + 1 crash = NO consensus (FLP)")
    print("  2. Randomized + async + crash = consensus with probability 1")
    print("  3. Partially synchronous + deterministic = consensus after GST")
    print("  4. Real systems use timeouts (failure detectors) + randomization")
    print("*" * 70)


if __name__ == "__main__":
    main()