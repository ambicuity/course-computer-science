# Side-Channels — Timing, Cache, Spectre/Meltdown

> Your encryption may be mathematically unbreakable. Your implementation probably isn't.

**Type:** Build
**Languages:** C
**Prerequisites:** Phase 12 lessons 01–18, Phase 06 (cache hierarchy, pipelining, branch prediction), Phase 02 (pointers, memory)
**Time:** ~75 minutes

## Learning Objectives

- Implement a timing attack that recovers a secret byte-by-byte from a variable-time `memcmp`.
- Write a constant-time comparison that eliminates timing leakage and verify its timing independence.
- Demonstrate a Flush+Reload cache side-channel attack and measure the hit/miss timing gap.
- Explain how Spectre v1 bypasses bounds checks via speculative execution and how it differs from Meltdown.

## The Problem

Your encryption algorithm might be mathematically perfect — AES-256, Kyber-1024, whatever — but still leak the key through **physical channels** the algorithm's designer never thought about. A simple `if (secret_bit)` branching on a secret value creates a timing difference measurable over a network. A cache-miss pattern reveals which T-table entry was accessed — and therefore the key byte. These are **side-channel attacks**, and they have broken real-world cryptography repeatedly.

Consider: OpenSSL's RSA implementation on a 2003 Apache server. An attacker on the same network measures response times and extracts the private RSA key bit by bit, using nothing but a millisecond-resolution clock and statistical analysis. This was not theoretical — Brumley and Boneh did it in 2003. The algorithm (RSA) was fine. The implementation leaked timing information through the Chinese Remainder Theorem's conditional reduction.

The phase capstone (TLS 1.3 library plus mini-CTF toolkit) requires you to understand side-channels because CTF challenges frequently include them: a `strcmp` timing oracle that reveals an admin cookie byte-by-byte, a padding oracle that leaks plaintext through response timing, or a Flush+Relave attack across Docker containers. More importantly, your own TLS implementation in the capstone must be constant-time — if it isn't, the CTF toolkit's other half (the attack scripts) will break it.

## The Concept

### Types of Side-Channels

| Channel | Mechanism | Classic Target |
|---------|-----------|---------------|
| **Timing** | Execution time depends on secret data | `memcmp` returning early on mismatch; variable-time RSA exponentiation |
| **Cache** | Memory access patterns reveal which addresses were accessed | AES T-table lookups indexed by key byte XOR plaintext |
| **Power** | Power consumption varies with operations | Square vs multiply in RSA; key bit through power trace |
| **Electromagnetic** | EM emissions correlate with instruction execution | Smartcard key extraction at 2 cm distance |
| **Acoustic** | CPU coil whine varies with computation | RSA key extraction from laptop microphone |
| **Microarchitectural** | CPU state leaks across security boundaries | Spectre/Meltdown reading kernel memory from userspace |

### Timing Attacks on Crypto

The simplest timing leak is a comparison function that returns on the first mismatch:

```
naive_memcmp("secret_key_x", "secret_key_a") → returns after 1 comparison (byte 0: 'x' ≠ 'a')
naive_memcmp("secret_key_X", "secret_key_A") → returns after 1 comparison (byte 0: same, byte 1: different)
```

The attacker tries all 256 values for position 0. The one that takes longest is correct (because the comparison continues to byte 1). Repeat for all positions. This recovers a 16-byte secret with ~256×16 measurements.

Real-world examples:
- **HMAC timing (Lucky 13):** TLS CBC padding oracles via timing differences in HMAC comparison
- **RSA timing:** Square-and-multiply exponentiation leaks the private exponent bit through multiplication timing
- **Ladder leaks:** Montgomery ladder in ECC has different power profiles for dummy vs real operations

**Mitigation:** Constant-time code — the comparison takes the same number of operations regardless of where the mismatch occurs.

```
constant_time_memcmp("secret_key_x", "secret_key_a") → XOR all bytes, OR results together → same timing always
```

### Cache Side-Channel Attacks

Caches are fast memory that holds recently accessed data. An access to cached data takes ~50 cycles; an access to main memory takes ~200-400 cycles. This timing gap is the basis for three classic attacks:

**Prime+Probe:** Attacker fills cache lines with own data → victim runs and displaces some → attacker measures which of its own lines were evicted.

**Flush+Reload:** Attacker flushes a shared cache line → victim accesses it (bringing it back to cache) → attacker measures reload time to determine if victim accessed it.

```
  Attacker flushes → Victim accesses line N → Attacker reloads all lines
  Line N: fast (cache hit — victim brought it back)
  Other lines: slow (cache miss)
```

**Evict+Time:** Attacker fills specific cache sets → victim runs → attacker measures timing difference compared to unfilled baseline.

These attacks break AES in practice: the last-round T-table lookup is `Table[plaintext_byte XOR key_byte]`. If the attacker knows the plaintext and can observe which cache line was accessed (because the table entry lands in a specific cache set), they recover the key byte.

### Spectre

**Spectre v1 (Bounds Check Bypass, CVE-2017-5753):**

```
if (x < array1_size) {           // bounds check
    y = array2[array1[x] * 64];  // speculative access
}
```

On a modern CPU, this compiles to:
1. Load `array1_size` from memory
2. Compare `x` with `array1_size`
3. Branch if `x >= array1_size` to skip body
4. Execute body (speculatively if the branch hasn't resolved yet)

If the attacker:
- **Trains** the branch predictor to always take the "x < array1_size" branch (by calling with valid x hundreds of times)
- **Flushes** `array1_size` from cache (so step 1 takes ~300 cycles)
- Calls with `x = malicious_x` where `malicious_x >= array1_size`

The CPU speculatively executes the body before the bounds check resolves. It reads `array1[malicious_x]` (out of bounds!) and uses that value to index `array2`. The cache now contains `array2[array1[malicious_x] * 64]`. The attacker probes `array2` via Flush+Reload to discover which line is cached, leaking the byte at `array1[malicious_x]`.

**Spectre v2 (Branch Target Injection, CVE-2017-5715):** Poi-son the indirect branch predictor (BTB) to redirect speculative execution to a gadget of the attacker's choice.

### Meltdown

Meltdown (CVE-2017-5754) exploits **out-of-order execution** rather than branch prediction:

```
// This should fault — but the CPU executes out-of-order
kernel_data = *kernel_address;
// The fault hasn't been raised yet; use kernel_data
probe_array[kernel_data * 64] = 1;  // this line is cached!
// Now the fault arrives, SIGSEGV handler runs
// But the cache side effect remains — attacker probes probe_array
```

Meltdown reads kernel memory directly from userspace by racing the fault handler with out-of-order execution. It affects Intel CPUs (and some Arm) but not AMD. The fix is **Kernel Page Table Isolation (KPTI)** — kernel pages are unmapped from userspace page tables entirely, so even speculative execution cannot access them.

### Mitigations Summary

| Attack | Software Mitigation | Hardware Mitigation |
|--------|--------------------|--------------------|
| Timing | Constant-time code (no secret-dependent branches/memory) | — |
| Cache | Cache line masking (read all possible lines, use one) | — |
| Spectre v1 | `lfence`/speculation barriers after bounds checks | Microcode + IBRS |
| Spectre v2 | Retpolines (return trampoline) | Microcode + STIBP |
| Meltdown | KPTI (kernel page table isolation) | CPU redesign |
| Hertzbleed | Constant-time even for frequency-scaled operations | Microcode + MSR isolation |

## Build It

All code is in `code/main.c`. Compile with:

```bash
gcc -O2 -o sidechannel main.c
```

The program demonstrates four side-channel techniques in sequence.

### Step 1: Timing Attack on Variable-Time Comparison

A naive `memcmp` that returns on the first mismatch creates a measurable timing signal. The attack recovers a secret byte-by-byte using only timing measurements.

```c
int naive_memcmp(const void *a, const void *b, size_t len) {
    const uint8_t *pa = a, *pb = b;
    for (size_t i = 0; i < len; i++) {
        if (pa[i] != pb[i]) return -1;
    }
    return 0;
}
```

The timing gradient for mismatch at position 0 vs position 15 is measured with cycle-accurate RDTSC instructions. Then the full secret recovery tries all 256 values per byte and picks the one with the longest average comparison time.

Key implementation details:
- `_mm_lfence()` serializes the instruction stream around `__rdtsc()` to prevent out-of-order execution from corrupting measurements
- Multiple trials are averaged to reduce noise from interrupts and context switches
- Cache lines are flushed between trials to ensure consistent starting conditions

### Step 2: Constant-Time Comparison

Replace the early-return loop with XOR-OR accumulation:

```c
int constant_time_memcmp(const void *a, const void *b, size_t len) {
    const uint8_t *pa = a, *pb = b;
    uint8_t diff = 0;
    for (size_t i = 0; i < len; i++)
        diff |= (pa[i] ^ pb[i]);
    return diff;
}
```

Every byte is processed regardless of match position. The timing measurement loop from Step 1 is reused to verify that the timing is now independent of where the mismatch occurs.

### Step 3: Cache Side-Channel — Flush+Reload

A shared buffer is allocated with cache-line-sized granularity:

```c
volatile uint8_t shared_array[256 * 64];  // 256 cache lines
```

The victim function accesses `shared_array[secret_byte * 64]`, bringing that cache line into L1. The spy function:
1. Flushes all cache lines with `_mm_clflush()`
2. Calls the victim
3. Measures reload time for each line — the accessed line is fastest

```c
for (int i = 0; i < 256; i++) {
    _mm_clflush(&shared_array[i * 64]);  // flush everything
}
victim(secret_byte);                       // access one line
for (int i = 0; i < 256; i++) {
    uint64_t t = __rdtsc();
    sink = shared_array[i * 64];           // reload — cached if victim touched it
    uint64_t elapsed = __rdtsc() - t;
    // The lowest elapsed time reveals which line was accessed
}
```

The calibration step prints cache hit vs miss times to confirm the timing gap is detectable on your hardware.

### Step 4: Spectre v1 — Bounds Check Bypass (Conceptual)

This demonstration shows the Spectre v1 gadget structure even if modern CPU mitigations prevent the actual leak. A bounds-checked access is trained with valid indices, then probed with an out-of-bounds index:

```c
__attribute__((noinline))
void spectre_victim(size_t x) {
    if (x < array1_size) {
        sink = array2[array1[x] * CACHE_LINE_SIZE];
    }
}
```

The training loop calls `spectre_victim(0)` hundreds of times to prime the branch predictor. Then `array1_size` is flushed from cache, and `spectre_victim(16)` is called. During the ~300 cycles while `array1_size` is being fetched, a CPU without mitigations would speculatively execute the body with `x = 16`, reading `array1[16]` (out of bounds) and caching `array2[array1[16]]`. The probe loop detects which `array2` entry is cached.

On modern CPUs with IBRS and microcode updates, this typically fails — but the code structure is identical to the original proof-of-concept. The demonstration educates on the mechanism rather than reliably exploiting it.

## Use It

Real-world side-channel attacks that changed the industry:

- **Dhem et al. (1998)** — First timing attack on RSA, extracting a private key from a smartcard using time measurements.
- **Brumley & Boneh (2003)** — Remote timing attack on OpenSSL RSA over a network, recovering the private key from an Apache server.
- **Osvik et al. (2006)** — Cache attack on AES using Prime+Probe on Linux; extracted an AES key from a co-located VM.
- **Alpaca (2020)** — Timing attack on LLM integration points via cache side-channels.
- **Lucky 13 (2013)** — TLS CBC padding oracle via HMAC timing differences (fixed in TLS 1.3).
- **Spectre/Meltdown (2018)** — Microarchitectural side-channels affecting billions of devices, forcing OS vendors to deploy KPTI, retpolines, and microcode updates.
- **Hertzbleed (2022)** — Frequency-scaling side-channel on Intel/AMD CPUs; power management leaks secret data through timing even in constant-time code.

Production mitigations:
- **OpenSSL:** `CRYPTO_memcmp()` in `crypto/constant_time_locl.h` — constant-time comparison, always processes all bytes.
- **libsodium:** `sodium_memcmp()` and `sodium_mlock()` — constant-time comparison and memory locking.
- **Linux kernel:** KPTI in `arch/x86/mm/tlb.c` — unmaps kernel pages from userspace page tables.
- **GCC/Clang:** `__builtin_constant_p()` for compile-time detection of constant-time requirements.
- **Intel microcode:** IBRS (Indirect Branch Restricted Speculation), STIBP (Single Thread Indirect Branch Predictors), SSBD (Speculative Store Bypass Disable).

## Read the Source

- **OpenSSL `crypto/constant_time_locl.h`** — constant-time utility functions used throughout the library; look at `constant_time_eq()` and `constant_time_select()`.
- **libsodium `src/libsodium/sodium/utils.c`** — `sodium_memcmp` and `sodium_mlock`; shows how production crypto libraries handle timing-safe comparison.
- **Linux kernel `arch/x86/mm/tlb.c`** — KPTI implementation that unmaps kernel pages from userspace during syscall entry/exit.
- **Spectre paper** ("Spectre Attacks: Exploiting Speculative Execution") and **Meltdown paper** ("Meltdown: Reading Kernel Memory from User Space") — the original disclosures, still the best explanations of the mechanisms.
- **Google's Tink library `cc/util/secret_data.h`** — constant-time utilities and secure memory management.
- **GCC `__builtin_constant_p()` docs** — compiler intrinsic for detecting whether an expression is compile-time constant, used to guide optimization without breaking constant-time guarantees.

## Ship It

The reusable artifact is a **side-channel analysis toolkit** in `outputs/`. It demonstrates:
- Variable-time comparison attack and secret recovery
- Constant-time comparison verification
- Flush+Reload cache side-channel detection
- Spectre v1 gadget structure

This toolkit serves as a reference for building constant-time code in the TLS 1.3 capstone and for recognizing side-channel attack surfaces in CTF challenges.

## Exercises

1. **Easy** — Run the compiled program on an Intel x86 machine. Observe the timing gradient for `naive_memcmp`: how many cycles does each additional matching byte add? How many cycles does the `constant_time_memcmp` show for different mismatch positions? Explain the difference.

2. **Medium** — Extend the timing attack to recover a secret twice as long (32 bytes). Modify the code and run the attack. Does the accuracy change? What happens if you reduce the number of trials per byte — at what point does the recovery start failing? Plot accuracy vs trial count.

3. **Hard** — Implement a **Prime+Probe** attack on a co-located VM or container. Use a shared library and monitor which cache sets a victim process evicts. Alternatively, implement a constant-time AES S-box lookup using bit-slicing or cache-line masking (read all 16 S-box entries, select one with bitwise operations). Compare its performance against a T-table implementation.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Side-channel | "Attacks that aren't on the algorithm" | Leaking secret data through physical effects of computation — timing, power, EM, cache state |
| Timing attack | "Watch how long it takes" | Measuring execution time variation caused by secret-dependent branches or operations |
| Constant-time | "Takes the same time always" | Code whose execution path and memory access pattern are independent of secret data; no secret-dependent branches, no secret-dependent array indices |
| Cache attack | "Read the cache" | Exploiting the 3-10x speed difference between cached and uncached memory to infer victim memory access patterns |
| Flush+Reload | "Flush, wait, measure" | Attacker flushes a shared cache line, victim may reload it, attacker measures reload time to detect access |
| Prime+Probe | "Fill, wait, check" | Attacker fills cache sets with own data, victim displaces some, attacker checks which sets were evicted |
| Spectre | "Trick the CPU" | Exploiting branch prediction and speculative execution to access data that should be out of bounds, leaking it through cache state |
| Meltdown | "Read kernel memory" | Exploiting out-of-order execution to access privileged memory before the fault handler runs, leaking it through cache state |
| Branch predictor | "CPU guesses the branch" | Hardware unit that predicts which way a conditional branch will go; trained by past execution history |
| Speculative execution | "CPU guesses the future" | CPU executes instructions before their inputs are resolved, then commits or discards results based on actual resolution |
| KPTI | "Kernel page table isolation" | OS mitigation for Meltdown: kernel pages are unmapped from userspace page tables so speculative execution cannot access them |
| Retpoline | "Return trampoline" | Compiler mitigation for Spectre v2: replaces indirect branches with return instructions that don't use the BTB |
| Cache line | "Smallest cache unit" | 64 bytes on x86; the granularity at which data moves between cache and memory |
| T-table | "AES lookup table" | Precomputed table combining SubBytes, ShiftRows, and MixColumns; cache-timing attacks on T-table lookups reveal AES keys |

## Further Reading

- "The Microarchitecture of Superscalar CPUs" — Patterson & Hennessy, *Computer Architecture: A Quantitative Approach*; the definitive reference on branch prediction, speculative execution, and cache hierarchies.
- "Cache Attacks and Countermeasures: The Case of AES" by Osvik et al. (2006) — the paper that demonstrated practical cache attacks on AES; explains Prime+Probe in detail.
- "Remote Timing Attacks are Practical" by Brumley & Boneh (2003) — the seminal paper on network-based timing attacks against OpenSSL RSA.
- "Spectre Attacks: Exploiting Speculative Execution" and "Meltdown: Reading Kernel Memory from User Space" (2018) — the original disclosure papers; read the Spectre paper for the v1 gadget and the Meltdown paper for the out-of-order race.
- "Timing Attacks on Implementations of Diffie-Hellman, RSA, DSS, and Other Systems" by Paul Kocher (1996) — the paper that invented timing attacks; still the clearest explanation of the basic principle.
- "BearSSL" by Thomas Pornin — a constant-time SSL/TLS library with extensive documentation on constant-time coding; the `src/int/` directory shows constant-time big-integer arithmetic.
