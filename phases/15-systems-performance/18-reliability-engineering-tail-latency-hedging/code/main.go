// Reliability Engineering — Tail Latency, Hedging
// Phase 15 — Systems Programming & Performance
//
// Implements: hedged requests, circuit breaker, latency percentile calculator.

package main

import (
	"context"
	"fmt"
	"math"
	"math/rand"
	"sort"
	"sync"
	"time"
)

// --- Latency Percentile Calculator ---

type LatencySample struct {
	Value    time.Duration
	Backend  string
	Success  bool
}

type PercentileCalculator struct {
	mu      sync.Mutex
	samples []time.Duration
}

func NewPercentileCalculator() *PercentileCalculator {
	return &PercentileCalculator{samples: make([]time.Duration, 0)}
}

func (pc *PercentileCalculator) Record(d time.Duration) {
	pc.mu.Lock()
	pc.samples = append(pc.samples, d)
	pc.mu.Unlock()
}

func (pc *PercentileCalculator) Percentile(p float64) time.Duration {
	pc.mu.Lock()
	defer pc.mu.Unlock()
	if len(pc.samples) == 0 {
		return 0
	}
	sorted := make([]time.Duration, len(pc.samples))
	copy(sorted, pc.samples)
	sort.Slice(sorted, func(i, j int) bool { return sorted[i] < sorted[j] })

	idx := int(math.Ceil(float64(len(sorted))*p/100)) - 1
	if idx < 0 {
		idx = 0
	}
	if idx >= len(sorted) {
		idx = len(sorted) - 1
	}
	return sorted[idx]
}

func (pc *PercentileCalculator) Stats() (p50, p90, p95, p99, max time.Duration, count int) {
	pc.mu.Lock()
	samples := make([]time.Duration, len(pc.samples))
	copy(samples, pc.samples)
	pc.mu.Unlock()

	if len(samples) == 0 {
		return 0, 0, 0, 0, 0, 0
	}
	sort.Slice(samples, func(i, j int) bool { return samples[i] < samples[j] })
	n := len(samples)
	count = n
	p50 = samples[percentileIndex(n, 50)]
	p90 = samples[percentileIndex(n, 90)]
	p95 = samples[percentileIndex(n, 95)]
	p99 = samples[percentileIndex(n, 99)]
	max = samples[n-1]
	return
}

func percentileIndex(n, p int) int {
	idx := int(math.Ceil(float64(n)*float64(p)/100)) - 1
	if idx < 0 {
		return 0
	}
	if idx >= n {
		return n - 1
	}
	return idx
}

// --- Circuit Breaker ---

type CircuitState int

const (
	Closed   CircuitState = iota
	Open
	HalfOpen
)

type CircuitBreaker struct {
	mu             sync.Mutex
	state          CircuitState
	failures       int
	threshold      int
	openTimeout    time.Duration
	lastStateChange time.Time
	successes      int
	halfOpenMax    int
}

func NewCircuitBreaker(threshold int, openTimeout time.Duration) *CircuitBreaker {
	return &CircuitBreaker{
		state:        Closed,
		threshold:    threshold,
		openTimeout:  openTimeout,
		halfOpenMax:  1,
	}
}

func (cb *CircuitBreaker) Allow() bool {
	cb.mu.Lock()
	defer cb.mu.Unlock()

	switch cb.state {
	case Closed:
		return true
	case Open:
		if time.Since(cb.lastStateChange) > cb.openTimeout {
			cb.state = HalfOpen
			cb.successes = 0
			cb.lastStateChange = time.Now()
			return true
		}
		return false
	case HalfOpen:
		return cb.successes < cb.halfOpenMax
	default:
		return false
	}
}

func (cb *CircuitBreaker) RecordSuccess() {
	cb.mu.Lock()
	defer cb.mu.Unlock()
	cb.failures = 0
	if cb.state == HalfOpen {
		cb.successes++
		if cb.successes >= cb.halfOpenMax {
			cb.state = Closed
			cb.lastStateChange = time.Now()
		}
	}
}

func (cb *CircuitBreaker) RecordFailure() {
	cb.mu.Lock()
	defer cb.mu.Unlock()
	cb.failures++
	if cb.state == HalfOpen {
		cb.state = Open
		cb.lastStateChange = time.Now()
	} else if cb.failures >= cb.threshold {
		cb.state = Open
		cb.lastStateChange = time.Now()
	}
}

func (cb *CircuitBreaker) State() CircuitState {
	cb.mu.Lock()
	defer cb.mu.Unlock()
	return cb.state
}

// --- Simulated Backend ---

type Backend struct {
	Name       string
	MinLatency time.Duration
	MaxLatency time.Duration
	FailRate   float64
}

func (b *Backend) Call(ctx context.Context) (time.Duration, error) {
	latencyRange := float64(b.MaxLatency - b.MinLatency)
	// Heavy-tailed distribution: use exponential for latency
	// Most requests are fast, but some are very slow
	u := rand.Float64()
	var latency time.Duration
	if u < 0.9 {
		// 90% of requests: fast path
		latency = b.MinLatency + time.Duration(rand.Float64()*latencyRange*0.3)
	} else if u < 0.99 {
		// 9% of requests: medium slow
		latency = b.MinLatency + time.Duration(latencyRange*0.3+rand.Float64()*latencyRange*0.5)
	} else {
		// 1% of requests: very slow (tail)
		latency = b.MinLatency + time.Duration(latencyRange*0.8+rand.Float64()*latencyRange*0.2)
	}

	select {
	case <-time.After(latency):
		if rand.Float64() < b.FailRate {
			return latency, fmt.Errorf("backend %s: random failure", b.Name)
		}
		return latency, nil
	case <-ctx.Done():
		return latency, ctx.Err()
	}
}

// --- Hedged Request ---

type HedgedResult struct {
	ResponseTime time.Duration
	UsedHedge   bool
	Backend     string
	Err         error
}

func HedgedRequest(ctx context.Context, backends []*Backend, hedgeDelay time.Duration) HedgedResult {
	type callResult struct {
		latency time.Duration
		name    string
		err     error
	}

	ch := make(chan callResult, len(backends))
	childCtx, cancel := context.WithCancel(ctx)
	defer cancel()

	// Send primary request
	go func() {
		d, err := backends[0].Call(childCtx)
		ch <- callResult{d, backends[0].Name, err}
	}()

	timer := time.NewTimer(hedgeDelay)
	defer timer.Stop()

	hedgeSent := false

	// Wait for either primary or hedge timer
	select {
	case r := <-ch:
		return HedgedResult{r.latency, false, r.name, r.err}
	case <-timer.C:
		// Primary is slow; send hedge
		if len(backends) > 1 {
			hedgeSent = true
			go func() {
				d, err := backends[1].Call(childCtx)
				ch <- callResult{d, backends[1].Name, err}
			}()
		}
	}

	// Wait for first response (primary or hedge)
	r := <-ch
	cancel() // cancel the outstanding request

	return HedgedResult{r.latency, hedgeSent, r.name, r.err}
}

// --- Fan-out Simulation ---

func simulateFanout(numRequests int, backends []*Backend, useHedging bool, hedgeDelay time.Duration, cb *CircuitBreaker) *PercentileCalculator {
	pc := NewPercentileCalculator()
	var wg sync.WaitGroup

	for i := 0; i < numRequests; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()

			if !cb.Allow() {
				pc.Record(1 * time.Millisecond) // fail-fast latency
				return
			}

			ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
			defer cancel()

			var result HedgedResult
			if useHedging {
				result = HedgedRequest(ctx, backends, hedgeDelay)
			} else {
				// Single request — no hedging
				d, err := backends[0].Call(ctx)
				result = HedgedResult{d, false, backends[0].Name, err}
			}

			if result.Err != nil {
				cb.RecordFailure()
			} else {
				cb.RecordSuccess()
			}

			pc.Record(result.ResponseTime)
		}()
	}

	wg.Wait()
	return pc
}

func main() {
	rand.Seed(time.Now().UnixNano())

	backends := []*Backend{
		{Name: "A", MinLatency: 5 * time.Millisecond, MaxLatency: 500 * time.Millisecond, FailRate: 0.02},
		{Name: "B", MinLatency: 5 * time.Millisecond, MaxLatency: 500 * time.Millisecond, FailRate: 0.02},
		{Name: "C", MinLatency: 5 * time.Millisecond, MaxLatency: 600 * time.Millisecond, FailRate: 0.03},
	}

	hedgeDelay := 20 * time.Millisecond
	numRequests := 1000

	fmt.Println("=== Tail Latency & Hedging Demo ===")
	fmt.Println()

	// Run without hedging
	cb1 := NewCircuitBreaker(5, 30*time.Second)
	pc1 := simulateFanout(numRequests, backends, false, hedgeDelay, cb1)
	p50, p90, p95, p99, max, count := pc1.Stats()
	fmt.Println("--- WITHOUT Hedging ---")
	fmt.Printf("  Requests:  %d\n", count)
	fmt.Printf("  p50:  %v\n", p50)
	fmt.Printf("  p90:  %v\n", p90)
	fmt.Printf("  p95:  %v\n", p95)
	fmt.Printf("  p99:  %v\n", p99)
	fmt.Printf("  max:  %v\n", max)
	fmt.Printf("  Circuit breaker state: %v\n", cb1.State())
	fmt.Println()

	// Run with hedging
	cb2 := NewCircuitBreaker(5, 30*time.Second)
	pc2 := simulateFanout(numRequests, backends, true, hedgeDelay, cb2)
	p50, p90, p95, p99, max, count = pc2.Stats()
	fmt.Println("--- WITH Hedging (delay=20ms) ---")
	fmt.Printf("  Requests:  %d\n", count)
	fmt.Printf("  p50:  %v\n", p50)
	fmt.Printf("  p90:  %v\n", p90)
	fmt.Printf("  p95:  %v\n", p95)
	fmt.Printf("  p99:  %v\n", p99)
	fmt.Printf("  max:  %v\n", max)
	fmt.Printf("  Circuit breaker state: %v\n", cb2.State())
	fmt.Println()

	// Demonstrate the latency lottery
	fmt.Println("--- Latency Lottery (fan-out=100) ---")
	p99val := float64(pc1.Percentile(99)) / float64(time.Millisecond)
	probSlow := 1 - math.Pow(0.99, 100)
	fmt.Printf("  Single-call p99:       %.1f ms\n", p99val)
	fmt.Printf("  P(at least one slow call in 100): %.1f%%\n", probSlow*100)
	fmt.Printf("  With hedging, P(both slow):      %.4f%%\n", math.Pow(0.01, 2)*100)
	fmt.Println()

	// Circuit breaker demo
	fmt.Println("--- Circuit Breaker Demo ---")
	cb := NewCircuitBreaker(3, 100*time.Millisecond)
	fmt.Printf("  Initial state:  %v\n", cb.State())
	for i := 0; i < 5; i++ {
		cb.RecordFailure()
		fmt.Printf("  After failure %d: state=%v\n", i+1, cb.State())
	}
	fmt.Printf("  Request allowed: %v\n", cb.Allow())
	time.Sleep(150 * time.Millisecond)
	fmt.Printf("  After timeout:   state=%v\n", cb.State())
	fmt.Printf("  Request allowed: %v\n", cb.Allow())
	cb.RecordSuccess()
	fmt.Printf("  After success:   state=%v\n", cb.State())
}