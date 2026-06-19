"""Software project management: estimation, scheduling, Brooks's Law."""
import math


def three_point_estimate(optimistic: float, most_likely: float, pessimistic: float) -> dict:
    """PERT three-point estimation."""
    expected = (optimistic + 4 * most_likely + pessimistic) / 6
    std_dev = (pessimistic - optimistic) / 6
    variance = std_dev ** 2
    return {
        'expected': round(expected, 2),
        'std_dev': round(std_dev, 2),
        'variance': round(variance, 2),
        'range_68': (round(expected - std_dev, 1), round(expected + std_dev, 1)),
        'range_95': (round(expected - 2 * std_dev, 1), round(expected + 2 * std_dev, 1)),
    }


def planning_poker_round(estimates: list[int]) -> dict:
    """Analyze a planning poker round."""
    from collections import Counter
    counts = Counter(estimates)
    median = sorted(estimates)[len(estimates) // 2]
    outliers = [e for e in estimates if abs(e - median) > median * 0.5]
    converged = len(set(estimates)) == 1
    return {
        'estimates': estimates,
        'median': median,
        'outliers': outliers,
        'converged': converged,
    }


def communication_overhead(n_people: int) -> int:
    """Number of communication channels: n(n-1)/2."""
    return n_people * (n_people - 1) // 2


def brooks_law_impact(original_team: int, new_members: int, ramp_up_weeks: int = 4) -> dict:
    """Estimate impact of adding people to a late project."""
    original_channels = communication_overhead(original_team)
    new_channels = communication_overhead(original_team + new_members)
    overhead_increase = new_channels - original_channels

    return {
        'original_team': original_team,
        'new_team': original_team + new_members,
        'original_channels': original_channels,
        'new_channels': new_channels,
        'overhead_increase': overhead_increase,
        'ramp_up_weeks': ramp_up_weeks,
        'warning': 'Adding people may slow the project due to communication overhead and ramp-up time',
    }


def critical_path(tasks: list[dict]) -> dict:
    """Find critical path in a task graph. tasks: [{name, duration, deps}]."""
    completed = {}
    schedule = []

    remaining = tasks.copy()
    while remaining:
        ready = [t for t in remaining if all(d in completed for d in t.get('deps', []))]
        if not ready:
            raise ValueError("Circular dependency!")
        for task in ready:
            start = max((completed[d] for d in task.get('deps', [])), default=0)
            end = start + task['duration']
            completed[task['name']] = end
            schedule.append({'name': task['name'], 'start': start, 'end': end, 'duration': task['duration']})
            remaining.remove(task)

    total = max(completed.values())
    # Find critical path (longest dependency chain)
    critical = [s for s in schedule if s['end'] == total or any(
        s['start'] == max((completed[d] for d in next(t for t in tasks if t['name'] == s['name']).get('deps', [])), default=0)
        for _ in [None]
    )]

    return {'schedule': schedule, 'total_days': total, 'critical_path_end': total}


if __name__ == "__main__":
    # Three-point estimation
    print("=== Three-Point Estimation ===")
    est = three_point_estimate(3, 5, 12)
    print(f"Task: Build auth API")
    print(f"  Expected: {est['expected']} days")
    print(f"  68% range: {est['range_68']} days")
    print(f"  95% range: {est['range_95']} days")

    # Communication overhead
    print("\n=== Brooks's Law ===")
    for n in [3, 5, 8, 12]:
        print(f"  {n} people: {communication_overhead(n)} channels")

    impact = brooks_law_impact(5, 3)
    print(f"\n  Adding 3 to team of 5:")
    print(f"    Channels: {impact['original_channels']} → {impact['new_channels']} (+{impact['overhead_increase']})")
    print(f"    Warning: {impact['warning']}")

    # Critical path
    print("\n=== Critical Path ===")
    tasks = [
        {'name': 'Schema', 'duration': 2, 'deps': []},
        {'name': 'API', 'duration': 5, 'deps': ['Schema']},
        {'name': 'Auth', 'duration': 3, 'deps': ['Schema']},
        {'name': 'Frontend', 'duration': 4, 'deps': ['API']},
        {'name': 'Tests', 'duration': 2, 'deps': ['API', 'Auth']},
    ]
    result = critical_path(tasks)
    print(f"  Total duration: {result['total_days']} days")
    for t in result['schedule']:
        print(f"    Day {t['start']:2.0f}-{t['end']:2.0f}: {t['name']} ({t['duration']}d)")
