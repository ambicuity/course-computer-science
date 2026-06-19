# Lesson 12: File Systems II — ext4, btrfs, ZFS deep cuts

## Why This Matters

Lesson 11 gave you the universal concepts — VFS, inodes, journaling. But production file systems differ enormously in how they organize blocks, handle corruption, snapshot data, and scale. Choosing ext4 vs btrfs vs ZFS is a real engineering decision: it determines your data integrity guarantees, performance profile, operational complexity, and even licensing constraints. This lesson dissects the three dominant Linux/Unix file systems so you can make informed choices.

## ext4 — The Workhorse

ext4 is the default Linux file system since 2008. It evolved from ext2 → ext3 (added journaling) → ext4 (added extents, delayed allocation, large file support).

### Architecture

```
┌────────────────────────────────────────────────────┐
│  ext4 Layout                                        │
├────────────┬───────────────────────────────────────┤
│  Superblock│  Block group descriptors               │
│  (backup)  │  (bitmap locations, inode tables)      │
├────────────┴───────────────────────────────────────┤
│  Block Group 0                                       │
│  ┌─────────┬──────────┬───────────┬──────────────┐ │
│  │Superblk │ Inode    │ Inode     │ Data         │ │
│  │+ Group  │ Bitmap   │ Table     │ Bitmap       │ │
│  │ Desc    │          │           │              │ │
│  └─────────┴──────────┴───────────┴──────────────┘ │
│  Block Group 1 ... N (replicated layout)           │
└────────────────────────────────────────────────────┘
```

The disk is divided into **block groups**, each containing its own inode table and data blocks. This keeps metadata close to data, improving locality.

### Extent-Based Allocation

ext3 used per-block pointers (like the inode lesson showed). ext4 uses **extents** — contiguous runs of blocks:

```
ext3 inode (per-block):        ext4 inode (extents):
  [ptr] → block 100             [start=100, len=64]  → blocks 100–163
  [ptr] → block 101             [start=200, len=32]  → blocks 200–231
  [ptr] → block 102             [start=500, len=128] → blocks 500–627
  ... (100 pointers for 100     (3 extent entries cover 224 blocks)
       contiguous blocks)
```

A single extent entry can describe 128 MB (32K blocks × 4 KB). A 1 GB file needs ~8 extent entries instead of 262,144 block pointers.

### Delayed Allocation (delalloc)

ext4 doesn't allocate blocks to a file immediately when you `write()`. Instead, it waits until the data is flushed to disk (typically every 5 seconds via `pdflush`). This lets the allocator see the full write pattern and make better placement decisions:

1. App calls `write(fd, buf, 1 MB)` — data goes to page cache, no block allocated yet
2. 5 seconds later, flusher thread writes 1 MB in one extent
3. Result: one extent instead of thousands of fragmented blocks

### Journal

ext4 uses **JBD2** (Journaling Block Device v2):

```
Journal layout:
┌──────────────────────────────────────┐
│  Descriptor block  (metadata tags)   │
│  Metadata block 1  (inode table)     │
│  Metadata block 2  (bitmap)          │
│  Commit block      (checksum + tag)  │
└──────────────────────────────────────┘

Modes:
  journal   → log data + metadata (safest, slowest)
  ordered   → log metadata, write data first (default)
  writeback → log metadata, data anytime (fastest, riskiest)
```

### Key Specs

| Property | Value |
|----------|-------|
| Max file size | 16 TiB (1 EB theoretical) |
| Max volume size | 1 EiB |
| Max filename | 255 bytes |
| Timestamp resolution | nanoseconds |
| Online resize | grow only (offline shrink) |
| Checksum | metadata only (optional, ext4 metadata_csum) |

## btrfs — The Feature-Rich Contender

btrfs (B-tree file system, Oracle, 2009) was designed to address ext4's limitations. It uses copy-on-write (CoW) everywhere, making snapshots and checksumming inherent to the design.

### Architecture

```
┌────────────────────────────────────────────────────┐
│  btrfs B-tree Structure                             │
│                                                     │
│  Root tree (fs tree roots, chunk tree, dev tree)   │
│     │                                               │
│     ├── Chunk tree (logical → physical mapping)    │
│     ├── Device tree (device info, free space)      │
│     ├── FS tree 0 (subvolume / snapshots)          │
│     │     ├── Extent item → data/metadata          │
│     │     ├── Inode item                           │
│     │     └── Dir item → name → inode              │
│     └── FS tree N (another subvolume)              │
└────────────────────────────────────────────────────┘
```

Everything is a B-tree. The root tree points to other trees. Each file system tree (subvolume) contains extents, inodes, and directory entries as B-tree items.

### Copy-on-Write (CoW)

btrfs never overwrites data in place. Every write creates new blocks:

```
Before write:
  File block → physical block 500

Write to block:
  1. Allocate new block (e.g., block 800)
  2. Write new data to block 800
  3. Update parent B-tree to point to block 800
  4. Free block 500 (if no snapshots reference it)

Snapshots:
  - Snapshot = clone of the root B-tree pointer
  - Both old and new version share unchanged blocks
  - Only modified blocks consume new space
  - Creating a snapshot is O(1) — just copy one pointer
```

This is fundamentally different from ext4, which overwrites blocks in place.

### Subvolumes and Snapshots

```
btrfs filesystem:
  /
  ├── @          ← root subvolume
  ├── @home      ← separate subvolume for /home
  ├── @snapshots/
  │     ├── 2024-01-15-00:00  ← snapshot of @ (CoW, instant)
  │     ├── 2024-01-16-00:00  ← only diffs from prev snapshot
  │     └── 2024-01-17-00:00
  └── @swap      ← no-CoW subvolume for swap file
```

Subvolumes are lightweight isolation units. Snapshots are instantaneous and space-efficient because of CoW.

### Checksumming

Every data and metadata block has a CRC32c (or xxhash/blake2) checksum. On read, btrfs verifies the checksum:

- **Detects bit rot**: silent corruption caught immediately
- **Self-healing (with RAID1/10)**: if checksum fails, read the mirror copy
- ext4 has no data checksumming by default

### RAID Support

btrfs has built-in RAID at the file system level (no mdadm needed):

| Profile | Copies | Min disks | Tolerance |
|---------|--------|-----------|-----------|
| RAID 0 | 1 | 2 | None (striping) |
| RAID 1 | 2 | 2 | 1 disk failure |
| RAID 10 | 2 | 4 | 1 per mirror |
| RAID 5/6 | parity | 3/4 | 1/2 disk failures |

RAID 5/6 in btrfs is **not recommended for production** due to the write hole problem.

### Compression

btrfs supports transparent compression (lzo, zstd) set per-file or per-subvolume:

```
$ btrfs property set /data compression zstd
$ btrfs filesystem defragment -czstd /data
```

Typical compression ratios: 1.5–3x for text/code, minimal for already-compressed formats.

### Key Specs

| Property | Value |
|----------|-------|
| Max file size | 16 EiB |
| Max volume size | 16 EiB |
| Max filename | 255 bytes |
| Checksums | CRC32c, xxhash, blake2, sha256 |
| Compression | lzo, zstd |
| Online defrag | Yes |
| Send/receive | Yes (incremental backup) |

## ZFS — The Enterprise Standard

ZFS (Sun Microsystems, 2005) combines volume management and file system in one. It's the gold standard for data integrity and scales to hundreds of petabytes.

### Architecture

```
┌────────────────────────────────────────────────────┐
│  ZFS Pooled Storage                                 │
│                                                     │
│  zpool "tank"                                       │
│  ┌──────────────────────────────────────────────┐  │
│  │  vdev 0: mirror (disk0 + disk1)              │  │
│  │  vdev 1: RAIDZ2 (disk2-disk5, dual parity)  │  │
│  │  vdev 2: mirror (disk6 + disk7)              │  │
│  └──────────────────────────────────────────────┘  │
│          │                                          │
│  Space is pooled across all vdevs                   │
│          │                                          │
│  Dataset: tank/home  (like a btrfs subvolume)      │
│  Dataset: tank/vm    (recordsize=128K, no CoW)     │
│  ZVOL:   tank/swap   (block device)                │
└────────────────────────────────────────────────────┘
```

**Key concepts**:
- **vdev**: a virtual device (single disk, mirror, RAIDZ, RAIDZ2, RAIDZ3)
- **zpool**: a pool of vdevs — all space is shared
- **Dataset**: a file system within the pool (like a subvolume)
- **ZVOL**: a raw block device within the pool (for swap, VMs)

### Pooled Storage

Unlike traditional file systems, ZFS doesn't partition disks. All vdevs contribute to one pool:

```
Traditional:   disk0 = /home, disk1 = /var, disk2 = /data
               (waste if /home is full but /var is empty)

ZFS:           zpool = disk0 + disk1 + disk2
               (any dataset can use any free space in the pool)
```

No fixed partition sizes. Datasets grow and shrink dynamically.

### Copy-on-Write + Transactional

Like btrfs, ZFS never overwrites in place. Every write is part of an atomic **transaction group** (txg):

```
Txg lifecycle (every ~5 seconds):
  1. Open  → start accepting writes
  2. Quiesce → flush dirty data to disk
  3. Sync   → write uberblock (atomic root pointer)
  4. Free   → release old blocks

Crash during any step:
  - If uberblock not updated → revert to previous txg (consistent)
  - If uberblock updated → new state is complete (consistent)
  - No partial writes possible
```

This is more robust than ext4's journal — ZFS is always consistent, even without a separate journal.

### Snapshots, Clones, and Send/Receive

```
Snapshot:  read-only point-in-time copy (CoW, instant)
Clone:     writable copy of a snapshot (shares unchanged blocks)

$ zfs snapshot tank/home@backup-2024-01-15
$ zfs clone tank/home@backup-2024-01-15 tank/home-clone
$ zfs send tank/home@backup-2024-01-15 | ssh remote zfs receive backup/home
```

`zfs send | zfs receive` enables incremental backup: only changed blocks are transmitted.

### Deduplication

ZFS can deduplicate blocks at write time using a DDT (Dedup Table):

```
Without dedup:  write 1000 VMs with identical OS → 1000 × OS size
With dedup:     write 1000 VMs with identical OS → 1 × OS size (999 references)
```

**Warning**: dedup requires massive RAM (5 GB per TB of deduped data). Most deployments avoid it and rely on compression instead.

### ARC Cache

ZFS uses **Adaptive Replacement Cache** (ARC), which outperforms LRU:

```
ARC = MRU (most recently used)
    + MFU (most frequently used)
    + ghost MRU (recently evicted from MRU)
    + ghost MFU (recently evicted from MFU)

Eviction: if MRU hit rate drops, prefer MFU; and vice versa.
```

ARC adapts to workload patterns automatically, unlike pure LRU which thrashes on scan-heavy workloads.

### Scrubbing

`zfs scrub` reads every block and verifies checksums:

```
$ zfs scrub tank
  scan: scrub repaired 0B in 2:15:00 with 0 errors
```

Run monthly on production systems to catch silent corruption before it spreads.

### Key Specs

| Property | Value |
|----------|-------|
| Max file size | 16 EiB |
| Max volume size | 256 ZiB (theoretical) |
| Checksums | Fletcher-4, SHA-256 |
| Compression | lz4, zstd, gzip |
| RAID | RAIDZ, RAIDZ2, RAIDZ3 |
| Dedup | Yes (expensive) |
| Encryption | Native (AES-256-GCM) |

## Comparison

| Feature | ext4 | btrfs | ZFS |
|---------|------|-------|-----|
| CoW | No | Yes | Yes |
| Snapshots | No | Yes (instant) | Yes (instant) |
| Subvolumes | No | Yes | Yes (datasets) |
| Data checksums | No | CRC32c + others | Fletcher-4 / SHA-256 |
| Self-healing | No | Yes (RAID) | Yes (RAIDZ) |
| Compression | No | lzo, zstd | lz4, zstd, gzip |
| Built-in RAID | No | Yes (raid5 risky) | Yes (RAIDZ/Z2/Z3) |
| Dedup | No | No | Yes |
| Max volume | 1 EiB | 16 EiB | 256 ZiB |
| Maturity | 16 years | 15 years | 19 years |
| Linux default | Yes (2008) | SUSE default | Third-party (OpenZFS) |
| License | GPL | GPL | CDDL (not mainline) |
| Complexity | Low | Medium | High |
| Performance | Best for simple workloads | Good, CoW overhead | Excellent at scale |

## Trade-Off Analysis

### ext4: Simple and Stable

```
Choose ext4 when:
  ✓ You need a reliable, well-tested default
  ✓ Performance on simple workloads matters most
  ✓ No snapshots or checksumming needed
  ✓ Boot/root filesystem (grub/systemd support)
  ✓ Embedded / resource-constrained systems

Avoid ext4 when:
  ✗ You need snapshots or rollback
  ✗ Silent corruption detection matters
  ✗ You want transparent compression
```

### btrfs: Features with Trade-offs

```
Choose btrfs when:
  ✓ Snapshots for backup/rollback (snapper, timeshift)
  ✓ Transparent compression saves significant space
  ✓ Subvolumes for flexible space sharing
  ✓ Checksumming for data integrity
  ✓ Desktop/laptop use (SUSE default)

Avoid btrfs when:
  ✗ Database workloads (CoW amplifies random writes)
  ✗ You need proven RAID5/6
  ✗ Kernel < 5.15 (many btrfs improvements landed late)
```

### ZFS: Enterprise Power, Licensing Pain

```
Choose ZFS when:
  ✓ Data integrity is non-negotiable (NAS, archival)
  ✓ Pooled storage across many disks
  ✓ Snapshots + send/receive for backup pipeline
  ✓ Dedup (only if you have massive RAM budget)
  ✓ FreeBSD / TrueNAS (native ZFS)

Avoid ZFS when:
  ✗ You need mainline kernel support (CDDL vs GPL)
  ✗ RAM is severely limited (ARC wants 1 GB+ minimum)
  ✗ You need the simplest possible setup
```

## Performance Characteristics

### Sequential I/O

| Workload | ext4 | btrfs | ZFS |
|----------|------|-------|-----|
| Large sequential write | Excellent (extents + delalloc) | Good (CoW overhead) | Excellent (txg batching) |
| Large sequential read | Excellent | Good | Excellent (ARC) |
| Metadata-heavy (ls -R) | Good | Good (B-tree) | Good (B-tree) |

### Random I/O

| Workload | ext4 | btrfs | ZFS |
|----------|------|-------|-----|
| Database writes | Best (no CoW) | Poor (CoW amplification) | Tunable (recordsize) |
| Small random reads | Good | Good | Excellent (ARC/MFU) |

### Space Efficiency

| Scenario | ext4 | btrfs | ZFS |
|----------|------|-------|-----|
| Snapshots | N/A | ~0 incremental | ~0 incremental |
| Compression | N/A | 1.5–3x typical | 1.5–3x typical |
| Dedup | N/A | N/A | Up to 10x (if applicable) |

## Operational Notes

### ext4 maintenance
```bash
# Defragment (offline)
e4defrag /dev/sda1

# Check and repair
fsck.ext4 -f /dev/sda1

# Grow (online)
resize2fs /dev/sda1
```

### btrfs maintenance
```bash
# Scrub
btrfs scrub start /mnt

# Balance (reclaim space from deleted snapshots)
btrfs balance start /mnt

# Defragment (online, loses CoW snapshots)
btrfs filesystem defragment -r /mnt
```

### ZFS maintenance
```bash
# Scrub (monthly recommended)
zfs scrub tank

# Check pool health
zpool status tank

# Send incremental snapshot
zfs send -i tank@snap1 tank@snap2 | ssh backup zfs receive backup/tank
```

## Exercises

### Level 1 — Recall

Compare ext4's journaling with ZFS's transaction groups. Why does ZFS not need a separate journal?

### Level 2 — Application

You're building a NAS with 8 disks for storing video archives (large files, rarely modified, integrity matters). Which file system do you choose and why? What RAID profile? Justify your answer with specific features from the comparison table.

### Level 3 — Build

Research OpenZFS on Linux. Write a script that:
1. Creates a zpool from loopback devices (or files)
2. Creates two datasets with different record sizes
3. Creates snapshots of both
4. Runs a benchmark (fio or dd) and compares performance
5. Destroys one snapshot and verifies the other is intact

Document what you learn about ZFS's operational characteristics.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Extent | "Contiguous block range" | A (start, length) pair describing a run of blocks — more compact than per-block pointers |
| Delalloc | "Delayed allocation" | Deferring block allocation until flush time, allowing better placement decisions |
| CoW | "Copy-on-write" | Never overwrite in place; write new blocks and atomically update pointers |
| Subvolume | "Lightweight filesystem" | An independent tree within a btrfs/ZFS pool, sharing free space with siblings |
| Snapshot | "Frozen copy" | A read-only CoW clone — instant to create, space-efficient (only diffs consume space) |
| Txg | "Transaction group" | ZFS's atomic batch of writes, committed every ~5s via uberblock update |
| ARC | "Adaptive cache" | ZFS's cache that balances recency (MRU) and frequency (MFU) with ghost lists |
| Scrub | "Integrity check" | Reading all blocks and verifying checksums to detect silent corruption |
| vdev | "Virtual device" | ZFS building block: a disk, mirror, or RAIDZ group that contributes to a zpool |

## Further Reading

- ext4 wiki: https://ext4.wiki.kernel.org/
- btrfs documentation: https://btrfs.readthedocs.io/
- OpenZFS documentation: https://openzfs.github.io/openzfs-docs/
- Jim Salter, "ZFS vs btrfs vs ext4" (Ars Technica, 2020)
- Matthew Ahrens, "ZFS: The Last Word in File Systems" (Sun, 2008)
