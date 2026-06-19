# TCP Congestion Control — Reno, CUBIC, BBR

> TCP Congestion Control — Reno, CUBIC, BBR — the part of CS you can't skip.

**Type:** Learn
**Languages:** Python, C
**Prerequisites:** Phase 09 lessons 01–07
**Time:** ~75 minutes

## Learning Objectives

- Understand the core concept introduced in this lesson and why it matters.
- Implement the lesson's "Build It" artifact from scratch in one of: Python, C.
- Compare your from-scratch implementation against the production tool used in industry.
- Ship the reusable artifact (see "Ship It") and add it to your toolbox.

## The Problem

This lesson sits in **Phase 09 — Computer Networks**. Without the concept it teaches, you cannot
build the phase's capstone (An HTTP/2 server on a custom userspace TCP/IP stack.). Concretely, *not* knowing this means you get stuck the
moment you try to build the stack: ethernet, ip, tcp, tls, http — by hand.

The next few sections walk through the smallest concrete scenario where this gap hurts, then build
the mental model, then the code, then the production equivalent.

## The Concept

TCP has flow control (receiver window) but that alone is not enough. If every TCP connection
fills the network as fast as possible, routers drop packets and throughput collapses — this is
**congestion collapse**. Congestion control algorithms regulate each connection's sending rate
to keep the network healthy.

### Core Variables

- **cwnd** (congestion window): maximum bytes in flight. Effective window = `min(cwnd, rwnd)`.
- **ssthresh** (slow-start threshold): the cwnd value at which we switch from slow start to congestion avoidance.
- **MSS** (maximum segment size): typically 1460 bytes on Ethernet.
- **RTT**: round-trip time measured per segment.

### Slow Start

When a connection begins (or after a timeout), cwnd starts at 1 MSS. Every ACK received
doubles cwnd — exponential growth. The sender probes the network's capacity quickly.

```
RTT 0: cwnd = 1    (send 1 segment)
RTT 1: cwnd = 2    (2 ACKs arrive, each adds 1 MSS)
RTT 2: cwnd = 4
RTT 3: cwnd = 8
...until cwnd >= ssthresh
```

In theory cwnd doubles each RTT. In practice, the doubling is per-ACK: each ACK adds
`min(bytes_acked, SMSS)` to cwnd.

### Congestion Avoidance

Once cwnd reaches ssthresh, the algorithm enters **congestion avoidance** — linear growth.
Each ACK increases cwnd by approximately 1/cwnd segments per RTT:

```
cwnd += MSS × (MSS / cwnd)
```

This is additive increase: the sending rate grows slowly, probing for additional capacity
without causing congestion.

### Fast Retransmit and Fast Recovery

When the sender receives **3 duplicate ACKs**, it infers a packet was lost but the network
is still delivering (ACKs are flowing). Instead of waiting for a timeout:

1. **Fast retransmit**: immediately retransmit the missing segment.
2. **Fast recovery**: set `ssthresh = cwnd / 2`, set `cwnd = ssthresh + 3 MSS`,
   then enter congestion avoidance (not slow start).

This avoids the costly slow-start restart and keeps the pipeline flowing.

### TCP Reno

Reno is the classic algorithm. Its behavior:
- **Slow start**: cwnd doubles each RTT until ssthresh.
- **Congestion avoidance**: cwnd grows by 1 MSS per RTT.
- **3 dup ACKs**: fast retransmit, fast recovery (cwnd halved).
- **Timeout**: ssthresh = cwnd/2, cwnd = 1 MSS, back to slow start.

Reno's weakness: it treats all packet loss equally. On high-bandwidth, high-latency links
(long fat networks), halving cwnd on every loss event wastes available bandwidth.

### TCP CUBIC

CUBIC (Linux default since kernel 2.6.19) replaces Reno's linear window growth with a
**cubic function**:

```
W(t) = C × (t - K)³ + W_max
```

Where:
- `t` = time since last loss event
- `W_max` = cwnd just before the last loss
- `C` = scaling constant (0.4)
- `K = ∛(W_max × β / C)` — the time to reach W_max again

CUBIC is **window-based** rather than time-based. It probes aggressively near `W_max`
(optimistic region), then more conservatively afterward. This yields better utilization on
high-bandwidth links where Reno's linear growth is too slow.

### TCP BBR (Bottleneck Bandwidth and RTT)

BBR (Google, 2016) takes a fundamentally different approach. Instead of reacting to packet
loss, BBR **models the network path**:

- **BtlBw**: estimated bottleneck bandwidth (max delivery rate observed).
- **RTprop**: minimum RTT observed (propagation delay at no queuing).

BBR's target sending rate: `pacing_rate = BtlBw × gain`.

BBR cycles through phases:
1. **Startup**: exponential increase to find BtlBw (like slow start but loss doesn't stop it).
2. **Drain**: drain the queue built during startup.
3. **ProbeBW**: cycle of +25% / -25% to find the true capacity.
4. **ProbeRTT**: periodically reduce inflight to 4 packets to measure fresh RTprop.

BBR avoids the bufferbloat problem — it doesn't fill router queues because it targets
the bandwidth-delay product, not the loss threshold.

## Build It

### Step 1: Reno in C

A minimal Reno implementation tracking cwnd and ssthresh through slow start, congestion
avoidance, and fast recovery.

```c
// code/congestion.c — see the full file for a runnable version
```

### Step 2: Reno, CUBIC, and BBR Simulators in Python

A Python simulation comparing all three algorithms under identical network conditions.

```python
// code/main.py — see the full file for a runnable version
```

The simulator models: bandwidth, RTT, buffer size, and loss rate. Each algorithm
runs for the same number of RTTs and cwnd is recorded at each step for comparison plots.

## Use It

**Linux CUBIC source:** `net/ipv4/tcp_cubic.c` — the `cubic_update()` function implements
the cubic window calculation. `bictcp_cong_avoid()` is called on every ACK.

**Linux BBR source:** `net/ipv4/tcp_bbr.c` — `bbr_advance_cycle_phase()` handles the
ProbeBW cycling. `bbr_lt_bw_sampling()` detects long-term bandwidth changes.

**RFC 8312** specifies CUBIC. **RFC 9438** updates it (2023). Google's BBR paper:
"BBR: Congestion-Based Congestion Control" (ACM Queue, 2016).

Your simulator captures the essential behavior: cwnd growth patterns, response to loss,
and the cubic vs linear vs model-based differences. The production versions add
per-ACK granularity, precise RTT measurement, and pacing with packet scatter.

## Read the Source

- Linux `net/ipv4/tcp_cubic.c` — `bictcp_update()` computes the cubic function.
- Linux `net/ipv4/tcp_bbr.c` — `bbr_set_pacing_rate()` sets the transmission rate from the model.
- RFC 9438 — CUBIC for TCP (2023 update of RFC 8312).

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A congestion control library** — pluggable Reno/CUBIC/BBR simulators for later use.

## Exercises

1. **Easy** — Implement the slow start phase of Reno in C without looking at the lesson code.
   Print cwnd at each RTT until it reaches ssthresh = 32 MSS.
2. **Medium** — Extend the Python simulator to model a **bottleneck link**: a single router
   with a fixed packet buffer. When cwnd exceeds `BDP + buffer`, packets are dropped.
   Verify that BBR's cwnd stays near the BDP while Reno oscillates.
3. **Hard** — Implement BBR v2: add inflight_delivered tracking, the EWMA filter for
   BtlBw estimation, and the PacingGain cycle (1.25, 0.75, 1, 1, 1, 1, 1, 1).
   Compare steady-state throughput and 95th-percentile latency vs BBR v1.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| cwnd | "Congestion window" | Maximum number of unacknowledged bytes the sender can have in flight |
| ssthresh | "Slow start threshold" | cwnd value at which slow start transitions to congestion avoidance |
| Slow start | "Exponential growth phase" | cwnd doubles each RTT until ssthresh, probing network capacity |
| Congestion avoidance | "Linear growth phase" | cwnd grows by ~1 MSS per RTT after reaching ssthresh |
| Fast retransmit | "Retransmit on 3 dup ACKs" | Retransmit lost segment immediately on 3 duplicate ACKs instead of waiting for timeout |
| Fast recovery | "Skip slow start" | After fast retransmit, set cwnd = ssthresh and enter congestion avoidance |
| CUBIC | "Linux default" | Cubic-function window growth, better utilization on high-BDP links |
| BBR | "Model-based CC" | Estimates bottleneck bandwidth and min RTT to set sending rate without relying on loss |
| Bufferbloat | "Full queues" | Excessive buffering in routers causes high latency; BBR addresses this by not filling queues |
| BDP | "Bandwidth-delay product" | Bandwidth × RTT — the number of bytes that should be in flight to fully utilize a link |

## Further Reading

- RFC 9438 — CUBIC for TCP (2023)
- RFC 9002 — QUIC Loss Detection and Congestion Control
- Cardwell et al., "BBR: Congestion-Based Congestion Control" (ACM Queue, 2016)
- Stevens, *TCP/IP Illustrated, Vol. 1*, Chapter 21 — TCP congestion control
- "Congestion Avoidance and Control" by Van Jacobson (1988) — the original paper
