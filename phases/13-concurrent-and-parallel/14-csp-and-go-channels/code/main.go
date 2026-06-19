// CSP and Go Channels
// Phase 13 — Concurrent & Parallel Computing, Lesson 14
//
// Build: 5 patterns demonstrating CSP in Go.
//   Step 1 — goroutines + unbuffered channels (synchronous handoff)
//   Step 2 — pipeline pattern (generate → square → print)
//   Step 3 — fan-out / fan-in  (multiple workers, merged results)
//   Step 4 — select with timeout
//   Step 5 — quit channel  (goroutine cleanup / cancellation)
//
// Run: go run main.go
package main

import (
	"fmt"
	"sync"
	"time"
)

// ──────────────────────────────────────────────────────────────────────
// Step 1 — goroutines + unbuffered channels  (synchronous handoff)
// ──────────────────────────────────────────────────────────────────────

func step1_unbuffered_channels() {
	fmt.Println("=== Step 1: Unbuffered Channels (Sync Handoff) ===")

	// An unbuffered channel has capacity 0.
	// A send blocks until a corresponding receive is ready — and vice‑versa.
	// This is a rendezvous: both goroutines arrive at the same moment.
	ch := make(chan string)

	// Sender
	go func() {
		msg := "ping"
		fmt.Printf("  sender  about to send %q (will block until receiver is ready)\n", msg)
		ch <- msg // blocks until main is ready to receive
		fmt.Printf("  sender  sent %q\n", msg)
	}()

	// tiny sleep so the goroutine starts and blocks on send
	time.Sleep(10 * time.Millisecond)
	fmt.Println("  main    about to receive (unblocks sender)")

	received := <-ch
	fmt.Printf("  main    received %q\n", received)
	// brief pause so the sender goroutine can print its "sent" line
	time.Sleep(5 * time.Millisecond)
	fmt.Println()
}

// ──────────────────────────────────────────────────────────────────────
// Step 2 — Pipeline (generate → square → print)
// ──────────────────────────────────────────────────────────────────────

// generate sends numbers 1..n on the returned channel.
func generate(n int) <-chan int {
	out := make(chan int)
	go func() {
		for i := 1; i <= n; i++ {
			out <- i
		}
		close(out)
	}()
	return out
}

// square reads ints from in, squares them, and writes results to out.
func square(in <-chan int) <-chan int {
	out := make(chan int)
	go func() {
		for v := range in {
			out <- v * v
		}
		close(out)
	}()
	return out
}

// print reads ints from in and prints them.
func print(in <-chan int, label string) {
	fmt.Printf("  %s:", label)
	for v := range in {
		fmt.Printf(" %d", v)
	}
	fmt.Println()
}

func step2_pipeline() {
	fmt.Println("=== Step 2: Pipeline (generate → square → print) ===")

	// Pipe connects three stages via channels.
	// Each stage runs in its own goroutine.
	// Closing the output channel when done signals the next stage.
	numbers := generate(5)
	squares := square(numbers)
	print(squares, "pipeline")

	fmt.Println()
}

// ──────────────────────────────────────────────────────────────────────
// Step 3 — Fan-out / Fan-in
// ──────────────────────────────────────────────────────────────────────

// worker reads jobs, squares them, and sends result on out.
// It uses a WaitGroup to signal when it's done.
func worker(id int, jobs <-chan int, out chan<- int, wg *sync.WaitGroup) {
	defer wg.Done()
	for j := range jobs {
		result := j * j
		fmt.Printf("    worker %d computed %d^2 = %d\n", id, j, result)
		out <- result
	}
}

// merge reads from multiple output channels and writes to a single channel.
func merge(cs ...<-chan int) <-chan int {
	out := make(chan int)
	var wg sync.WaitGroup
	wg.Add(len(cs))

	for _, c := range cs {
		go func(ch <-chan int) {
			defer wg.Done()
			for v := range ch {
				out <- v
			}
		}(c)
	}

	// Close out when all worker output channels are exhausted.
	go func() {
		wg.Wait()
		close(out)
	}()

	return out
}

func step3_fan_out_fan_in() {
	fmt.Println("=== Step 3: Fan-out / Fan-in ===")

	const numJobs = 8
	const numWorkers = 3

	jobs := make(chan int, numJobs) // buffered so we can send before workers start

	// Fan-out: launch workers
	var workerWg sync.WaitGroup
	workerWg.Add(numWorkers)

	// Create per-worker output channels so we can merge them individually.
	workerChs := make([]chan int, numWorkers)
	for i := 0; i < numWorkers; i++ {
		workerChs[i] = make(chan int)
		go worker(i+1, jobs, workerChs[i], &workerWg)
	}

	// Send jobs
	for j := 1; j <= numJobs; j++ {
		jobs <- j
	}
	close(jobs)

	// Wait for all workers to finish, then close each output channel.
	go func() {
		workerWg.Wait()
		for _, ch := range workerChs {
			close(ch)
		}
	}()

	// Convert []chan int to []<-chan int for merge
	readOnlyChs := make([]<-chan int, numWorkers)
	for i, ch := range workerChs {
		readOnlyChs[i] = ch
	}

	// Fan-in: merge all worker outputs into one.
	results := merge(readOnlyChs...)

	fmt.Print("  merged results:")
	for r := range results {
		fmt.Printf(" %d", r)
	}
	fmt.Println("\n")
}

// ──────────────────────────────────────────────────────────────────────
// Step 4 — select with timeout
// ──────────────────────────────────────────────────────────────────────

func step4_select_timeout() {
	fmt.Println("=== Step 4: select with Timeout ===")

	// Simulate a slow operation.
	slowOp := make(chan string)
	go func() {
		time.Sleep(150 * time.Millisecond)
		slowOp <- "result ready"
	}()

	// select races multiple channel operations.
	// The first one that unblocks wins.
	select {
	case res := <-slowOp:
		fmt.Printf("  slow operation completed: %s\n", res)
	case <-time.After(100 * time.Millisecond):
		fmt.Println("  timeout: slow operation took too long!")
	}

	// Second attempt with a generous timeout.
	go func() {
		time.Sleep(50 * time.Millisecond)
		slowOp <- "second result"
	}()

	select {
	case res := <-slowOp:
		fmt.Printf("  fast enough: %s\n", res)
	case <-time.After(200 * time.Millisecond):
		fmt.Println("  timeout (should not happen)")
	}

	fmt.Println()
}

// ──────────────────────────────────────────────────────────────────────
// Step 5 — Quit channel pattern  (goroutine cleanup / cancellation)
// ──────────────────────────────────────────────────────────────────────

// A "quit" channel signals a long‑lived goroutine to stop.
// The goroutine selects on both work and quit — whichever fires first wins.
func step5_quit_channel() {
	fmt.Println("=== Step 5: Quit Channel Pattern ===")

	work := make(chan int)
	quit := make(chan struct{}) // empty struct signals "no data needed"

	// Start a worker that generates numbers until told to stop.
	go func() {
		i := 1
		for {
			select {
			case work <- i:
				i++
				time.Sleep(20 * time.Millisecond)
			case <-quit:
				fmt.Println("  worker received quit signal — cleaning up")
				return
			}
		}
	}()

	// Consume some values, then send the quit signal.
	for i := 0; i < 5; i++ {
		fmt.Printf("  main received: %d\n", <-work)
	}
	fmt.Println("  main sending quit signal")
	close(quit) // closing the quit channel unblocks all waiting receivers

	// Give the worker time to print its cleanup message.
	time.Sleep(10 * time.Millisecond)

	fmt.Println()
}

// ──────────────────────────────────────────────────────────────────────
// main
// ──────────────────────────────────────────────────────────────────────

func main() {
	step1_unbuffered_channels()
	step2_pipeline()
	step3_fan_out_fan_in()
	step4_select_timeout()
	step5_quit_channel()

	fmt.Println("=== Summary ===")
	fmt.Println("Demonstrated 5 CSP patterns in Go:")
	fmt.Println("  1. Unbuffered channels — synchronous rendezvous")
	fmt.Println("  2. Pipeline — generate → square → print")
	fmt.Println("  3. Fan-out / Fan-in — multiple workers, merged results")
	fmt.Println("  4. select with timeout")
	fmt.Println("  5. Quit channel — goroutine cancellation")
}
