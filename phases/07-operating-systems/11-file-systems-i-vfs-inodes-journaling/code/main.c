/*
 * File Systems I — VFS, inodes, journaling
 * Phase 07 — Operating Systems
 *
 * In-memory file system with inodes, directories,
 * file descriptors, and journaling simulation.
 * Compile: gcc -O2 -o vfs main.c
 * Run:     ./vfs
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAX_INODES     256
#define BLOCK_SIZE     512
#define MAX_NAME       64
#define MAX_FD         32
#define MAX_DATA       (12 * BLOCK_SIZE)  /* direct blocks only */
#define MAX_DIR_ENTRIES 64
#define JOURNAL_SIZE   256

/* ---------- Data Structures ---------- */

typedef struct {
    int    mode;    /* 0 = free, 1 = dir, 2 = file */
    int    size;
    int    nlinks;
    char   data[MAX_DATA];
} Inode;

typedef struct {
    char   name[MAX_NAME];
    int    inode_num;
} DirEntry;

typedef struct {
    int    total_inodes;
    int    free_inodes;
    int    block_size;
} Superblock;

typedef struct {
    Superblock sb;
    Inode      inodes[MAX_INODES];
    int        inode_used[MAX_INODES];
    DirEntry   dirs[MAX_INODES][MAX_DIR_ENTRIES];
    int        dir_counts[MAX_INODES];
    struct {
        int in_use;
        int inode_num;
        int offset;
        int flags;
    } fd_table[MAX_FD];
    struct {
        char op[16];
        int  inode_num;
        char data[BLOCK_SIZE];
        int  committed;
    } journal[JOURNAL_SIZE];
    int journal_head;
} FileSystem;

static FileSystem fs;

/* ---------- Initialization ---------- */

static void vfs_init(void) {
    memset(&fs, 0, sizeof(fs));
    fs.sb.total_inodes = MAX_INODES;
    fs.sb.free_inodes  = MAX_INODES - 1;
    fs.sb.block_size   = BLOCK_SIZE;

    /* Root directory at inode 0 */
    fs.inodes[0].mode = 1;
    fs.inode_used[0] = 1;
    fs.dirs[0][0].inode_num = 0;
    strcpy(fs.dirs[0][0].name, ".");
    fs.dirs[0][1].inode_num = 0;
    strcpy(fs.dirs[0][1].name, "..");
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

/* ---------- Directory Operations ---------- */

static int vfs_mkdir(const char *name) {
    int ino = alloc_inode();
    if (ino < 0) return -1;
    fs.inodes[ino].mode = 1;
    fs.inodes[ino].nlinks = 2;

    int cnt = fs.dir_counts[0];
    if (cnt >= MAX_DIR_ENTRIES) return -1;
    strncpy(fs.dirs[0][cnt].name, name, MAX_NAME - 1);
    fs.dirs[0][cnt].inode_num = ino;
    fs.dir_counts[0]++;

    /* . and .. */
    strcpy(fs.dirs[ino][0].name, ".");
    fs.dirs[ino][0].inode_num = ino;
    strcpy(fs.dirs[ino][1].name, "..");
    fs.dirs[ino][1].inode_num = 0;
    fs.dir_counts[ino] = 2;

    return 0;
}

static int vfs_create(const char *name) {
    int ino = alloc_inode();
    if (ino < 0) return -1;
    fs.inodes[ino].mode = 2;
    fs.inodes[ino].size = 0;
    fs.inodes[ino].nlinks = 1;

    int cnt = fs.dir_counts[0];
    if (cnt >= MAX_DIR_ENTRIES) return -1;
    strncpy(fs.dirs[0][cnt].name, name, MAX_NAME - 1);
    fs.dirs[0][cnt].inode_num = ino;
    fs.dir_counts[0]++;
    return 0;
}

static void vfs_ls(const char *dir) {
    int ino = 0;
    printf("%s:\n", dir);
    for (int i = 0; i < fs.dir_counts[ino]; i++) {
        int dino = fs.dirs[ino][i].inode_num;
        const char *type = fs.inodes[dino].mode == 1 ? "DIR " : "FILE";
        printf("  %s  %-4d  %s\n", type, dino, fs.dirs[ino][i].name);
    }
}

/* ---------- File Operations ---------- */

static int vfs_open(const char *name) {
    for (int i = 0; i < fs.dir_counts[0]; i++) {
        if (strcmp(fs.dirs[0][i].name, name) == 0) {
            int ino = fs.dirs[0][i].inode_num;
            if (fs.inodes[ino].mode != 2) return -1;  /* not a file */
            for (int fd = 0; fd < MAX_FD; fd++) {
                if (!fs.fd_table[fd].in_use) {
                    fs.fd_table[fd].in_use = 1;
                    fs.fd_table[fd].inode_num = ino;
                    fs.fd_table[fd].offset = 0;
                    fs.fd_table[fd].flags = 2;
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
    if (len <= 0) return 0;

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

/* ---------- Journaling Simulation ---------- */

static void journal_begin(const char *op, int ino, const char *data) {
    if (fs.journal_head >= JOURNAL_SIZE) return;
    int slot = fs.journal_head++;
    strncpy(fs.journal[slot].op, op, 15);
    fs.journal[slot].inode_num = ino;
    if (data)
        strncpy(fs.journal[slot].data, data, BLOCK_SIZE - 1);
    else
        fs.journal[slot].data[0] = '\0';
    fs.journal[slot].committed = 0;
    printf("  [journal] BEGIN: %s inode %d\n", op, ino);
}

static void journal_commit(void) {
    if (fs.journal_head <= 0) return;
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

/* ---------- Main ---------- */

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
        int written = vfs_write(fd, msg, strlen(msg));
        printf("\nWrote %d bytes to readme.txt\n", written);
        vfs_close(fd);
    }

    /* Read it back */
    fd = vfs_open("readme.txt");
    if (fd >= 0) {
        char buf[256];
        int n = vfs_read(fd, buf, sizeof(buf) - 1);
        buf[n] = '\0';
        printf("Read from readme.txt: \"%s\" (%d bytes)\n", buf, n);
        vfs_close(fd);
    }

    /* Append more data */
    fd = vfs_open("readme.txt");
    if (fd >= 0) {
        vfs_write(fd, " Appended.", 10);
        vfs_close(fd);
        fd = vfs_open("readme.txt");
        if (fd >= 0) {
            char buf[256];
            int n = vfs_read(fd, buf, sizeof(buf) - 1);
            buf[n] = '\0';
            printf("After append: \"%s\" (%d bytes)\n", buf, n);
            vfs_close(fd);
        }
    }

    /* Journaling demo: simulate a crash scenario */
    printf("\n--- Journaling Simulation ---\n");
    journal_begin("write", 3, "new data block");
    journal_commit();
    journal_begin("update_inode", 3, "size=100");
    journal_commit();
    journal_begin("write", 4, "incomplete write");
    /* crash before commit */
    journal_replay();

    /* Print superblock */
    printf("\n--- Superblock ---\n");
    printf("Total inodes: %d\n", fs.sb.total_inodes);
    printf("Free inodes:  %d\n", fs.sb.free_inodes);
    printf("Block size:   %d\n", fs.sb.block_size);
    printf("Journal entries: %d (committed: %d)\n",
           fs.journal_head,
           fs.journal[0].committed + fs.journal[1].committed +
           fs.journal[2].committed);

    return 0;
}
