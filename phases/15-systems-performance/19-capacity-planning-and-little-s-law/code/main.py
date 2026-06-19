"""
Capacity Planning and Little's Law
Phase 15 — Systems Programming & Performance

Implements:
- Little's Law calculator
- M/M/1 queueing model
- M/M/c queueing model (with Erlang C)
- Connection pool sizer
- Thread pool sizer
- Latency vs utilization curve generator
"""

import math
from dataclasses import dataclass


# ---------------------------------------------------------------------------
# Little's Law
# ---------------------------------------------------------------------------

def littles_law(arrival_rate: float, avg_time_in_system: float) -> float:
    """L = lambda * W. Returns average number of items in the system."""
    if arrival_rate < 0 or avg_time_in_system < 0:
        raise ValueError("Arrival rate and time must be non-negative")
    return arrival_rate * avg_time_in_system


def arrival_rate_from_littles(l: float, avg_time_in_system: float) -> float:
    """lambda = L / W. Returns arrival rate."""
    if avg_time_in_system <= 0:
        raise ValueError("Time in system must be positive")
    return l / avg_time_in_system


def avg_time_from_littles(l: float, arrival_rate: float) -> float:
    """W = L / lambda. Returns average time in system."""
    if arrival_rate <= 0:
        raise ValueError("Arrival rate must be positive")
    return l / arrival_rate


# ---------------------------------------------------------------------------
# M/M/1 Queueing Model
# ---------------------------------------------------------------------------

@dataclass
class MM1Results:
    utilization: float
    avg_in_system: float
    avg_in_queue: float
    avg_time_in_system: float
    avg_wait_in_queue: float
    prob_empty: float


def mm1(arrival_rate: float, service_rate: float) -> MM1Results:
    """Compute M/M/1 queueing metrics."""
    if arrival_rate >= service_rate:
        raise ValueError(
            f"Unstable: arrival rate ({arrival_rate}) >= service rate ({service_rate})"
        )
    rho = arrival_rate / service_rate
    return MM1Results(
        utilization=rho,
        avg_in_system=rho / (1 - rho),
        avg_in_queue=rho ** 2 / (1 - rho),
        avg_time_in_system=1 / (service_rate - arrival_rate),
        avg_wait_in_queue=rho / (service_rate - arrival_rate),
        prob_empty=1 - rho,
    )


# ---------------------------------------------------------------------------
# M/M/c Queueing Model (Erlang C)
# ---------------------------------------------------------------------------

def erlang_c(num_servers: int, traffic_intensity: float) -> float:
    """
    Compute Erlang C: probability an arrival must wait.

    traffic_intensity = arrival_rate / (num_servers * service_rate) = rho per server.
    """
    if num_servers < 1:
        raise ValueError("Must have at least 1 server")
    if traffic_intensity >= 1:
        raise ValueError("System unstable: per-server utilization >= 1")

    a = num_servers * traffic_intensity  # total offered load in Erlangs
    # Sum for k = 0..c-1 of a^k / k!
    denom_sum = sum(a ** k / math.factorial(k) for k in range(num_servers))
    last_term = a ** num_servers / (math.factorial(num_servers) * (1 - traffic_intensity))
    return last_term / (denom_sum + last_term)


@dataclass
class MMcResults:
    num_servers: int
    utilization_per_server: float
    system_utilization: float
    prob_wait: float
    avg_in_queue: float
    avg_wait_in_queue: float
    avg_in_system: float
    avg_time_in_system: float


def mmc(arrival_rate: float, service_rate: float, num_servers: int) -> MMcResults:
    """Compute M/M/c queueing metrics."""
    rho = arrival_rate / (num_servers * service_rate)
    if rho >= 1:
        raise ValueError("Unstable: system utilization >= 1")

    p_wait = erlang_c(num_servers, rho)
    avg_in_queue = p_wait * rho / (1 - rho)
    service_time = 1 / service_rate
    avg_wait_in_queue = avg_in_queue / arrival_rate if arrival_rate > 0 else 0
    avg_time_in_system = avg_wait_in_queue + service_time
    avg_in_system = arrival_rate * avg_time_in_system

    return MMcResults(
        num_servers=num_servers,
        utilization_per_server=rho,
        system_utilization=rho * num_servers,
        prob_wait=p_wait,
        avg_in_queue=avg_in_queue,
        avg_wait_in_queue=avg_wait_in_queue,
        avg_in_system=avg_in_system,
        avg_time_in_system=avg_time_in_system,
    )


# ---------------------------------------------------------------------------
# Connection Pool Sizer
# ---------------------------------------------------------------------------

def size_connection_pool(
    peak_qps: float,
    avg_query_time_sec: float,
    headroom_pct: float = 25.0,
) -> dict:
    """Size a database connection pool using Little's Law."""
    base_pool = littles_law(peak_qps, avg_query_time_sec)
    pool_with_headroom = math.ceil(base_pool * (1 + headroom_pct / 100))
    return {
        "peak_qps": peak_qps,
        "avg_query_time_ms": avg_query_time_sec * 1000,
        "base_pool_size": math.ceil(base_pool),
        "headroom_pct": headroom_pct,
        "recommended_pool_size": pool_with_headroom,
    }


# ---------------------------------------------------------------------------
# Thread Pool Sizer
# ---------------------------------------------------------------------------

def size_thread_pool(
    num_cores: int,
    cpu_time_ms: float,
    io_time_ms: float,
    target_utilization: float = 0.7,
) -> dict:
    """
    Size a thread pool.
    CPU-bound:  threads = num_cores
    I/O-bound:  threads = num_cores * (1 + io_time / cpu_time)
    """
    is_cpu_bound = io_time_ms < cpu_time_ms * 0.1

    if is_cpu_bound:
        optimal_threads = num_cores
        formula = "N_cores (CPU-bound)"
    else:
        optimal_threads = math.ceil(num_cores * (1 + io_time_ms / cpu_time_ms))
        formula = "N_cores × (1 + W_io / C_cpu)"

    max_throughput = optimal_threads / ((cpu_time_ms + io_time_ms) / 1000)

    return {
        "num_cores": num_cores,
        "cpu_time_ms": cpu_time_ms,
        "io_time_ms": io_time_ms,
        "is_cpu_bound": is_cpu_bound,
        "formula_used": formula,
        "optimal_threads": optimal_threads,
        "max_throughput_rps": round(max_throughput, 1),
        "target_utilization": target_utilization,
        "recommended_threads_at_target": math.ceil(optimal_threads / target_utilization),
    }


# ---------------------------------------------------------------------------
# Latency vs Utilization Curve
# ---------------------------------------------------------------------------

def latency_curve(
    service_rate: float,
    util_range: tuple = (0.1, 0.99),
    num_points: int = 10,
) -> list[dict]:
    """Generate latency multiplier vs utilization for an M/M/1 queue."""
    step = (util_range[1] - util_range[0]) / (num_points - 1)
    results = []
    for i in range(num_points):
        rho = util_range[0] + step * i
        if rho >= 1.0:
            break
        latency_multiplier = 1 / (1 - rho)
        results.append({
            "utilization": round(rho, 3),
            "latency_multiplier": round(latency_multiplier, 2),
            "avg_time_in_system": round(1 / (service_rate * (1 - rho)), 6),
        })
    return results


# ---------------------------------------------------------------------------
# Print Helpers
# ---------------------------------------------------------------------------

def print_separator(title: str = "") -> None:
    print()
    print("=" * 60)
    if title:
        print(f"  {title}")
        print("=" * 60)
    print()


def print_table(headers: list[str], rows: list[list], col_widths: list[int] | None = None) -> None:
    if col_widths is None:
        col_widths = [max(len(headers[i]), *(len(str(r[i])) for r in rows)) + 2 for i in range(len(headers))]
    header_line = "".join(h.ljust(w) for h, w in zip(headers, col_widths))
    print(header_line)
    print("-" * len(header_line))
    for row in rows:
        print("".join(str(v).ljust(w) for v, w in zip(row, col_widths)))


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    print_separator("CAPACITY PLANNING & LITTLE'S LAW")

    # --- Little's Law ---
    print("--- Little's Law: L = lambda * W ---")
    examples = [
        ("Web server", 200, 0.05),
        ("DB queries", 500, 0.02),
        ("API gateway", 1000, 0.01),
    ]
    rows = []
    for name, lam, w in examples:
        l = littles_law(lam, w)
        rows.append([name, str(lam), f"{w*1000:.0f} ms", f"{l:.1f}"])
    print_table(
        ["Scenario", "λ (req/s)", "W", "L (in system)"],
        rows,
    )

    # --- Reverse Little's Law ---
    print()
    print("--- Reverse: given L and λ, find W ---")
    l, lam = 25, 500
    w = avg_time_from_littles(l, lam)
    print(f"  L={l}, λ={lam}/s → W = {w*1000:.1f} ms")

    # --- M/M/1 ---
    print_separator("M/M/1 QUEUE RESULTS")
    mm1_cases = [
        ("Low load", 50, 100),
        ("Moderate", 70, 100),
        ("High", 90, 100),
        ("Very high", 95, 100),
    ]
    rows = []
    for name, lam, mu in mm1_cases:
        r = mm1(lam, mu)
        rows.append([
            name,
            f"{r.utilization:.0%}",
            f"{r.avg_in_system:.2f}",
            f"{r.avg_in_queue:.2f}",
            f"{r.avg_time_in_system*1000:.1f} ms",
            f"{r.avg_wait_in_queue*1000:.1f} ms",
            f"{r.prob_empty:.2f}",
        ])
    print_table(
        ["Case", "Util", "L_sys", "L_q", "W_sys", "W_q", "P(empty)"],
        rows,
    )

    # --- M/M/c ---
    print_separator("M/M/c QUEUE — ERLANG C")
    lam, mu = 80, 20
    for c in [5, 6, 7, 8]:
        r = mmc(lam, mu, c)
        print(f"  c={c}: util/server={r.utilization_per_server:.1%}, "
              f"P(wait)={r.prob_wait:.3f}, L_q={r.avg_in_queue:.2f}, "
              f"W_sys={r.avg_time_in_system*1000:.1f} ms")

    # --- Connection Pool ---
    print_separator("CONNECTION POOL SIZING")
    pool = size_connection_pool(peak_qps=500, avg_query_time_sec=0.02, headroom_pct=25)
    for k, v in pool.items():
        print(f"  {k}: {v}")

    # --- Thread Pool ---
    print_separator("THREAD POOL SIZING")
    print("  CPU-bound task (8 cores, 10ms CPU, 1ms I/O):")
    tp_cpu = size_thread_pool(num_cores=8, cpu_time_ms=10, io_time_ms=1)
    for k, v in tp_cpu.items():
        print(f"    {k}: {v}")
    print()
    print("  I/O-bound task (8 cores, 5ms CPU, 45ms I/O):")
    tp_io = size_thread_pool(num_cores=8, cpu_time_ms=5, io_time_ms=45)
    for k, v in tp_io.items():
        print(f"    {k}: {v}")

    # --- Latency vs Utilization ---
    print_separator("LATENCY vs UTILIZATION (M/M/1)")
    curve = latency_curve(service_rate=100, util_range=(0.1, 0.99), num_points=10)
    rows = []
    for pt in curve:
        rows.append([
            f"{pt['utilization']:.0%}",
            f"{pt['latency_multiplier']:.1f}x",
            f"{pt['avg_time_in_system']*1000:.2f} ms",
        ])
    print_table(
        ["Utilization", "Latency ×idle", "Avg Time in System"],
        rows,
    )

    # --- Key Insight ---
    print()
    print("KEY INSIGHT:")
    print("  At 70% utilization, avg latency = 3.3× idle service time")
    print("  At 90% utilization, avg latency = 10× idle service time")
    print("  At 99% utilization, avg latency = 100× idle service time")
    print()
    print("  → Target 50-70% for steady-state, 80% max before scaling")


if __name__ == "__main__":
    main()