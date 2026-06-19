#!/usr/bin/env python3
from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class RiskProfile:
    logic: float
    boundary: float
    journey: float
    change_frequency: float
    runtime_budget_minutes: float


@dataclass(frozen=True)
class Allocation:
    unit: int
    integration: int
    e2e: int


def clamp01(value: float) -> float:
    return max(0.0, min(1.0, value))


def normalized(allocation: Allocation) -> tuple[float, float, float]:
    total = allocation.unit + allocation.integration + allocation.e2e
    if total <= 0:
        raise ValueError("allocation total must be positive")
    return (
        allocation.unit / total,
        allocation.integration / total,
        allocation.e2e / total,
    )


def utility(profile: RiskProfile, allocation: Allocation) -> float:
    u, i, e = normalized(allocation)
    coverage = (
        profile.logic * (0.9 * u + 0.2 * i + 0.1 * e)
        + profile.boundary * (0.2 * u + 0.9 * i + 0.4 * e)
        + profile.journey * (0.1 * u + 0.4 * i + 1.0 * e)
    )

    runtime_penalty = 0.8 * u + 2.2 * i + 5.0 * e
    flake_penalty = profile.change_frequency * (0.03 * u + 0.08 * i + 0.22 * e)
    budget_pressure = max(0.0, runtime_penalty - profile.runtime_budget_minutes / 4.0)

    return coverage - 0.10 * runtime_penalty - 1.3 * flake_penalty - 0.20 * budget_pressure


def recommend(profile: RiskProfile) -> Allocation:
    candidates = [
        Allocation(70, 20, 10),
        Allocation(60, 30, 10),
        Allocation(55, 35, 10),
        Allocation(50, 35, 15),
        Allocation(45, 40, 15),
        Allocation(40, 45, 15),
    ]
    best = max(candidates, key=lambda alloc: utility(profile, alloc))
    return best


def scenario(name: str, profile: RiskProfile) -> None:
    alloc = recommend(profile)
    score = utility(profile, alloc)
    u, i, e = normalized(alloc)
    print(f"Scenario: {name}")
    print(f"  Recommended split: unit={alloc.unit} integration={alloc.integration} e2e={alloc.e2e}")
    print(f"  Normalized: unit={u:.2%}, integration={i:.2%}, e2e={e:.2%}")
    print(f"  Utility score: {score:.4f}")
    print("  Monitor: flake_rate_by_level, escaped_defects_by_root_cause, ci_p95_minutes")


def main() -> None:
    checkout = RiskProfile(
        logic=clamp01(0.70),
        boundary=clamp01(0.90),
        journey=clamp01(0.80),
        change_frequency=clamp01(0.60),
        runtime_budget_minutes=12.0,
    )

    data_pipeline = RiskProfile(
        logic=0.85,
        boundary=0.55,
        journey=0.35,
        change_frequency=0.45,
        runtime_budget_minutes=10.0,
    )

    ui_app = RiskProfile(
        logic=0.45,
        boundary=0.70,
        journey=0.95,
        change_frequency=0.75,
        runtime_budget_minutes=16.0,
    )

    scenario("checkout", checkout)
    print()
    scenario("data-pipeline", data_pipeline)
    print()
    scenario("ui-app", ui_app)


if __name__ == "__main__":
    main()
