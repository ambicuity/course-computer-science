# Lesson 11: File Systems I — VFS, inodes, journaling

## Why This Matters

Every program reads and writes files. But "file" is an abstraction: a stream of bytes backed by blocks on a disk. The kernel must map filenames to metadata, metadata to disk blocks, and coordinate writes so a crash mid-operation doesn't corrupt the file system. Understanding VFS, inodes, and journaling is how you reason about `open()`, `ls`, `fsck`, and why your data survives power failures.

## The Virtual File System (VFS)

VFS is the kernel's abstraction layer that unifies all file system implementations behind a single interface:

```
User space
    │
    ▼
  syscall: open("/home/user/foo", O_RDONLY)
    │
    ▼
┌─────────────────────────────┐
│          VFS Layer           │
│  ┌────────────────────────┐ │
│  │  struct file_operations │ │  ← function pointer table
│  │  .read  = ext4_read    │ │
│  │  .write = ext4_write   │ │
│  │  .open  = ext4_open    │ │
│  └────────────────────────┘ │
├─────────────────────────────┤
│  ext4  │  xfs  │  btrfs    │  ← file system implementations
├─────────────────────────────┤
│        Block Layer          │
├─────────────────────────────┤
│         Disk / SSD          │
└─────────────────────────────┘
```

**VFS operations** (the common interface):

| Operation | Purpose |
|-----------|---------|
| `open(path, flags)` | Resolve path, return file descriptor |
| `read(fd, buf, len)` | Read bytes from current position |
| `write(fd, buf, len)` | Write bytes at current position |
| `close(fd)` | Release file descriptor |
| `stat(path, buf)` | Get file metadata |
| `mkdir(path)` | Create directory |
| `unlink(path)` | Remove directory entry |

Each file system implements these in its own way, but VFS presents the same interface to user space.

## Inodes — The Heart of a Unix File

An **inode** (index node) is a fixed-size structure storing file metadata. Every file has exactly one inode. Inodes do **not** store the filename — that's the directory's job.

```
┌─────────────────────────────┐
│         Inode #42           │
├─────────────────────────────┤
│  Mode (type + permissions)  │  0100644 = regular file, rw-r--r--
│  UID / GID                  │  1000 / 1000
│  Size (bytes)               │  65536
│  Timestamps                 │  atime, mtime, ctime
│  Link count                 │  2 (original name + hard link)
│  Block pointers             │  ← see below
└─────────────────────────────┘
```

### Block Pointers

An inode has a fixed number of block pointers. A typical ext4 inode stores:

```
Direct blocks (12):
  [ptr0] [ptr1] [ptr2] ... [ptr11]     → 12 × 4KB = 48 KB

Single indirect:
  [ptr → block of 1024 ptrs]            → 1024 × 4KB = 4 MB

Double indirect:
  [ptr → block of 1024 ptrs
         → each points to block of 1024 ptrs]  → 1024² × 4KB = 4 GB

Triple indirect:
  [ptr → block → block → block]         → 1024³ × 4KB = 4 TB
```

This three-level scheme handles files from a few bytes up to terabytes with the same inode structure.

### Directories

A directory is a special file mapping **filenames → inode numbers**:

```
Directory file contents:
┌──────────────────────────────────────────┐
│ inode  │ rec_len │ name_len │ name       │
├────────┼─────────┼──────────┼────────────┤
│ 42     │ 12      │ 1        │ .          │
│ 42     │ 12      │ 2        │ ..         │
│ 43     │ 16      │ 4        │ data       │
│ 44     │ 20      │ 8        │ readme.txt │
└──────────────────────────────────────────┘
```

`open("/home/user/data")` walks the tree: root inode → find "home" → inode → find "user" → inode → find "data" → inode 43.

## File Descriptors

When you call `open()`, the kernel returns a small integer (file descriptor) that indexes into a per-process table:

```
Process file descriptor table:
  fd 0 → stdin  → terminal
  fd 1 → stdout → terminal
  fd 2 → stderr → terminal
  fd 3 → /home/user/data (read, offset=0)
       ↓
  struct file { offset, refcount, inode*, fops* }
       ↓
  VFS → ext4 → block layer → disk
```

## Journaling — Crash Consistency

Without journaling, a power failure during a write can leave the file system inconsistent: the inode says the file is 8 KB but the block bitmap shows the last block as free.

**Journaling** solves this with a write-ahead log:

```
Normal operation:
  1. Write intent: "I'm about to modify inode 42 and block 100"
     → journal (sequential write, fast)
  2. Apply the actual changes to the file system
  3. Mark journal entry as committed

After crash + reboot:
  1. fsck reads the journal
  2. Replays any committed-but-not-applied entries
  3. File system is consistent
```

### Journaling Modes (ext4)

| Mode | Behavior | Performance | Safety |
|------|----------|-------------|--------|
| `journal` | Journal both data AND metadata | Slowest | Most safe |
| `ordered` (default) | Journal metadata only; data written before metadata commit | Fast | Metadata consistent, data is from before crash |
| `writeback` | Journal metadata only; data written anytime | Fastest | Data may be stale after crash |

`ordered` mode is the practical default: it guarantees that the metadata (directory entries, inode pointers) is always consistent, and file data reflects the most recent successful `fsync()`.

### Superblock

The **superblock** is file system metadata stored on disk:

```
┌────────────────────────────────────────┐
│  Superblock                            │
├────────────────────────────────────────┤
│  Total inodes          65536           │
│  Total blocks          1048576         │
│  Block size            4096            │
│  Free inodes           60000           │
│  Free blocks           900000          │
│  Magic number          0xEF53 (ext4)   │
│  Journal inode         8               │
│  Mount count           12              │
│  Max mount count       20              │
└────────────────────────────────────────┘
```

## Build It

We'll build an in-memory file system with inodes, directories, file descriptors, and a simple journal.

### Step 1: Inode and Superblock

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAX_INODES     256
#define MAX_BLOCKS     1024
#define BLOCK_SIZE     512
#define MAX_NAME       64
#define MAX_FD         32
#define MAX_DATA       (12 * BLOCK_SIZE)  /* direct blocks only */

typedef struct {
    int    mode;           /* 0 = free, 1 = dir, 2 = file */
    int    size;
    int    nlinks;
    char   data[MAX_DATA];
} Inode;

typedef struct {
    int    total_inodes;
    int    free_inodes;
    int    block_size;
} Superblock;
```

### Step 2: Directory Entries and File System State

```c
typedef struct {
    char   name[MAX_NAME];
    int    inode_num;
} DirEntry;

typedef struct {
    Superblock sb;
    Inode      inodes[MAX_INODES];
    int        inode_used[MAX_INODES];  /* 0 = free, 1 = used */
    /* Directory contents stored as arrays of DirEntry per dir inode */
    DirEntry   dirs[MAX_INODES][64];
    int        dir_counts[MAX_INODES];
    /* File descriptors */
    struct {
        int in_use;
        int inode_num;
        int offset;
        int flags;  /* O_RDONLY=0, O_WRONLY=1, O_RDWR=2 */
    } fd_table[MAX_FD];
    /* Journal */
    struct {
        char op[16];
        int  inode_num;
        char data[BLOCK_SIZE];
        int  committed;
    } journal[256];
    int journal_head;
} FileSystem;
```

### Step 3: File System Operations

```c
static FileSystem fs;

static void vfs_init(void) {
    memset(&fs, 0, sizeof(fs));
    fs.sb.total_inodes = MAX_INODES;
    fs.sb.free_inodes  = MAX_INODES - 1;  /* root inode */
    fs.sb.block_size   = BLOCK_SIZE;

    /* Root directory (inode 0) */
    fs.inodes[0].mode = 1;  /* directory */
    fs.inode_used[0] = 1;
    strcpy(fs.dirs[0][0].name, ".");
    fs.dirs[0][0].inode_num = 0;
    strcpy(fs.dirs[0][1].name, "..");
    fs.dirs[0][1].inode_num = 0;
    fs.dir_counts[0] = 2;
}

static int alloc_inode(void) {
    for (int i = 1; i < MAX_INODES; i++) {
        if (!fs.inode_used[i]) {
            fs.inode_used[i] = 1;
            fs.sb.free_inodes--;
            return i;
        }
    }
    return -1;
}

static int vfs_mkdir(const char *name) {
    int ino = alloc_inode();
    if (ino < 0) return -1;
    fs.inodes[ino].mode = 1;
    fs.inodes[ino].nlinks = 2;

    /* Add to root directory */
    int cnt = fs.dir_counts[0];
    if (cnt >= 64) return -1;
    strncpy(fs.dirs[0][cnt].name, name, MAX_NAME - 1);
    fs.dirs[0][cnt].inode_num = ino;
    fs.dir_counts[0]++;

    /* Add . and .. */
    strcpy(fs.dirs[ino][0].name, ".");
    fs.dirs[ino][0].inode_num = ino;
    strcpy(fs.dirs[ino][1].name, "..");
    fs.dirs[ino][1].inode_num = 0;
    fs.dir_counts[ino] = 2;

    return 0;
}
```

### Step 4: File Open, Read, Write

```c
static int vfs_create(const char *name) {
    int ino = alloc_inode();
    if (ino < 0) return -1;
    fs.inodes[ino].mode = 2;  /* file */
    fs.inodes[ino].size = 0;
    fs.inodes[ino].nlinks = 1;

    int cnt = fs.dir_counts[0];
    if (cnt >= 64) return -1;
    strncpy(fs.dirs[0][cnt].name, name, MAX_NAME - 1);
    fs.dirs[0][cnt].inode_num = ino;
    fs.dir_counts[0]++;
    return 0;
}

static int vfs_open(const char *name) {
    /* Search root directory for the name */
    for (int i = 0; i < fs.dir_counts[0]; i++) {
        if (strcmp(fs.dirs[0][i].name, name) == 0) {
            int ino = fs.dirs[0][i].inode_num;
            for (int fd = 0; fd < MAX_FD; fd++) {
                if (!fs.fd_table[fd].in_use) {
                    fs.fd_table[fd].in_use = 1;
                    fs.fd_table[fd].inode_num = ino;
                    fs.fd_table[fd].offset = 0;
                    fs.fd_table[fd].flags = 2;  /* O_RDWR */
                    return fd;
                }
            }
        }
    }
    return -1;
}

static int vfs_write(int fd, const char *buf, int len) {
    if (fd < 0 || fd >= MAX_FD || !fs.fd_table[fd].in_use) return -1;
    int ino = fs.fd_table[fd].inode_num;
    Inode *ip = &fs.inodes[ino];

    int space = MAX_DATA - ip->size;
    if (len > space) len = space;

    memcpy(ip->data + ip->size, buf, len);
    ip->size += len;
    fs.fd_table[fd].offset = ip->size;
    return len;
}

static int vfs_read(int fd, char *buf, int len) {
    if (fd < 0 || fd >= MAX_FD || !fs.fd_table[fd].in_use) return -1;
    int ino = fs.fd_table[fd].inode_num;
    Inode *ip = &fs.inodes[ino];

    int avail = ip->size - fs.fd_table[fd].offset;
    if (len > avail) len = avail;
    if (len <= 0) return 0;

    memcpy(buf, ip->data + fs.fd_table[fd].offset, len);
    fs.fd_table[fd].offset += len;
    return len;
}

static void vfs_close(int fd) {
    if (fd >= 0 && fd < MAX_FD)
        fs.fd_table[fd].in_use = 0;
}

static void vfs_ls(const char *dir) {
    int ino = 0;  /* root */
    printf("%s:\n", dir);
    for (int i = 0; i < fs.dir_counts[ino]; i++) {
        int dino = fs.dirs[ino][i].inode_num;
        const char *type = fs.inodes[dino].mode == 1 ? "DIR " : "FILE";
        printf("  %s  %-4d  %s\n", type, dino, fs.dirs[ino][i].name);
    }
}
```

### Step 5: Journaling Simulation

```c
static void journal_begin(const char *op, int ino, const char *data) {
    int slot = fs.journal_head++;
    strncpy(fs.journal[slot].op, op, 15);
    fs.journal[slot].inode_num = ino;
    if (data) strncpy(fs.journal[slot].data, data, BLOCK_SIZE - 1);
    fs.journal[slot].committed = 0;
    printf("  [journal] BEGIN: %s inode %d\n", op, ino);
}

static void journal_commit(void) {
    fs.journal[fs.journal_head - 1].committed = 1;
    printf("  [journal] COMMIT: entry %d\n", fs.journal_head - 1);
}

static void journal_replay(void) {
    printf("\n[recovery] Replaying journal (%d entries)...\n", fs.journal_head);
    for (int i = 0; i < fs.journal_head; i++) {
        if (fs.journal[i].committed) {
            printf("  Replay: %s inode %d\n",
                   fs.journal[i].op, fs.journal[i].inode_num);
        } else {
            printf("  Skip uncommitted: %s inode %d\n",
                   fs.journal[i].op, fs.journal[i].inode_num);
        }
    }
}
```

### Step 6: Main — Tying It Together

```c
int main(void) {
    printf("Mini In-Memory File System\n");
    printf("==========================\n\n");

    vfs_init();

    /* Create directories and files */
    vfs_mkdir("home");
    vfs_mkdir("tmp");
    vfs_create("readme.txt");
    vfs_create("data.bin");
    vfs_ls("/");

    /* Write to a file */
    int fd = vfs_open("readme.txt");
    if (fd >= 0) {
        const char *msg = "Hello from the VFS!";
        vfs_write(fd, msg, strlen(msg));
        vfs_close(fd);
    }

    /* Read it back */
    fd = vfs_open("readme.txt");
    if (fd >= 0) {
        char buf[256];
        int n = vfs_read(fd, buf, sizeof(buf) - 1);
        buf[n] = '\0';
        printf("\nRead from readme.txt: \"%s\"\n", buf);
        vfs_close(fd);
    }

    /* Journaling demo */
    printf("\n--- Journaling Simulation ---\n");
    journal_begin("write", 3, "new data block");
    journal_commit();
    journal_begin("update_inode", 3, "size=100");
    journal_commit();
    journal_begin("write", 4, "incomplete write");  /* not committed */
    journal_replay();

    return 0;
}
```

**Compile and run**: `gcc -O2 -o vfs main.c && ./vfs`

## Use It

- **ext4**: uses extents (contiguous block ranges) instead of per-block pointers, with ordered journaling as default.
- **xfs**: allocation groups for parallelism, uses B+trees for directory and extent indexing.
- **btrfs**: copy-on-write replaces journaling entirely (see Lesson 12).
- **All** go through the VFS layer — the same `open()`/`read()`/`write()` syscalls work regardless of underlying file system.

## Read the Source

- `fs/ext4/inode.c` — ext4 inode operations including extent tree traversal.
- `fs/ext4/ext4_jbd2.c` — journal commit path (JBD2 = Journaling Block Device v2).

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained reference snippet you can reuse in later phases.**

## Exercises

### Level 1 — Recall

An inode stores metadata but not the filename. Where does the filename-to-inode mapping live, and how does path resolution work for `/a/b/c`?

### Level 2 — Application

Extend `vfs_ls` to work on any directory (not just root). Walk the directory tree by resolving path components one at a time: split `"home/user"` → find "home" in root → find "user" in the "home" directory inode.

### Level 3 — Build

Add **indirect block pointers** to the inode. Extend the data capacity beyond 12 direct blocks by adding a single-indirect block that points to a block of 128 additional block pointers. Implement `vfs_read` and `vfs_write` that handle offsets crossing direct/indirect boundaries.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| VFS | "Virtual file system" | Kernel abstraction layer dispatching file ops to concrete implementations (ext4, xfs, etc.) |
| Inode | "File metadata" | Fixed-size structure storing size, permissions, timestamps, and block pointers — one per file |
| Directory | "List of files" | A special file mapping filename strings to inode numbers |
| Superblock | "FS metadata header" | On-disk structure with total inodes/blocks, block size, free counts, magic number |
| Journaling | "Write-ahead log for files" | Log intent to journal before applying changes; replay on crash for consistency |
| File descriptor | "fd" | Small integer indexing a per-process table of open file state (offset, inode, flags) |

## Further Reading

- W. Richard Stevens, *Advanced Programming in the UNIX Environment*, Ch. 4 (File I/O)
- ext4 wiki: https://ext4.wiki.kernel.org/
- "Journaling the Linux ext2fs File System" (Stephen Tweedie, 1998 Linux Expo)
