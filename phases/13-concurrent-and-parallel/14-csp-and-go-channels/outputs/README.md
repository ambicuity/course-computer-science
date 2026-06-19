# CSP and Go Channels — Output Artifact

## What This Is

A self-contained Go program (`code/main.go`) that demonstrates the five canonical
CSP (Communicating Sequential Processes) patterns that form the backbone of
concurrent Go programming:

1. **Unbuffered channels** — synchronous rendezvous between goroutines
2. **Pipeline** — chained stages (generate → square → print)
3. **Fan-out / Fan-in** — distribute work to multiple workers, merge results
4. **Select with timeout** — race channel operations against a deadline
5. **Quit channel** — clean goroutine cancellation and cleanup

## How to Run

```bash
go run code/main.go
```

Requires Go 1.18+ (generics not used; any modern Go version works).

## How to Use as a Reference

- Copy individual patterns into your own concurrent programs.
- Modify buffer sizes, timeout durations, and worker counts to experiment.
- Extend the pipeline with additional stages (filter, reduce, transform).
- Replace `time.After` timeouts with `context.Context` deadlines in production.

## Key Terms Covered

CSP, goroutine, channel, select, pipeline, fan-out, fan-in, multiplexing,
goroutine leak, Go scheduler, M:N scheduling.

## Production Counterpart

See `docs/en.md` for comparison with Go's standard library (`net/http`,
`io.Pipe`, `context.Context`, `database/sql` pool) and pointers to the
runtime source (`runtime/chan.go`, `runtime/proc.go`, `runtime/select.go`).
