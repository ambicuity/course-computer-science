# outputs — Semaphores and the Classics

This directory contains compiled binaries and reusable reference material for
Phase 13, Lesson 6 (Semaphores and the Classics — Producer/Consumer, Dining).

## Artifacts

### Compiled binaries

Build from `code/main.c` / `code/main.go`:

```bash
# C binary
gcc -pthread -O2 -o prodcon_c ../code/main.c
./prodcon_c

# Go binary
go build -o prodcon_go ../code/main.go
./prodcon_go
```

Each binary runs all three synchronization problems in sequence:

1. **Producer–Consumer** (bounded buffer):  2 producers + 2 consumers,
   8-slot ring buffer, `sem_t` empty/full, POSIX mutex.
2. **Dining Philosophers**:  5 philosophers, 5 chopsticks, resource-ordering
   to prevent deadlock.
3. **Readers–Writers**:  4 readers, 2 writers, readers-preference (first
   problem).

### Reference snippets

- `prodcon_snippet.c` — Minimal bounded buffer (copy-paste template).
- `philosophers_snippet.c` — Deadlock-free dining philosophers skeleton.
- `rw_snippet.c` — First readers–writers problem skeleton.

### Performance notes

- The C version uses POSIX semaphores (`sem_wait`/`sem_post`) which are
  implemented via `futex` on Linux — fast path is ~25 ns uncontended.
- The Go version uses buffered channels as counting semaphores. Go channels
  are implemented on top of the runtime's semaphore (`runtime/sema.go`).
- Under high contention the C version tends to be 2–5× faster because it
  avoids the Go scheduler's M:N multiplexing overhead.  Under low contention
  the difference is negligible.

## Reuse in later phases

| Pattern       | Reused in                        |
|---------------|----------------------------------|
| Bounded buffer| Phase 14 — Lock-free ring buffer |
| Readers–writers | Phase 15 — Transactional memory |
| Semaphore pool | Phase 17 — Connection pooling   |
