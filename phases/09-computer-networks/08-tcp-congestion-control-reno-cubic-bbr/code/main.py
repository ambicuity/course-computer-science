"""
TCP Congestion Control — Reno, CUBIC, BBR Simulator
Phase 09 — Computer Networks

Simulates three congestion control algorithms under identical network
conditions and prints/plots cwnd over time.

Run: python3 main.py
"""

import math
import random
from dataclasses import dataclass, field
from typing import List, Tuple


# ── Network Model ───────────────────────────────────────────────────

@dataclass
class NetworkParams:
    """Simulated network characteristics."""
    bandwidth_mbps: float = 100.0      # Bottleneck bandwidth
    rtt_ms: float = 50.0               # Base round-trip time
    buffer_packets: int = 64           # Router buffer in packets
    loss_rate: float = 0.0             # Random loss probability
    mss: int = 1460                    # Maximum segment size
    sim_rounds: int = 200              # Number of RTT rounds to simulate


# ── Reno ─────────────────────────────────────────────────────────────

class Reno:
    """TCP Reno: slow start, congestion avoidance, fast recovery."""

    def __init__(self, mss: int = 1460):
        self.mss = mss
        self.cwnd = 1.0 * mss
        self.ssthresh = 64 * mss
        self.duplicate_acks = 0
        self.in_recovery = False
        self.history: List[float] = []

    def on_ack(self):
        self.history.append(self.cwnd / self.mss)

        if self.in_recovery:
            self.cwnd += self.mss  # inflate during recovery
            return

        if self.cwnd < self.ssthresh:
            # Slow start: double per RTT (per-ACK adds 1 MSS)
            self.cwnd += self.mss
        else:
            # Congestion avoidance: add MSS^2 / cwnd per ACK
            self.cwnd += self.mss * self.mss / self.cwnd

    def on_loss(self, loss_type: str = "dup_ack"):
        self.history.append(self.cwnd / self.mss)

        if loss_type == "timeout":
            self.ssthresh = self.cwnd / 2
            self.cwnd = self.mss
            self.in_recovery = False
            self.duplicate_acks = 0
        elif loss_type == "dup_ack":
            self.duplicate_acks += 1
            if self.duplicate_acks >= 3:
                # Fast retransmit + fast recovery
                self.ssthresh = self.cwnd / 2
                self.cwnd = self.ssthresh + 3 * self.mss
                self.in_recovery = True
                self.duplicate_acks = 0

    def on_recovery_ack(self):
        """ACK received during fast recovery — exit recovery."""
        self.in_recovery = False
        self.cwnd = self.ssthresh

    def reset(self):
        self.cwnd = 1.0 * self.mss
        self.ssthresh = 64 * self.mss
        self.duplicate_acks = 0
        self.in_recovery = False
        self.history.clear()


# ── CUBIC ────────────────────────────────────────────────────────────

class Cubic:
    """TCP CUBIC: cubic window growth function."""

    C = 0.4        # Cubic scaling constant
    BETA = 0.7     # Multiplicative decrease factor

    def __init__(self, mss: int = 1460):
        self.mss = mss
        self.cwnd = 1.0 * mss
        self.ssthresh = 64 * mss
        self.w_max = self.cwnd / mss  # cwnd (in segments) before last loss
        self.epoch_start = 0.0
        self.t = 0.0
        self.duplicate_acks = 0
        self.in_recovery = False
        self.history: List[float] = []

    def _k(self) -> float:
        return math.cbrt(self.w_max * self.BETA / self.C)

    def _cubic_window(self, t: float) -> float:
        k = self._k()
        return self.C * ((t - k) ** 3) + self.w_max

    def on_ack(self):
        self.history.append(self.cwnd / self.mss)
        self.t += 1.0  # simplified: 1 unit per ACK

        if self.in_recovery:
            self.cwnd += self.mss
            return

        if self.cwnd < self.ssthresh:
            # Slow start
            self.cwnd += self.mss
        else:
            # Cubic congestion avoidance
            target = self._cubic_window(self.t) * self.mss
            if target > self.cwnd:
                self.cwnd += self.mss * (target - self.cwnd) / self.cwnd
            else:
                # Convex region: grow faster
                self.cwnd += self.mss * self.mss / self.cwnd

    def on_loss(self, loss_type: str = "dup_ack"):
        self.history.append(self.cwnd / self.mss)

        if loss_type == "timeout":
            self.w_max = self.cwnd / self.mss
            self.ssthresh = self.cwnd / 2
            self.cwnd = self.mss
            self.epoch_start = self.t
            self.in_recovery = False
        elif loss_type == "dup_ack":
            self.duplicate_acks += 1
            if self.duplicate_acks >= 3:
                self.w_max = self.cwnd / self.mss
                self.ssthresh = self.cwnd * self.BETA
                self.cwnd = self.ssthresh + 3 * self.mss
                self.epoch_start = self.t
                self.in_recovery = True
                self.duplicate_acks = 0

    def on_recovery_ack(self):
        self.in_recovery = False
        self.cwnd = self.ssthresh

    def reset(self):
        self.cwnd = 1.0 * self.mss
        self.ssthresh = 64 * self.mss
        self.w_max = 1.0
        self.epoch_start = 0.0
        self.t = 0.0
        self.in_recovery = False
        self.history.clear()


# ── BBR (simplified) ────────────────────────────────────────────────

class BBR:
    """Simplified BBR: model-based congestion control."""

    PACING_GAIN_CYCLE = [1.25, 0.75, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]

    def __init__(self, mss: int = 1460):
        self.mss = mss
        self.cwnd = 2.0 * mss
        self.btl_bw = 0.0           # Estimated bottleneck bandwidth (bytes/RTT)
        self.rt_prop = float('inf') # Minimum RTT observed
        self.pacing_rate = 0.0
        self.cycle_index = 0
        self.delivered = 0
        self.delivery_rate_samples: List[float] = []
        self.history: List[float] = []
        self.phase = "startup"

    def on_ack(self, delivered_bytes: int, rtt_sample: float):
        self.history.append(self.cwnd / self.mss)

        # Update delivery rate estimate
        if rtt_sample > 0:
            rate = delivered_bytes / rtt_sample
            self.delivery_rate_samples.append(rate)
            # EWMA of bandwidth (alpha = 0.875)
            if self.btl_bw == 0:
                self.btl_bw = rate
            else:
                self.btl_bw = 0.875 * self.btl_bw + 0.125 * rate

        # Update min RTT
        if rtt_sample < self.rt_prop:
            self.rt_prop = rtt_sample

        # Target cwnd = BDP (bandwidth-delay product)
        if self.rt_prop > 0 and self.btl_bw > 0:
            bdp = self.btl_bw * self.rt_prop
            gain = self.PACING_GAIN_CYCLE[self.cycle_index]
            target_cwnd = bdp * gain
            self.cwnd = max(2.0 * self.mss, target_cwnd)

        self.delivered += delivered_bytes

    def advance_cycle(self):
        self.cycle_index = (self.cycle_index + 1) % len(self.PACING_GAIN_CYCLE)

    def on_loss(self):
        # BBR doesn't drastically cut cwnd on loss — it uses the model
        self.history.append(self.cwnd / self.mss)
        # Slight reduction
        self.cwnd *= 0.9

    def reset(self):
        self.cwnd = 2.0 * self.mss
        self.btl_bw = 0.0
        self.rt_prop = float('inf')
        self.cycle_index = 0
        self.history.clear()


# ── Simulation ───────────────────────────────────────────────────────

def simulate(params: NetworkParams, algo) -> List[float]:
    """Run a congestion control algorithm through simulated network events."""
    algo.reset()

    bdp = (params.bandwidth_mbps * 1e6 / 8) * (params.rtt_ms / 1000)
    max_inflight = bdp + params.buffer_packets * params.mss

    rtt_samples = []

    for round_num in range(params.sim_rounds):
        # Simulate variable RTT (adds queuing delay when cwnd is large)
        queuing_delay = 0
        if hasattr(algo, 'cwnd'):
            queuing_delay = max(0, (algo.cwnd - bdp) / (params.bandwidth_mbps * 1e6 / 8)) * 1000
        current_rtt = params.rtt_ms + queuing_delay

        # Check for loss
        if algo.cwnd > max_inflight:
            algo.on_loss("dup_ack" if random.random() > 0.3 else "timeout")
        elif random.random() < params.loss_rate:
            algo.on_loss("dup_ack")
        else:
            # Successful ACK
            if isinstance(algo, BBR):
                algo.on_ack(params.mss, current_rtt / 1000)
                if round_num % 8 == 0:
                    algo.advance_cycle()
            elif hasattr(algo, 'in_recovery') and algo.in_recovery:
                algo.on_recovery_ack()
            else:
                algo.on_ack()

    return algo.history


def print_ascii_chart(data: List[float], title: str, width: int = 80, height: int = 20):
    """Print a simple ASCII chart of cwnd over time."""
    if not data:
        return

    max_val = max(data) if data else 1
    min_val = 0
    step = max(1, len(data) // width)

    print(f"\n{'=' * width}")
    print(f"  {title}")
    print(f"  Max cwnd: {max_val:.1f} MSS")
    print(f"{'=' * width}")

    # Sample data to fit width
    sampled = [data[min(i * step, len(data) - 1)] for i in range(width)]

    for row in range(height, 0, -1):
        threshold = min_val + (max_val - min_val) * row / height
        line = "  "
        for val in sampled:
            if val >= threshold:
                line += "█"
            else:
                line += " "
        if row == height:
            print(f"{threshold:6.0f} |{line}")
        elif row == 1:
            print(f"{min_val:6.0f} |{line}")
        elif row == height // 2:
            mid = (max_val + min_val) / 2
            print(f"{mid:6.0f} |{line}")
        else:
            print(f"       |{line}")

    print(f"       +{'─' * width}")
    print(f"        RTT rounds →\n")


def main():
    print("TCP Congestion Control Simulator")
    print("=" * 40)

    params = NetworkParams(
        bandwidth_mbps=100,
        rtt_ms=50,
        buffer_packets=64,
        loss_rate=0.01,
        mss=1460,
        sim_rounds=300,
    )

    print(f"Network: {params.bandwidth_mbps} Mbps, RTT={params.rtt_ms}ms, "
          f"buffer={params.buffer_packets} pkts, loss={params.loss_rate:.1%}")
    print(f"BDP = {(params.bandwidth_mbps * 1e6 / 8 * params.rtt_ms / 1000):.0f} bytes "
          f"= {params.bandwidth_mbps * 1e6 / 8 * params.rtt_ms / 1000 / params.mss:.0f} MSS")

    # Run each algorithm
    reno = Reno(params.mss)
    cubic = Cubic(params.mss)
    bbr = BBR(params.mss)

    reno_data = simulate(params, reno)
    cubic_data = simulate(params, cubic)
    bbr_data = simulate(params, bbr)

    print_ascii_chart(reno_data, "TCP Reno — cwnd (MSS)")
    print_ascii_chart(cubic_data, "TCP CUBIC — cwnd (MSS)")
    print_ascii_chart(bbr_data, "TCP BBR — cwnd (MSS)")

    # Summary
    print("Summary (last 50 rounds average):")
    print(f"  Reno  : {sum(reno_data[-50:]) / 50:6.1f} MSS")
    print(f"  CUBIC : {sum(cubic_data[-50:]) / 50:6.1f} MSS")
    print(f"  BBR   : {sum(bbr_data[-50:]) / 50:6.1f} MSS")


if __name__ == "__main__":
    main()
