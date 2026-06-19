// Phase 13, Lesson 11 — Futures, Promises, async/await (TypeScript)
// Demonstrates: manual callback-based promise, async/await rewrite,
// Promise.all / Promise.race / Promise.allSettled, error handling,
// microtask scheduling vs macrotask.
//
// Run with Node.js:  node main.ts
// (No dependencies — uses only built-in Promise API)

// ============================================================================
// Step 1: Manual Callback-Based Promise (Minimal Polyfill)
// ============================================================================
// A minimal Promise implementation to show how .then() and the resolve/reject
// pattern work under the hood. The real Promise is more complex (microtask
// scheduling, chaining, error propagation) but the core idea is the same.

type Executor<T> = (
  resolve: (value: T | PromiseLike<T>) => void,
  reject: (reason?: unknown) => void,
) => void;

type ThenCallback<T, U> = (value: T) => U | PromiseLike<U>;
type CatchCallback<U> = (reason: unknown) => U | PromiseLike<U>;

class SimplePromise<T> {
  private state: "pending" | "resolved" | "rejected" = "pending";
  private value: T | undefined = undefined;
  private reason: unknown = undefined;
  private thenCallbacks: Array<{
    onFulfilled?: ThenCallback<T, unknown>;
    onRejected?: CatchCallback<unknown>;
    resolve: (value: unknown) => void;
    reject: (reason: unknown) => void;
  }> = [];

  constructor(executor: Executor<T>) {
    // The executor runs *synchronously* during construction (eager evaluation)
    try {
      executor(
        (val) => this.resolve(val),
        (err) => this.reject(err),
      );
    } catch (err) {
      this.reject(err);
    }
  }

  private resolve(value: T | PromiseLike<T>): void {
    if (this.state !== "pending") return;
    this.state = "resolved";
    this.value = value as T;
    this.flush();
  }

  private reject(reason: unknown): void {
    if (this.state !== "pending") return;
    this.state = "rejected";
    this.reason = reason;
    this.flush();
  }

  private flush(): void {
    // In a real Promise, callbacks run as microtasks — delayed until
    // the current synchronous execution finishes. Here we call them
    // synchronously for simplicity. This is the key difference from
    // the real Promise (and why microtasks matter).
    const callbacks = this.thenCallbacks;
    this.thenCallbacks = [];
    for (const cb of callbacks) {
      if (this.state === "resolved") {
        if (cb.onFulfilled) {
          try {
            const result = cb.onFulfilled(this.value!);
            cb.resolve(result);
          } catch (err) {
            cb.reject(err);
          }
        } else {
          cb.resolve(this.value!);
        }
      } else {
        if (cb.onRejected) {
          try {
            const result = cb.onRejected(this.reason);
            cb.resolve(result);
          } catch (err) {
            cb.reject(err);
          }
        } else {
          cb.reject(this.reason);
        }
      }
    }
  }

  then<U>(onFulfilled?: ThenCallback<T, U>): SimplePromise<U> {
    return new SimplePromise<U>((resolve, reject) => {
      if (this.state === "pending") {
        this.thenCallbacks.push({
          onFulfilled: onFulfilled as ThenCallback<T, unknown> | undefined,
          resolve: resolve as (value: unknown) => void,
          reject,
        });
      } else if (this.state === "resolved" && onFulfilled) {
        try {
          resolve(onFulfilled(this.value!));
        } catch (err) {
          reject(err);
        }
      }
    });
  }

  catch<U>(onRejected: CatchCallback<U>): SimplePromise<U> {
    return new SimplePromise<U>((resolve, reject) => {
      if (this.state === "pending") {
        this.thenCallbacks.push({
          onFulfilled: undefined,
          onRejected: onRejected as CatchCallback<unknown>,
          resolve: resolve as (value: unknown) => void,
          reject,
        });
      } else if (this.state === "rejected") {
        try {
          resolve(onRejected(this.reason));
        } catch (err) {
          reject(err);
        }
      }
    });
  }
}

// Helper: wraps setTimeout into a SimplePromise
function delaySimple(ms: number): SimplePromise<void> {
  return new SimplePromise<void>((resolve) => {
    setTimeout(() => resolve(undefined), ms);
  });
}

function step1ManualPromise(): void {
  console.log("--- Step 1: Manual Callback-Based Promise ---");
  const p = delaySimple(50);
  p.then(() => {
    console.log("  SimplePromise resolved after 50ms");
  });
  console.log("  (SimplePromise executor runs synchronously in constructor)");
}

// ============================================================================
// Step 2: Async/Await with Real Promises
// ============================================================================

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function fetchData(id: number): Promise<string> {
  console.log(`    fetchData(${id}): starting`);
  await delay(50);
  const result = `data-${id}`;
  console.log(`    fetchData(${id}): complete -> ${result}`);
  return result;
}

async function step2AsyncAwait(): Promise<void> {
  console.log("\n--- Step 2: Async/Await Rewrite ---");
  const start = Date.now();
  const result = await fetchData(42);
  console.log(`  Result: ${result} in ${Date.now() - start}ms`);
  console.log("  (async/await desugars to .then() chains)");
}

// ============================================================================
// Step 3: Microtask Scheduling
// ============================================================================
// Demonstrates that Promise.then callbacks run as microtasks — before
// setTimeout (macrotask) but after the current synchronous block.

async function step3EventLoop(): Promise<void> {
  console.log("\n--- Step 3: Microtask vs Macrotask ---");
  console.log("  1: synchronous start");

  setTimeout(() => console.log("  5: macrotask (setTimeout)"), 0);

  Promise.resolve().then(() => console.log("  3: microtask (Promise.then)"));

  // queueMicrotask is the explicit microtask API
  queueMicrotask(() => console.log("  4: microtask (queueMicrotask)"));

  // await also schedules via microtask
  await Promise.resolve();
  console.log("  2: after await (microtask)");

  // Note: The await continuation (step "2") is queued as a microtask, but
  // .then() and queueMicrotask callbacks registered during the current
  // synchronous block are queued *before* the await continuation because
  // the await Promise.resolve() resolves in the current microtask batch.
  //
  // Output will be: 1, 3, 4, 2, 5
}

// ============================================================================
// Step 4a: Promise.all — Concurrent Composition
// ============================================================================

async function step4aPromiseAll(): Promise<void> {
  console.log("\n--- Step 4a: Promise.all (run concurrently, collect all) ---");

  async function slowDouble(n: number): Promise<number> {
    await delay(30 * n);
    return n * 2;
  }

  const start = Date.now();
  const results = await Promise.all([
    slowDouble(1),
    slowDouble(2),
    slowDouble(3),
  ]);
  const elapsed = Date.now() - start;
  console.log(`  Results: [${results}] in ${elapsed}ms`);
  console.log("  (sequential would take ~180ms, concurrent took ~90ms)");

  // Promise.all fails-fast: if any promise rejects, the whole thing rejects
  try {
    await Promise.all([
      Promise.resolve("ok"),
      Promise.reject(new Error("boom")),
      Promise.resolve("also ok (but never seen)"),
    ]);
  } catch (err) {
    console.log(`  Promise.all fail-fast: caught ${(err as Error).message}`);
  }
}

// ============================================================================
// Step 4b: Promise.race — Select semantics
// ============================================================================

async function step4bPromiseRace(): Promise<void> {
  console.log("\n--- Step 4b: Promise.race (first settles wins) ---");

  const fast = delay(20).then(() => "fast");
  const slow = delay(100).then(() => "slow");

  const start = Date.now();
  const winner = await Promise.race([fast, slow]);
  const elapsed = Date.now() - start;
  console.log(`  Winner: ${winner} in ${elapsed}ms`);
  console.log("  (the slower promise continues but its result is ignored)");

  // Timeout pattern using Promise.race
  async function withTimeout<T>(
    promise: Promise<T>,
    ms: number,
  ): Promise<T> {
    const timeout = new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error(`timeout after ${ms}ms`)), ms),
    );
    return Promise.race([promise, timeout]);
  }

  try {
    await withTimeout(delay(100), 30);
  } catch (err) {
    console.log(`  Timeout pattern: ${(err as Error).message}`);
  }
}

// ============================================================================
// Step 4c: Promise.allSettled — collect results regardless of outcome
// ============================================================================

async function step4cAllSettled(): Promise<void> {
  console.log("\n--- Step 4c: Promise.allSettled (wait for all, collect all) ---");

  const results = await Promise.allSettled([
    Promise.resolve("ok"),
    Promise.reject(new Error("fail")),
    delay(20).then(() => "delayed ok"),
  ]);

  for (const r of results) {
    if (r.status === "fulfilled") {
      console.log(`  Fulfilled: ${r.value}`);
    } else {
      console.log(`  Rejected: ${r.reason}`);
    }
  }
}

// ============================================================================
// Step 4d: Error Handling in Async Functions
// ============================================================================

async function fallibleCompute(shouldFail: boolean): Promise<string> {
  await delay(10);
  if (shouldFail) {
    throw new Error("computation failed");
  }
  return "computation succeeded";
}

async function step4dErrorHandling(): Promise<void> {
  console.log("\n--- Step 4d: Error Handling ---");

  // try/catch with await — clean error handling
  try {
    const ok = await fallibleCompute(false);
    console.log(`  Success: ${ok}`);
  } catch (err) {
    console.log(`  Unexpected error: ${err}`);
  }

  // catch() on the promise — chain style
  const result = await fallibleCompute(true).catch(
    (err) => `fallback: ${(err as Error).message}`,
  );
  console.log(`  After catch: ${result}`);

  // Promise.all with error handling — individual catch to isolate
  const [a, b] = await Promise.all([
    fallibleCompute(false).catch((_) => "default-a"),
    fallibleCompute(true).catch((_) => "default-b"),
  ]);
  console.log(`  Isolated errors: a=${a}, b=${b}`);
}

// ============================================================================
// Step 4e: Async Iterator (Bonus — streaming async values)
// ============================================================================

async function* asyncCounter(upTo: number): AsyncGenerator<number> {
  for (let i = 1; i <= upTo; i++) {
    await delay(10);
    yield i;
  }
}

async function step4eAsyncIterator(): Promise<void> {
  console.log("\n--- Step 4e: Async Iterator (bonus) ---");
  const values: number[] = [];
  for await (const n of asyncCounter(3)) {
    values.push(n);
  }
  console.log(`  Async iterated: [${values}] (each yielded after 10ms)`);
}

// ============================================================================
// Main — run all steps sequentially
// ============================================================================

async function main(): Promise<void> {
  console.log("=== Phase 13.11: Futures, Promises, async/await (TypeScript) ===\n");

  step1ManualPromise();

  // Allow SimplePromise callbacks to fire before moving on
  await new Promise((r) => setTimeout(r, 60));

  await step2AsyncAwait();
  await step3EventLoop();

  // Allow the event loop to flush before continuing
  await delay(10);

  await step4aPromiseAll();
  await step4bPromiseRace();
  await step4cAllSettled();
  await step4dErrorHandling();
  await step4eAsyncIterator();

  console.log("\n=== All steps completed. ===");
  console.log("Key insight: JavaScript Promises are *eager* — the executor");
  console.log("runs immediately in the constructor. Async/await is syntactic");
  console.log("sugar over .then() chains, scheduled as microtasks on the event loop.");
}

main().catch(console.error);
