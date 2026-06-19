#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>

/*
 * Simulated device driver framework: char, block, net.
 * Demonstrates the file_operations pattern used in real OS drivers.
 * Compile: gcc -o driver_demo main.c
 * Run:     ./driver_demo
 */

/* ===================================================================
 *  Device Driver Interface (simulates kernel file_operations)
 * =================================================================== */

#define DRIVER_NAME_MAX 64
#define BUF_SIZE        4096
#define BLOCK_SIZE      512
#define NUM_BLOCKS      64
#define MAX_PACKETS     32
#define PACKET_SIZE     1500

/* Forward declarations */
struct DeviceDriver;
struct Device;

/* Device context — holds driver-specific state */
struct Device {
    char name[DRIVER_NAME_MAX];
    struct DeviceDriver *driver;
    int is_open;
    void *private_data;
};

/* Function pointer table — simulates file_operations */
struct DeviceDriver {
    char name[DRIVER_NAME_MAX];
    int  (*open)(struct Device *dev);
    int  (*close)(struct Device *dev);
    ssize_t (*read)(struct Device *dev, char *buf, size_t count);
    ssize_t (*write)(struct Device *dev, const char *buf, size_t count);
    int  (*ioctl)(struct Device *dev, unsigned int cmd, unsigned long arg);
};

/* Driver registry */
#define MAX_DRIVERS 16
static struct DeviceDriver *driver_registry[MAX_DRIVERS];
static int driver_count = 0;

int register_driver(struct DeviceDriver *drv) {
    if (driver_count >= MAX_DRIVERS) {
        fprintf(stderr, "register_driver: registry full\n");
        return -1;
    }
    driver_registry[driver_count++] = drv;
    printf("  [kernel] registered driver: %s\n", drv->name);
    return 0;
}

/* Generic device operations — dispatch through function pointers */
static ssize_t device_read(struct Device *dev, char *buf, size_t count) {
    if (!dev->is_open || !dev->driver->read) return -EBADF;
    return dev->driver->read(dev, buf, count);
}

static ssize_t device_write(struct Device *dev, const char *buf, size_t count) {
    if (!dev->is_open || !dev->driver->write) return -EBADF;
    return dev->driver->write(dev, buf, count);
}

static int device_ioctl(struct Device *dev, unsigned int cmd, unsigned long arg) {
    if (!dev->is_open || !dev->driver->ioctl) return -ENOTTY;
    return dev->driver->ioctl(dev, cmd, arg);
}

/* ===================================================================
 *  Character Driver — Simulated Serial Port
 * =================================================================== */

/* Circular buffer for the serial device */
struct CharDevData {
    char buffer[BUF_SIZE];
    int  head;  /* read position */
    int  tail;  /* write position */
    int  count; /* bytes in buffer */
    int  baud_rate;
};

static int char_open(struct Device *dev) {
    struct CharDevData *data = malloc(sizeof(*data));
    if (!data) return -ENOMEM;
    memset(data, 0, sizeof(*data));
    data->baud_rate = 9600;
    dev->private_data = data;
    printf("  [char] %s opened (baud=%d)\n", dev->name, data->baud_rate);
    return 0;
}

static int char_close(struct Device *dev) {
    free(dev->private_data);
    dev->private_data = NULL;
    printf("  [char] %s closed\n", dev->name);
    return 0;
}

static ssize_t char_read(struct Device *dev, char *buf, size_t count) {
    struct CharDevData *data = dev->private_data;
    size_t n = 0;
    while (n < count && data->count > 0) {
        buf[n++] = data->buffer[data->head];
        data->head = (data->head + 1) % BUF_SIZE;
        data->count--;
    }
    return (ssize_t)n;
}

static ssize_t char_write(struct Device *dev, const char *buf, size_t count) {
    struct CharDevData *data = dev->private_data;
    size_t n = 0;
    while (n < count && data->count < BUF_SIZE) {
        data->buffer[data->tail] = buf[n++];
        data->tail = (data->tail + 1) % BUF_SIZE;
        data->count++;
    }
    return (ssize_t)n;
}

static int char_ioctl(struct Device *dev, unsigned int cmd, unsigned long arg) {
    struct CharDevData *data = dev->private_data;
    switch (cmd) {
    case 0x1001:  /* set baud rate */
        data->baud_rate = (int)arg;
        printf("  [char] %s baud rate set to %d\n", dev->name, data->baud_rate);
        return 0;
    case 0x1002:  /* query buffer count */
        return data->count;
    default:
        return -ENOTTY;
    }
}

static struct DeviceDriver char_driver = {
    .name  = "sim_serial",
    .open  = char_open,
    .close = char_close,
    .read  = char_read,
    .write = char_write,
    .ioctl = char_ioctl,
};

/* ===================================================================
 *  Block Driver — Simulated Disk
 * =================================================================== */

struct BlockDevData {
    char sectors[NUM_BLOCKS][BLOCK_SIZE];
    int  sector_used[NUM_BLOCKS];
};

static int block_open(struct Device *dev) {
    struct BlockDevData *data = malloc(sizeof(*data));
    if (!data) return -ENOMEM;
    memset(data, 0, sizeof(*data));
    dev->private_data = data;
    printf("  [block] %s opened (%d sectors of %d bytes)\n",
           dev->name, NUM_BLOCKS, BLOCK_SIZE);
    return 0;
}

static int block_close(struct Device *dev) {
    free(dev->private_data);
    dev->private_data = NULL;
    printf("  [block] %s closed\n", dev->name);
    return 0;
}

static ssize_t block_read(struct Device *dev, char *buf, size_t count) {
    /* count = sector number to read (passed as size_t for demo) */
    struct BlockDevData *data = dev->private_data;
    int sector = (int)count;
    if (sector < 0 || sector >= NUM_BLOCKS) return -EINVAL;
    memcpy(buf, data->sectors[sector], BLOCK_SIZE);
    return BLOCK_SIZE;
}

static ssize_t block_write(struct Device *dev, const char *buf, size_t count) {
    /* count = sector number to write (passed as size_t for demo) */
    struct BlockDevData *data = dev->private_data;
    int sector = (int)count;
    if (sector < 0 || sector >= NUM_BLOCKS) return -EINVAL;
    memcpy(data->sectors[sector], buf, BLOCK_SIZE);
    data->sector_used[sector] = 1;
    return BLOCK_SIZE;
}

static int block_ioctl(struct Device *dev, unsigned int cmd, unsigned long arg) {
    struct BlockDevData *data = dev->private_data;
    (void)arg;
    switch (cmd) {
    case 0x2001: { /* count used sectors */
        int used = 0;
        for (int i = 0; i < NUM_BLOCKS; i++)
            if (data->sector_used[i]) used++;
        return used;
    }
    default:
        return -ENOTTY;
    }
}

static struct DeviceDriver block_driver = {
    .name  = "sim_disk",
    .open  = block_open,
    .close = block_close,
    .read  = block_read,
    .write = block_write,
    .ioctl = block_ioctl,
};

/* ===================================================================
 *  Network Driver — Simulated NIC
 * =================================================================== */

struct NetPacket {
    char data[PACKET_SIZE];
    int  length;
};

struct NetDevData {
    struct NetPacket tx_queue[MAX_PACKETS];
    struct NetPacket rx_queue[MAX_PACKETS];
    int tx_count;
    int rx_count;
    int tx_head;
    int rx_head;
    unsigned long packets_sent;
    unsigned long packets_recv;
};

static int net_open(struct Device *dev) {
    struct NetDevData *data = malloc(sizeof(*data));
    if (!data) return -ENOMEM;
    memset(data, 0, sizeof(*data));
    dev->private_data = data;
    printf("  [net] %s opened (packet size=%d)\n", dev->name, PACKET_SIZE);
    return 0;
}

static int net_close(struct Device *dev) {
    free(dev->private_data);
    dev->private_data = NULL;
    printf("  [net] %s closed\n", dev->name);
    return 0;
}

static ssize_t net_read(struct Device *dev, char *buf, size_t count) {
    struct NetDevData *data = dev->private_data;
    if (data->rx_count == 0) return 0;  /* no packets */
    struct NetPacket *pkt = &data->rx_queue[data->rx_head];
    size_t n = (count < (size_t)pkt->length) ? count : (size_t)pkt->length;
    memcpy(buf, pkt->data, n);
    data->rx_head = (data->rx_head + 1) % MAX_PACKETS;
    data->rx_count--;
    data->packets_recv++;
    return (ssize_t)n;
}

static ssize_t net_write(struct Device *dev, const char *buf, size_t count) {
    struct NetDevData *data = dev->private_data;
    if (data->tx_count >= MAX_PACKETS) return -ENOSPC;
    int slot = (data->tx_head + data->tx_count) % MAX_PACKETS;
    size_t n = (count > PACKET_SIZE) ? PACKET_SIZE : count;
    memcpy(data->tx_queue[slot].data, buf, n);
    data->tx_queue[slot].length = (int)n;
    data->tx_count++;
    data->packets_sent++;
    return (ssize_t)n;
}

static int net_ioctl(struct Device *dev, unsigned int cmd, unsigned long arg) {
    struct NetDevData *data = dev->private_data;
    (void)arg;
    switch (cmd) {
    case 0x3001:  /* get packets sent */
        return (int)data->packets_sent;
    case 0x3002:  /* get packets received */
        return (int)data->packets_recv;
    case 0x3003:  /* get rx queue depth */
        return data->rx_count;
    default:
        return -ENOTTY;
    }
}

static struct DeviceDriver net_driver = {
    .name  = "sim_nic",
    .open  = net_open,
    .close = net_close,
    .read  = net_read,
    .write = net_write,
    .ioctl = net_ioctl,
};

/* ===================================================================
 *  Interrupt Simulation (Top Half → Bottom Half)
 * =================================================================== */

typedef void (*bottom_half_fn)(void *data);

struct InterruptHandler {
    bottom_half_fn fn;
    void *data;
    int  pending;
};

#define MAX_IRQS 4
static struct InterruptHandler irq_table[MAX_IRQS];

/* Top half: simulate hardware interrupt by setting pending flag */
static void irq_raise(int irq_num) {
    if (irq_num < 0 || irq_num >= MAX_IRQS) return;
    printf("  [irq] top half: IRQ %d raised\n", irq_num);
    irq_table[irq_num].pending = 1;
}

/* Bottom half: process pending deferred work */
static void irq_process(void) {
    for (int i = 0; i < MAX_IRQS; i++) {
        if (irq_table[i].pending) {
            printf("  [irq] bottom half: processing IRQ %d\n", i);
            if (irq_table[i].fn) {
                irq_table[i].fn(irq_table[i].data);
            }
            irq_table[i].pending = 0;
        }
    }
}

static void irq_register(int irq_num, bottom_half_fn fn, void *data) {
    if (irq_num < 0 || irq_num >= MAX_IRQS) return;
    irq_table[irq_num].fn = fn;
    irq_table[irq_num].data = data;
    irq_table[irq_num].pending = 0;
}

static void sample_bottom_half(void *data) {
    (void)data;
    printf("    -> bottom half callback: processing data...\n");
}

/* ===================================================================
 *  Test Harness
 * =================================================================== */

static void test_char_driver(void) {
    printf("--- Character Driver Test ---\n");
    struct Device dev = { .name = "ttyS0", .driver = &char_driver, .is_open = 0 };

    dev.driver->open(&dev);
    dev.is_open = 1;

    const char *msg = "Hello, serial!";
    device_write(&dev, msg, strlen(msg));
    printf("  Wrote %zu bytes to serial\n", strlen(msg));

    device_ioctl(&dev, 0x1001, 115200);  /* set baud rate */
    int count = device_ioctl(&dev, 0x1002, 0);
    printf("  Buffer has %d bytes\n", count);

    char buf[256];
    ssize_t n = device_read(&dev, buf, sizeof(buf));
    buf[n] = '\0';
    printf("  Read %zd bytes: '%s'\n", n, buf);

    dev.driver->close(&dev);
    dev.is_open = 0;
    printf("\n");
}

static void test_block_driver(void) {
    printf("--- Block Driver Test ---\n");
    struct Device dev = { .name = "sda", .driver = &block_driver, .is_open = 0 };

    dev.driver->open(&dev);
    dev.is_open = 1;

    /* Write data to sector 0 */
    char sector_buf[BLOCK_SIZE];
    memset(sector_buf, 0, BLOCK_SIZE);
    strcpy(sector_buf, "This is sector 0 data.");
    device_write(&dev, sector_buf, 0);  /* arg 0 = sector number */

    /* Write data to sector 5 */
    strcpy(sector_buf, "This is sector 5 data.");
    device_write(&dev, sector_buf, 5);

    int used = device_ioctl(&dev, 0x2001, 0);
    printf("  Used sectors: %d\n", used);

    /* Read back sector 0 */
    char read_buf[BLOCK_SIZE];
    device_read(&dev, read_buf, 0);
    printf("  Sector 0: '%s'\n", read_buf);

    dev.driver->close(&dev);
    dev.is_open = 0;
    printf("\n");
}

static void test_net_driver(void) {
    printf("--- Network Driver Test ---\n");
    struct Device dev = { .name = "eth0", .driver = &net_driver, .is_open = 0 };

    dev.driver->open(&dev);
    dev.is_open = 1;

    /* Send packets */
    for (int i = 0; i < 5; i++) {
        char pkt[64];
        snprintf(pkt, sizeof(pkt), "packet-%d", i);
        device_write(&dev, pkt, strlen(pkt));
    }
    printf("  Sent 5 packets\n");

    int sent = device_ioctl(&dev, 0x3001, 0);
    printf("  ioctl: packets sent = %d\n", sent);

    dev.driver->close(&dev);
    dev.is_open = 0;
    printf("\n");
}

static void test_interrupt_sim(void) {
    printf("--- Interrupt Simulation (Top Half → Bottom Half) ---\n");

    irq_register(0, sample_bottom_half, NULL);

    irq_raise(0);     /* simulate hardware interrupt */
    irq_process();    /* process deferred work */

    printf("\n");
}

int main(void) {
    printf("Devices and Drivers: Char, Block, Net\n");
    printf("=======================================\n\n");

    /* Register all drivers */
    printf("--- Registering Drivers ---\n");
    register_driver(&char_driver);
    register_driver(&block_driver);
    register_driver(&net_driver);
    printf("\n");

    test_char_driver();
    test_block_driver();
    test_net_driver();
    test_interrupt_sim();

    printf("Done.\n");
    return 0;
}
