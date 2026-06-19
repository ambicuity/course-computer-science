// Semaphores and the Classics (Producer/Consumer, Dining)
// Phase 13 — Concurrent & Parallel Computing
//
// Demonstrates three classic synchronization problems in Go:
//
//  1. Producer–Consumer  (bounded buffer via buffered channels)
//  2. Dining Philosophers (resource-ordering + channel-based chopsticks)
//  3. Readers–Writers    (sync.RWMutex)
//
// Run: go run main.go

package main

import (
	"fmt"
	"math/rand"
	"sync"
	"time"
)

// -----------------------------------------------------------------------
//  Utility
// -----------------------------------------------------------------------

func randsleep(maxMs int) {
	time.Sleep(time.Duration(rand.Intn(maxMs)) * time.Millisecond)
}

// -----------------------------------------------------------------------
//  1. Producer–Consumer (bounded buffer)
//
//  Go's buffered channels ARE counting semaphores: a channel with
//  capacity N blocks the sender when full and the receiver when empty.
//  We wrap an unbuffered + buffered pair to mirror the classic empty/full
//  semaphore pattern.  A mutex protects the ring-buffer slice itself.
// -----------------------------------------------------------------------

const (
	prodBufSize = 8
	prodItems   = 16
)

type boundedBuffer struct {
	buf   []int
	head  int
	tail  int
	empty chan struct{} // counts empty slots (cap = prodBufSize)
	full  chan struct{} // counts filled slots (cap = prodBufSize)
	mu    sync.Mutex
}

func newBoundedBuffer() *boundedBuffer {
	bb := &boundedBuffer{
		buf:   make([]int, prodBufSize),
		empty: make(chan struct{}, prodBufSize),
		full:  make(chan struct{}, prodBufSize),
	}
	// Seed the empty channel so producers can send N tokens
	for i := 0; i < prodBufSize; i++ {
		bb.empty <- struct{}{}
	}
	return bb
}

func (bb *boundedBuffer) put(val int) {
	<-bb.empty // P(empty) — wait for an empty slot
	bb.mu.Lock()
	bb.buf[bb.tail%prodBufSize] = val
	bb.tail++
	bb.mu.Unlock()
	bb.full <- struct{}{} // V(full) — signal a filled slot
}

func (bb *boundedBuffer) get() int {
	<-bb.full // P(full) — wait for a filled slot
	bb.mu.Lock()
	val := bb.buf[bb.head%prodBufSize]
	bb.head++
	bb.mu.Unlock()
	bb.empty <- struct{}{} // V(empty) — signal an empty slot
	return val
}

func runProducerConsumer() {
	fmt.Println("\n========== Producer–Consumer (Bounded Buffer) ==========\n")

	bb := newBoundedBuffer()
	var wg sync.WaitGroup

	nProducers := 2
	nConsumers := 2
	itemsPer := prodItems / (nProducers + nConsumers)

	for p := 0; p < nProducers; p++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()
			for i := 0; i < itemsPer; i++ {
				val := id*1000 + i
				bb.put(val)
				fmt.Printf("[Producer %d] put %d\n", id, val)
				randsleep(50)
			}
		}(p + 1)
	}

	for c := 0; c < nConsumers; c++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()
			for i := 0; i < itemsPer; i++ {
				val := bb.get()
				fmt.Printf("[Consumer %d] got %d\n", id, val)
				randsleep(80)
			}
		}(c + 1)
	}

	wg.Wait()
	fmt.Printf("\nProducer–consumer finished: %d items\n", prodItems)
}

// -----------------------------------------------------------------------
//  2. Dining Philosophers
//
//  Each chopstick is a buffered channel of capacity 1 (a binary semaphore).
//  Resource-ordering: each philosopher picks up the lower-numbered
//  chopstick first to break circular wait (deadlock prevention).
// -----------------------------------------------------------------------

const nPhilosophers = 5

type chopstick chan struct{}

func newChopstick() chopstick {
	c := make(chan struct{}, 1)
	c <- struct{}{} // initially available
	return c
}

func (c chopstick) pickUp()  { <-c }       // P
func (c chopstick) putDown() { c <- struct{}{} } // V

func runDiningPhilosophers() {
	fmt.Println("\n========== Dining Philosophers ==========\n")

	sticks := make([]chopstick, nPhilosophers)
	for i := range sticks {
		sticks[i] = newChopstick()
	}

	var wg sync.WaitGroup

	for i := 0; i < nPhilosophers; i++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()

			// Resource ordering: always pick the lower-numbered chopstick
			left, right := id, (id+1)%nPhilosophers
			if left > right {
				left, right = right, left
			}

			for round := 0; round < 3; round++ {
				fmt.Printf("Philosopher %d is thinking...\n", id)
				randsleep(50)

				sticks[left].pickUp()
				fmt.Printf("Philosopher %d picked up chopstick %d\n", id, left)
				sticks[right].pickUp()
				fmt.Printf("Philosopher %d picked up chopstick %d\n", id, right)

				fmt.Printf("Philosopher %d is eating...\n", id)
				randsleep(30)

				sticks[left].putDown()
				sticks[right].putDown()
				fmt.Printf("Philosopher %d put down chopsticks\n", id)
			}
		}(i)
	}

	wg.Wait()
	fmt.Println("\nDining philosophers finished — no deadlock.")
}

// -----------------------------------------------------------------------
//  3. Readers–Writers
//
//  Go's sync.RWMutex implements the readers–writers pattern natively:
//  multiple concurrent RLock holders, exclusive Lock for writers.
//  We show the equivalent semaphore-style logic for illustration.
// -----------------------------------------------------------------------

const (
	rwReaders = 4
	rwWriters = 2
	rwOps     = 6
)

type readWriters struct {
	mu         sync.RWMutex
	sharedData int
}

func runReadersWriters() {
	fmt.Println("\n========== Readers–Writers (sync.RWMutex) ==========\n")

	rw := &readWriters{}
	var wg sync.WaitGroup

	for r := 0; r < rwReaders; r++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()
			for i := 0; i < rwOps; i++ {
				rw.mu.RLock()
				val := rw.sharedData
				fmt.Printf("[Reader %d] read value = %d\n", id, val)
				randsleep(20)
				rw.mu.RUnlock()
				randsleep(30)
			}
		}(r + 1)
	}

	for w := 0; w < rwWriters; w++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()
			for i := 0; i < rwOps; i++ {
				rw.mu.Lock()
				rw.sharedData++
				fmt.Printf("[Writer %d] wrote value = %d\n", id, rw.sharedData)
				randsleep(30)
				rw.mu.Unlock()
				randsleep(50)
			}
		}(w + 1)
	}

	wg.Wait()
	fmt.Printf("\nReaders–writers finished — final value: %d\n", rw.sharedData)
}

// -----------------------------------------------------------------------
//  Main
// -----------------------------------------------------------------------

func main() {
	rand.Seed(time.Now().UnixNano())

	runProducerConsumer()
	runDiningPhilosophers()
	runReadersWriters()

	fmt.Println("\nAll classic problems completed successfully.")
}
