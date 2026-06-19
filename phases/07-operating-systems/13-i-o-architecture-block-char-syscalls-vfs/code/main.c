/*
 * Lesson 13: I/O Architecture — Block, Char, syscalls, vfs
 *
 * Simulates the I/O stack: block device, character device,
 * file descriptor table, VFS dispatch, and select() multiplexing.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <stdint.h>
#include <stdbool.h>

/* ─── Simulated Block Device ───────────────────────────────────── */

#define SECTOR_SIZE   512
#define NUM_SECTORS   64

typedef struct {
    uint8_t data[NUM_SECTORS][SECTOR_SIZE];
} block_device_t;

static int block_read(block_device_t *dev, int sector, void *buf, int count) {
    if (sector < 0 || sector >= NUM_SECTORS) return -1;
    int bytes = count < SECTOR_SIZE ? count : SECTOR_SIZE;
    memcpy(buf, dev->data[sector], bytes);
    return bytes;
}

static int block_write(block_device_t *dev, int sector, const void *buf, int count) {
    if (sector < 0 || sector >= NUM_SECTORS) return -1;
    int bytes = count < SECTOR_SIZE ? count : SECTOR_SIZE;
    memcpy(dev->data[sector], buf, bytes);
    return bytes;
}

/* ─── Simulated Character Device ───────────────────────────────── */

#define CHAR_BUF_SIZE 256

typedef struct {
    char buffer[CHAR_BUF_SIZE];
    int  read_pos;
    int  write_pos;
    int  count;
} char_device_t;

static void char_init(char_device_t *dev) {
    memset(dev, 0, sizeof(*dev));
}

static int char_read(char_device_t *dev, void *buf, int count) {
    int i = 0;
    uint8_t *out = buf;
    while (i < count && dev->count > 0) {
        out[i++] = dev->buffer[dev->read_pos];
        dev->read_pos = (dev->read_pos + 1) % CHAR_BUF_SIZE;
        dev->count--;
    }
    return i;
}

static int char_write(char_device_t *dev, const void *buf, int count) {
    int i = 0;
    const uint8_t *in = buf;
    while (i < count && dev->count < CHAR_BUF_SIZE) {
        dev->buffer[dev->write_pos] = in[i++];
        dev->write_pos = (dev->write_pos + 1) % CHAR_BUF_SIZE;
        dev->count++;
    }
    return i;
}

/* ─── VFS Operation Table ──────────────────────────────────────── */

typedef enum { DEV_BLOCK, DEV_CHAR } dev_type_t;

typedef struct {
    const char *name;
    dev_type_t  type;
    void       *device;   /* block_device_t* or char_device_t* */
    int (*read_fn)(void *dev, void *buf, int count);
    int (*write_fn)(void *dev, const void *buf, int count);
} vfs_ops_t;

/* ─── File Descriptor Table ────────────────────────────────────── */

#define MAX_FDS 16

typedef struct {
    bool       in_use;
    int        offset;
    vfs_ops_t *ops;
} fd_entry_t;

static fd_entry_t fd_table[MAX_FDS];

static int fd_open(vfs_ops_t *ops) {
    for (int i = 0; i < MAX_FDS; i++) {
        if (!fd_table[i].in_use) {
            fd_table[i].in_use  = true;
            fd_table[i].offset  = 0;
            fd_table[i].ops     = ops;
            return i;
        }
    }
    return -1;
}

static int fd_read(int fd, void *buf, int count) {
    if (fd < 0 || fd >= MAX_FDS || !fd_table[fd].in_use) return -1;
    return fd_table[fd].ops->read_fn(fd_table[fd].ops->device, buf, count);
}

static int fd_write(int fd, const void *buf, int count) {
    if (fd < 0 || fd >= MAX_FDS || !fd_table[fd].in_use) return -1;
    return fd_table[fd].ops->write_fn(fd_table[fd].ops->device, buf, count);
}

static void fd_close(int fd) {
    if (fd >= 0 && fd < MAX_FDS)
        fd_table[fd].in_use = false;
}

/* ─── select() Simulation ──────────────────────────────────────── */

typedef struct {
    int  fd;
    bool readable;
    bool writable;
} select_fd_t;

static int sim_select(select_fd_t *fds, int nfds) {
    int ready = 0;
    for (int i = 0; i < nfds; i++) {
        fds[i].readable = fds[i].writable = false;
        if (fds[i].fd < 0 || fds[i].fd >= MAX_FDS || !fd_table[fds[i].fd].in_use)
            continue;
        dev_type_t t = fd_table[fds[i].fd].ops->type;
        if (t == DEV_CHAR) {
            char_device_t *cd = fd_table[fds[i].fd].ops->device;
            if (cd->count > 0) { fds[i].readable = true; ready++; }
            if (cd->count < CHAR_BUF_SIZE) { fds[i].writable = true; ready++; }
        } else {
            fds[i].readable = fds[i].writable = true;
            ready++;
        }
    }
    return ready;
}

/* ─── Adapter wrappers ─────────────────────────────────────────── */

static int block_read_wrapper(void *dev, void *buf, int count) {
    block_device_t *b = dev;
    return block_read(b, 0, buf, count); /* always sector 0 for demo */
}

static int block_write_wrapper(void *dev, const void *buf, int count) {
    block_device_t *b = dev;
    return block_write(b, 0, buf, count);
}

static int char_read_wrapper(void *dev, void *buf, int count) {
    return char_read(dev, buf, count);
}

static int char_write_wrapper(void *dev, const void *buf, int count) {
    return char_write(dev, buf, count);
}

/* ─── Main Demo ────────────────────────────────────────────────── */

int main(void) {
    printf("=== Lesson 13: I/O Architecture Demo ===\n\n");

    /* Initialize devices */
    block_device_t blk;
    memset(&blk, 0, sizeof(blk));
    const char *blk_msg = "Hello from block device sector 0!";
    block_write(&blk, 0, blk_msg, strlen(blk_msg));

    char_device_t ch;
    char_init(&ch);
    const char *ch_msg = "Hello from char device!";
    char_write(&ch, ch_msg, strlen(ch_msg));

    /* Register with VFS */
    vfs_ops_t blk_ops = { "block0", DEV_BLOCK, &blk,
                           block_read_wrapper, block_write_wrapper };
    vfs_ops_t ch_ops  = { "tty0",   DEV_CHAR,  &ch,
                           char_read_wrapper, char_write_wrapper };

    /* Open file descriptors */
    int fd0 = fd_open(&blk_ops);
    int fd1 = fd_open(&ch_ops);
    printf("Opened: block device -> fd %d, char device -> fd %d\n\n", fd0, fd1);

    /* Read from each fd */
    char buf[128];
    int n;

    n = fd_read(fd0, buf, sizeof(buf) - 1);
    buf[n] = '\0';
    printf("read(fd=%d) -> %d bytes: \"%s\"\n", fd0, n, buf);

    n = fd_read(fd1, buf, sizeof(buf) - 1);
    buf[n] = '\0';
    printf("read(fd=%d) -> %d bytes: \"%s\"\n", fd1, n, buf);

    /* Write to block device */
    const char *write_msg = "Overwritten sector data";
    fd_write(fd0, write_msg, strlen(write_msg));
    memset(buf, 0, sizeof(buf));
    /* Re-read: need to reset sector offset for our simplified demo */
    block_read(&blk, 0, buf, sizeof(buf) - 1);
    buf[strlen(write_msg)] = '\0';
    printf("\nAfter write+re-read on fd %d: \"%s\"\n", fd0, buf);

    /* Write to char device */
    const char *ch_write = " + more data";
    fd_write(fd1, ch_write, strlen(ch_write));
    memset(buf, 0, sizeof(buf));
    /* drain remaining */
    n = char_read(&ch, buf, sizeof(buf) - 1);
    buf[n] = '\0';
    printf("After write on fd %d, remaining: \"%s\"\n", fd1, buf);

    /* select() simulation */
    printf("\n--- select() simulation ---\n");
    select_fd_t fds[2] = { {fd0, false, false}, {fd1, false, false} };
    int ready = sim_select(fds, 2);
    printf("select() -> %d fds ready\n", ready);
    for (int i = 0; i < 2; i++) {
        printf("  fd %d: readable=%s writable=%s (%s)\n",
               fds[i].fd,
               fds[i].readable ? "yes" : "no",
               fds[i].writable ? "yes" : "no",
               fd_table[fds[i].fd].ops->name);
    }

    /* Close fds */
    fd_close(fd0);
    fd_close(fd1);
    printf("\nClosed all file descriptors.\n");

    return 0;
}
