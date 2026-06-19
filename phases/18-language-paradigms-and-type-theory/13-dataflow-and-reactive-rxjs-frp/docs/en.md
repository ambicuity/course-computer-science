# Dataflow and Reactive — RxJS, FRP

> Model time-varying values as streams, not ad-hoc callback chains.

**Type:** Learn
**Languages:** TypeScript
**Prerequisites:** Phase 18 lessons 01-12
**Time:** ~60 minutes

## Learning Objectives

- Understand push-based stream processing basics.
- Compose event pipelines with map/filter/scan patterns.
- Reason about backpressure and cancellation concerns.
- Build a lightweight reactive stream abstraction example.

## The Problem

A search box needs: debounce keystrokes, call an API, cancel in-flight requests when a new keystroke arrives, handle errors, and show a loading spinner. With callbacks:

```typescript
let timeoutId: number;
let currentAbort: AbortController;

function onInput(query: string) {
  clearTimeout(timeoutId);
  currentAbort?.abort();
  setLoading(true);

  timeoutId = setTimeout(async () => {
    currentAbort = new AbortController();
    try {
      const results = await fetch(`/search?q=${query}`, {
        signal: currentAbort.signal
      });
      renderResults(await results.json());
    } catch (e) {
      if (e.name !== 'AbortError') showError(e);
    } finally {
      setLoading(false);
    }
  }, 300);
}
```

This works, but the control flow is tangled. Debouncing, cancellation, error handling, and loading state are interleaved. Adding "retry on failure" or "cache results" makes it worse.

Reactive streams decompose the same problem into composable operators:

```typescript
search$.pipe(
  debounceTime(300),
  distinctUntilChanged(),
  switchMap(query => fetch(`/search?q=${query}`).pipe(
    catchError(e => of({ error: e }))
  ))
);
```

Each operator handles one concern. They compose. Adding retry is just `.pipe(retry(2))`.

## The Concept

### Streams and operators

A stream (Observable) is a sequence of values over time. Operators transform streams:

```
source:     --1---2---3---4---5--->
map(x=>x*2): --2---4---6---8---10-->
filter(even): ------4-------8------>
```

| Category | Operators | Purpose |
|----------|----------|---------|
| Creation | `of`, `from`, `interval`, `fromEvent` | Create streams |
| Transformation | `map`, `scan`, `mergeMap`, `switchMap` | Transform values |
| Filtering | `filter`, `debounceTime`, `distinctUntilChanged`, `take` | Select values |
| Combination | `merge`, `combineLatest`, `zip`, `forkJoin` | Combine streams |
| Error handling | `catchError`, `retry`, `finalize` | Handle failures |
| Utility | `tap`, `delay`, `timeout` | Side effects, timing |

### Push vs pull

| Model | Direction | Example |
|-------|-----------|---------|
| Pull | Consumer requests value | Iterators, generators |
| Push | Producer sends value | Observables, event emitters |

Reactive is push-based: the producer pushes values to subscribers. The subscriber reacts when values arrive.

### Hot vs cold observables

```
Cold:  Each subscription creates a new producer
       (HTTP request, file read)

Hot:   Multiple subscribers share one producer
       (mouse events, WebSocket messages)
```

```typescript
// Cold: each subscription triggers a new request
const search$ = (query: string) =>
  from(fetch(`/search?q=${query}`));

// Hot: shared across subscribers
const clicks$ = fromEvent(document, 'click').pipe(share());
```

### Backpressure

When the producer is faster than the consumer:

| Strategy | Behavior |
|----------|----------|
| Buffer | Store unconsumed values |
| Drop | Discard new values |
| Sample | Take latest value at fixed interval |
| Throttle | Allow one value per time window |

```typescript
// Buffer: store for later
fastProducer$.pipe(buffer(slowConsumer$))

// Drop: keep only latest
fastProducer$.pipe(debounceTime(100))

// Sample: take latest every second
fastProducer$.pipe(sampleTime(1000))
```

### FRP (Functional Reactive Programming)

FRP is a stricter model where time-varying values are first-class:

```
Behavior a : a value that changes over time
Event a    : discrete occurrences at points in time
```

RxJS is reactive but not strictly FRP (it's more "reactive programming with functional operators"). True FRP systems include Elm, Reflex (Haskell), and Sodium.

### switchMap vs mergeMap vs concatMap

| Operator | Behavior | Use case |
|----------|----------|----------|
| `switchMap` | Cancel previous, start new | Search-as-you-type |
| `mergeMap` | Run all concurrently | Independent parallel requests |
| `concatMap` | Run sequentially, queue | Order-dependent operations |

```
switchMap:   -a--b--c-->
             /  /  /
requests:   A--B--C-->
result:     --a'--b'--c'-->   (A cancelled, B cancelled)

mergeMap:    -a--b--c-->
             /  /  /
requests:   A--B--C-->
result:     --a'--b'--c'-->   (all complete)
```

## Build It

### Step 1: Basic stream operators (TypeScript)

```typescript
type Subscriber<T> = {
  next: (value: T) => void;
  error: (err: any) => void;
  complete: () => void;
};

type Teardown = () => void;

class Observable<T> {
  constructor(
    private subscribeFn: (subscriber: Subscriber<T>) => Teardown
  ) {}

  subscribe(subscriber: Partial<Subscriber<T>>): Teardown {
    const fullSub: Subscriber<T> = {
      next: subscriber.next ?? (() => {}),
      error: subscriber.error ?? (() => {}),
      complete: subscriber.complete ?? (() => {}),
    };
    return this.subscribeFn(fullSub);
  }

  map<U>(fn: (value: T) => U): Observable<U> {
    return new Observable(subscriber =>
      this.subscribe({
        next: value => subscriber.next(fn(value)),
        error: err => subscriber.error(err),
        complete: () => subscriber.complete(),
      })
    );
  }

  filter(predicate: (value: T) => boolean): Observable<T> {
    return new Observable(subscriber =>
      this.subscribe({
        next: value => { if (predicate(value)) subscriber.next(value); },
        error: err => subscriber.error(err),
        complete: () => subscriber.complete(),
      })
    );
  }
}
```

### Step 2: Creation operators

```typescript
function of<T>(...values: T[]): Observable<T> {
  return new Observable(subscriber => {
    for (const v of values) subscriber.next(v);
    subscriber.complete();
    return () => {};
  });
}

function fromArray<T>(values: T[]): Observable<T> {
  return new Observable(subscriber => {
    values.forEach(v => subscriber.next(v));
    subscriber.complete();
    return () => {};
  });
}

function interval(ms: number): Observable<number> {
  return new Observable(subscriber => {
    let i = 0;
    const id = setInterval(() => subscriber.next(i++), ms);
    return () => clearInterval(id);
  });
}
```

### Step 3: Pipeline demo

```typescript
// Transform and filter
of(1, 2, 3, 4, 5)
  .map(x => x * 2)
  .filter(x => x > 4)
  .subscribe({
    next: v => console.log(v),  // 6, 8, 10
    complete: () => console.log('done'),
  });

// Interval with map
interval(1000)
  .map(i => `tick ${i}`)
  .subscribe({ next: console.log });
```

### Step 4: Cancellation

```typescript
const sub = interval(1000).subscribe({
  next: v => console.log(v),
});

// Later: cancel
sub();  // calls teardown, clears interval
```

### Step 5: Scan (accumulate)

```typescript
function scan<T, A>(
  source: Observable<T>,
  reducer: (acc: A, value: T) => A,
  initial: A
): Observable<A> {
  return new Observable(subscriber => {
    let acc = initial;
    return source.subscribe({
      next: value => {
        acc = reducer(acc, value);
        subscriber.next(acc);
      },
      error: err => subscriber.error(err),
      complete: () => subscriber.complete(),
    });
  });
}

// Running sum
scan(of(1, 2, 3, 4, 5), (acc, x) => acc + x, 0)
  .subscribe({ next: console.log });  // 1, 3, 6, 10, 15
```

## Use It

Reactive pipelines are common in:

- **UI event handling**: Angular uses RxJS extensively. Click streams, form validation, route changes.
- **Telemetry processing**: Metrics, logs, and traces as streams with aggregation operators.
- **Async orchestration**: Microservice calls with retry, timeout, and fallback.
- **Real-time data**: WebSocket messages, SSE, live dashboards.
- **Game development**: Input handling, animation frames, state updates.

Production libraries: RxJS (TypeScript), Project Reactor (Java), Akka Streams (Scala), tokio-streams (Rust).

## Read the Source

- [RxJS Documentation](https://rxjs.dev/) — comprehensive operator reference.
- [Introduction to FRP](https://wiki.haskell.org/Functional_Reactive_Programming) — the theoretical foundation.
- [ReactiveX](http://reactivex.io/) — cross-language reactive extensions.
- [Elm Architecture](https://guide.elm-lang.org/architecture/) — FRP-inspired UI pattern.

## Ship It

- `code/main.ts`: stream operator demo.
- `outputs/README.md`: reactive design checklist.

## Quiz

**Q1 (Pre).** What's the difference between push and pull in reactive programming?

- A) Push sends values to consumers; pull has consumers request values.
- B) They're the same thing.
- C) Push is for UI; pull is for data.
- D) Pull is always faster.

**Answer: A.** In push-based reactive streams, the producer pushes values to subscribers when they're available. In pull-based systems (iterators, generators), the consumer requests the next value. Push is natural for events and async data; pull is natural for synchronous iteration.

**Q2 (Pre).** What does `switchMap` do?

- A) Switches between two observables alternately.
- B) Cancels the previous inner observable and subscribes to the new one.
- C) Maps values to switches.
- D) Merges all inner observables.

**Answer: B.** `switchMap` maps each source value to an inner observable, but when a new source value arrives, it unsubscribes from the previous inner observable. This is perfect for search-as-you-type: cancel the old request when a new keystroke arrives.

**Q3 (Post).** Why is the reactive version of the search box cleaner than the callback version?

- A) Reactive is faster.
- B) Each concern (debounce, cancel, error, loading) is a separate composable operator, not interleaved control flow.
- C) Callbacks can't handle errors.
- D) Reactive doesn't need types.

**Answer: B.** Reactive decomposition separates concerns: `debounceTime` handles timing, `switchMap` handles cancellation, `catchError` handles errors. Each operator is independently testable and composable. The callback version tangles all concerns in one function.

**Q4 (Post).** What's backpressure and how do you handle it?

- A) A type error.
- B) When a producer emits faster than a consumer processes; handle with buffering, dropping, sampling, or throttling.
- C) When a consumer is faster than a producer.
- D) A memory leak.

**Answer: B.** Backpressure occurs when the producer outpaces the consumer (e.g., a fast sensor feeding a slow display). Strategies: buffer (store for later), drop (discard new), sample (take latest at intervals), throttle (one per time window). The right strategy depends on the domain.

**Q5 (Post).** What's the difference between hot and cold observables?

- A) Hot is faster.
- B) Cold creates a new producer per subscription; hot shares one producer across subscribers.
- C) Cold can be cancelled; hot can't.
- D) They're the same.

**Answer: B.** A cold observable (e.g., an HTTP request) creates a new producer for each subscription. A hot observable (e.g., mouse events) shares one producer; late subscribers miss earlier values. Converting cold to hot uses `share()` or `Subject`.

## Exercises

1. **Easy.** Implement a `take(n)` operator that completes the observable after `n` values.
2. **Medium.** Add debounce semantics: a function that creates an observable that only emits after a specified silence period.
3. **Hard.** Compare a reactive pipeline for "type-ahead search" with an equivalent callback-based implementation. Count the lines of code and the number of state variables.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Stream | "event list over time" | An ordered sequence of asynchronous values |
| Operator | "transform step" | A function from stream to stream, composing into pipelines |
| Subscription | "listener" | An active consumer binding with lifecycle (subscribe/unsubscribe) |
| Backpressure | "too fast producer" | Managing rate mismatch between producer and consumer |
| switchMap | "cancel and switch" | Maps to inner observable, cancelling previous on new value |

## Further Reading

- [RxJS Docs](https://rxjs.dev/)
- [Introduction to FRP](https://wiki.haskell.org/Functional_Reactive_Programming)
- [ReactiveX](http://reactivex.io/)
- [The Elm Architecture](https://guide.elm-lang.org/architecture/)
