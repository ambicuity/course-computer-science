"""HCI design principles: Fitts's Law, Hick's Law, heuristic evaluation."""
import math


def fitts_time(distance: float, width: float, a: float = 0.05, b: float = 0.15) -> float:
    """Fitts's Law: T = a + b * log2(D/W + 1)."""
    return a + b * math.log2(distance / width + 1)


def hick_time(n_choices: int, b: float = 0.155) -> float:
    """Hick's Law: T = b * log2(n + 1)."""
    return b * math.log2(n_choices + 1)


def heuristic_score(violations: list[dict]) -> dict:
    """Score a heuristic evaluation. Violations: [{heuristic, severity}]."""
    weights = {'cosmetic': 0.1, 'minor': 0.5, 'major': 1.0, 'catastrophic': 3.0}
    total_penalty = sum(weights.get(v['severity'], 0) for v in violations)
    score = max(0, 10 - total_penalty)
    by_severity = {}
    for v in violations:
        s = v['severity']
        by_severity[s] = by_severity.get(s, 0) + 1
    return {'score': round(score, 1), 'violations': by_severity, 'total': len(violations)}


def progressive_disclosure(items: list[dict], show_advanced: bool = False) -> list[str]:
    """Filter menu items by frequency for progressive disclosure."""
    if show_advanced:
        return [i['label'] for i in items]
    return [i['label'] for i in items if i.get('frequency') == 'common']


if __name__ == "__main__":
    # Fitts's Law
    print("=== Fitts's Law ===")
    print(f"Wide button (100px):  {fitts_time(500, 100):.3f}s")
    print(f"Narrow button (50px): {fitts_time(500, 50):.3f}s")
    print(f"Edge target (inf):    {fitts_time(500, 1000):.3f}s")

    # Hick's Law
    print("\n=== Hick's Law ===")
    for n in [2, 5, 10, 25, 50]:
        print(f"  {n:2d} choices: {hick_time(n):.3f}s")

    # Heuristic Evaluation
    print("\n=== Heuristic Evaluation ===")
    violations = [
        {'heuristic': 1, 'severity': 'major', 'description': 'No loading indicator'},
        {'heuristic': 9, 'severity': 'major', 'description': 'Generic error message'},
        {'heuristic': 3, 'severity': 'catastrophic', 'description': 'No undo for delete'},
        {'heuristic': 5, 'severity': 'minor', 'description': 'No confirmation dialog'},
    ]
    result = heuristic_score(violations)
    print(f"  Score: {result['score']}/10")
    print(f"  Violations: {result['violations']}")

    # Progressive Disclosure
    print("\n=== Progressive Disclosure ===")
    menu = [
        {'label': 'Save', 'frequency': 'common'},
        {'label': 'Save As...', 'frequency': 'occasional'},
        {'label': 'Export as PDF', 'frequency': 'rare'},
        {'label': 'Print', 'frequency': 'occasional'},
        {'label': 'Close', 'frequency': 'common'},
    ]
    print(f"  Basic:    {progressive_disclosure(menu, False)}")
    print(f"  Advanced: {progressive_disclosure(menu, True)}")
