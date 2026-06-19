# Debug Checklist — When a Binary Crashes in Production

## Phase 1: Capture the corpse

```
[ ] Confirm a core dump exists.
    Linux:   coredumpctl list | head
             coredumpctl info <pid|exe>
             ls /var/lib/systemd/coredump/
    Manual:  ulimit -c unlimited; pattern in /proc/sys/kernel/core_pattern
    macOS:   ls /cores/

[ ] Match the binary with the core.
    The binary's build-id must match what the core references.
    `eu-unstrip -n core` (Linux)  or  `dwarfdump --uuid core` (macOS)

[ ] Make sure the binary has debug symbols available.
    Either:
      - the dev binary is the one that crashed,
      - or you have a separate symbol package (e.g., .deb-dbgsym), or
      - you saved the binary's .pdb / .dSYM at build time.
```

## Phase 2: Load and stack-trace

```
[ ] gdb <binary> <core>      (Linux)
    lldb <binary> -c <core>  (macOS)

[ ] bt           → full call stack at the moment of death
[ ] bt full      → with locals per frame
[ ] info threads → show all threads (often the crashing thread is not main)
[ ] thread N     → switch to a specific thread
[ ] frame N      → switch to frame N (in the current thread)
[ ] info args, info locals
[ ] disassemble  → see the actual instructions at the crash PC
```

## Phase 3: Triage hypotheses

For each "I think it's X" hypothesis, look for evidence:

| Hypothesis | Evidence to look for |
|------------|----------------------|
| Null deref      | rip near a load/store; `info reg` shows the bad address is 0x0 or near-0 |
| Use-after-free  | The faulting pointer is in a region the allocator already reclaimed |
| Stack overflow  | rsp near a guard page; `bt` is hundreds of frames deep |
| Buffer overrun  | A neighbor variable in the stack frame has nonsense; canary mismatch |
| Race            | The crash only reproduces under load; multiple threads in `bt` |
| Bad cast        | Vtable / type tag is wrong; `p *this` shows nonsense |

## Phase 4: Reproduce

```
[ ] Can you reproduce locally?
    Same toolchain version? Same compile flags? Same env?
[ ] Is there a regression range?
    git bisect on commits since the last green build (see Lesson 03)
[ ] Can the test be promoted to a permanent regression test?
```

## Phase 5: Fix and verify

```
[ ] Patch.
[ ] Reproduce the original crash with the patch reverted — confirm the patch
    is necessary.
[ ] Run the patched binary under sanitizers (AddressSanitizer / UBSan / TSan)
    on the same input that caused the crash, looking for adjacent issues.
[ ] Add a test that would have caught this. If a test can't catch it,
    add a comment + assertion in the code instead.
```
