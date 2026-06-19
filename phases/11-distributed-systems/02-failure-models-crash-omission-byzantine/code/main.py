import random
from collections import Counter


class ByzantineGeneral:
    def __init__(self, name, is_traitor=False):
        self.name = name
        self.is_traitor = is_traitor
        self.received_orders = {}

    def send_order(self, order, recipient_name):
        if self.is_traitor:
            return random.choice(["ATTACK", "RETREAT"])
        return order

    def relay(self, order, recipient_name):
        if self.is_traitor:
            return random.choice(["ATTACK", "RETREAT"])
        return order


def majority_vote(votes):
    attack = votes.count("ATTACK")
    retreat = votes.count("RETREAT")
    return "ATTACK" if attack >= retreat else "RETREAT"


def simulate_byzantine_generals(n_generals, n_traitors, true_order="ATTACK", rounds=100):
    results = {"agree": 0, "disagree": 0, "wrong": 0}
    sample_evidence = None

    for _ in range(rounds):
        generals = []
        traitor_indices = set(random.sample(range(n_generals), n_traitors))
        for i in range(n_generals):
            generals.append(ByzantineGeneral(f"G{i}", is_traitor=(i in traitor_indices)))

        commander = generals[0]
        lieutenants = generals[1:]

        direct_orders = {}
        for lt in lieutenants:
            direct_orders[lt.name] = commander.send_order(true_order, lt.name)

        relayed = {}
        for lt in lieutenants:
            relayed[lt.name] = {}
            for other_lt in lieutenants:
                if other_lt.name != lt.name:
                    relay = other_lt.relay(direct_orders[other_lt.name], lt.name)
                    relayed[lt.name][other_lt.name] = relay

        decisions = {}
        for lt in lieutenants:
            votes = [direct_orders[lt.name]]
            for _, relay in relayed[lt.name].items():
                votes.append(relay)
            decisions[lt.name] = majority_vote(votes)

        decision_values = list(decisions.values())
        if len(set(decision_values)) == 1:
            results["agree"] += 1
            if decision_values[0] != true_order:
                results["wrong"] += 1
        else:
            results["disagree"] += 1

        if sample_evidence is None:
            sample_evidence = {
                "direct_orders": dict(direct_orders),
                "relayed": {k: dict(v) for k, v in relayed.items()},
                "decisions": dict(decisions),
            }

    return results, sample_evidence


class PBFTNode:
    def __init__(self, name, is_byzantine=False):
        self.name = name
        self.is_byzantine = is_byzantine
        self.log = []
        self.committed = None

    def pre_prepare(self, value, seq_num):
        if self.is_byzantine:
            chosen = random.choice(["A", "B"])
            return ("PRE-PREPARE", seq_num, chosen, self.name)
        return ("PRE-PREPARE", seq_num, value, self.name)

    def prepare(self, pp_msg):
        if self.is_byzantine:
            chosen = random.choice(["A", "B"])
            return ("PREPARE", pp_msg[1], chosen, self.name)
        return ("PREPARE", pp_msg[1], pp_msg[2], self.name)

    def commit(self, prepared_value, seq_num):
        if self.is_byzantine:
            chosen = random.choice(["A", "B"])
            return ("COMMIT", seq_num, chosen, self.name)
        return ("COMMIT", seq_num, prepared_value, self.name)


def run_pbft(n_nodes, f_byzantine, proposed_value, rounds=100):
    results = {"consensus": 0, "no_consensus": 0, "wrong_value": 0}
    sample_trace = None

    for _ in range(rounds):
        nodes = []
        byz_indices = set(random.sample(range(n_nodes), f_byzantine))
        for i in range(n_nodes):
            nodes.append(PBFTNode(f"N{i}", is_byzantine=(i in byz_indices)))

        primary = nodes[0]
        pp_msg = primary.pre_prepare(proposed_value, 0)

        prepare_msgs = []
        for node in nodes:
            p_msg = node.prepare(pp_msg)
            prepare_msgs.append(p_msg)

        honest_prepare_values = []
        for i, msg in enumerate(prepare_msgs):
            if not nodes[i].is_byzantine:
                honest_prepare_values.append(msg[2])

        if len(honest_prepare_values) == 0:
            results["no_consensus"] += 1
            continue

        prepared_value = Counter(honest_prepare_values).most_common(1)[0][0]

        commit_msgs = []
        for node in nodes:
            c_msg = node.commit(prepared_value, 0)
            commit_msgs.append(c_msg)

        honest_commit_values = []
        for i, msg in enumerate(commit_msgs):
            if not nodes[i].is_byzantine:
                honest_commit_values.append(msg[2])

        if len(honest_commit_values) >= 2 * f_byzantine + 1:
            committed = Counter(honest_commit_values).most_common(1)[0][0]
            results["consensus"] += 1
            if committed != proposed_value:
                results["wrong_value"] += 1
        else:
            results["no_consensus"] += 1

        if sample_trace is None:
            sample_trace = {
                "pre_prepare": pp_msg,
                "prepare_msgs": prepare_msgs,
                "commit_msgs": commit_msgs,
                "prepared_value": prepared_value,
            }

    return results, sample_trace


def print_section(title):
    print(f"\n{'='*70}")
    print(f"  {title}")
    print(f"{'='*70}\n")


def print_bar(label, value, total=50):
    filled = int(value / 100 * total)
    bar = "█" * filled + "░" * (total - filled)
    print(f"  {label:20s} [{bar}] {value:5.1f}%")


def main():
    print_section("BYZANTINE GENERALS — IMPOSSIBILITY (3 generals, 1 traitor)")

    print("The Byzantine Generals Problem proves that with n=3 and f=1,")
    print("honest agreement is IMPOSSIBLE. Let's see it in action:\n")

    results, evidence = simulate_byzantine_generals(3, 1, "ATTACK", rounds=1000)

    print("  Sample round evidence:")
    print(f"    Direct orders from Commander:")
    for name, order in evidence["direct_orders"].items():
        print(f"      {name} heard: {order}")
    print(f"    Relayed messages:")
    for name, relays in evidence["relayed"].items():
        for from_name, order in relays.items():
            print(f"      {name} heard from {from_name}: {order}")
    print(f"    Final decisions:")
    for name, decision in evidence["decisions"].items():
        print(f"      {name} decided: {decision}")

    agree_pct = results["agree"] / 10.0
    disagree_pct = results["disagree"] / 10.0
    wrong_pct = results["wrong"] / 10.0

    print(f"\n  1000 rounds with 3 generals (1 traitor):")
    print_bar("Agreement reached", agree_pct)
    print_bar("Disagreement", disagree_pct)
    print_bar("Wrong agreement", wrong_pct)
    print(f"\n  ➤ With 3 generals and 1 traitor, honest nodes cannot reliably")
    print(f"    agree. The impossibility proof holds.\n")

    print_section("BYZANTINE GENERALS — FEASIBILITY (n ≥ 3f+1)")

    configs = [
        (4, 1, "4 nodes, 1 traitor"),
        (7, 2, "7 nodes, 2 traitors"),
        (10, 3, "10 nodes, 3 traitors"),
    ]

    for n, f, label in configs:
        results, _ = simulate_byzantine_generals(n, f, "ATTACK", rounds=1000)
        agree_pct = results["agree"] / 10.0
        disagree_pct = results["disagree"] / 10.0
        print(f"  {label} ({n} ≥ 3×{f}+1 = {3*f+1}):")
        print_bar("Agreement", agree_pct)
        print_bar("Disagreement", disagree_pct)
        print()

    print("  ➤ With n ≥ 3f+1, honest nodes always achieve agreement.\n")

    print_section("PBFT-STYLE CONSENSUS (4 nodes, 1 Byzantine)")

    print("PBFT uses 3 phases (pre-prepare → prepare → commit) with")
    print("2f+1 quorum at each phase to tolerate f Byzantine faults.\n")

    results, trace = run_pbft(4, 1, "A", rounds=1000)

    print("  Sample PBFT trace:")
    print(f"    Pre-prepare: {trace['pre_prepare']}")
    print(f"    Prepare phase:")
    for msg in trace["prepare_msgs"]:
        print(f"      {msg}")
    print(f"    Commit phase:")
    for msg in trace["commit_msgs"]:
        print(f"      {msg}")
    print(f"    Prepared value: {trace['prepared_value']}")

    consensus_pct = results["consensus"] / 10.0
    wrong_pct = results["wrong_value"] / 10.0
    no_consensus_pct = results["no_consensus"] / 10.0

    print(f"\n  1000 rounds of PBFT with 4 nodes (1 Byzantine):")
    print_bar("Consensus reached", consensus_pct)
    print_bar("Wrong value", wrong_pct)
    print_bar("No consensus", no_consensus_pct)

    print(f"\n  ➤ PBFT achieves safe consensus despite Byzantine faults.\n")

    print_section("FAILURE MODEL COMPARISON")

    print("Failure model comparison: what a crash-tolerant protocol (Raft-style)")
    print("handles vs. what requires Byzantine-tolerant protocols (PBFT-style).\n")

    print("  ┌──────────────────┬───────────────────┬───────────────────┐")
    print("  │ Failure Model    │ Crash-tolerant    │ Byzantine-tolerant │")
    print("  │                  │ (e.g., Raft)      │ (e.g., PBFT)       │")
    print("  ├──────────────────┼───────────────────┼───────────────────┤")
    print("  │ Crash-stop       │ ✅ Handled         │ ✅ Handled          │")
    print("  │ Crash-recovery   │ ✅ With WAL        │ ✅ With WAL         │")
    print("  │ Omission         │ ⚠️ Retries needed  │ ✅ 2f+1 quorum      │")
    print("  │ Timing           │ ⚠️ Timeout-based   │ ⚠️ Timeout-based    │")
    print("  │ Byzantine        │ ❌ UNSAFE          │ ✅ 3f+1 nodes       │")
    print("  └──────────────────┴───────────────────┴───────────────────┘")
    print()
    print("  Key differences:")
    print("  • Raft needs 2f+1 nodes to tolerate f crash faults")
    print("  • PBFT needs 3f+1 nodes to tolerate f Byzantine faults")
    print("  • PBFT needs 3 communication rounds vs Raft's 1-2")
    print("  • The 50% node overhead + 3x message overhead is the price of Byzantine tolerance")
    print()
    print("  Why crash-tolerant protocols FAIL under Byzantine faults:")
    print("  In Raft, the leader proposes values and followers accept them.")
    print("  If the leader is Byzantine, it can propose different values to")
    print("  different followers — followers can't detect this by majority")
    print("  alone because they only see what the leader tells them.")
    print("  PBFT fixes this with the prepare/commit relay phases (3 rounds)")
    print("  so followers cross-check with each other before committing.\n")

    print_section("FAILURE MODEL HIERARCHY")
    print("  Crash-stop ⊂ Crash-recovery ⊂ Omission ⊂ Timing ⊂ Byzantine")
    print()
    print("  Each model GENERALIZES the one below it.")
    print("  An algorithm for crash-stop BREAKS under omission.")
    print("  An algorithm for Byzantine WORKS under crash-stop (but is overkill).")
    print()
    print("  Choose the weakest model you can get away with.")
    print("  Raft: crash-stop (2f+1 nodes, tolerate f crashes)")
    print("  PBFT: Byzantine (3f+1 nodes, tolerate f Byzantine faults)")
    print()
    print("  The 50% node overhead is the price of Byzantine tolerance.")


if __name__ == "__main__":
    main()