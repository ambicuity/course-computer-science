# Lesson 18: Devices and Drivers — Char, Block, Net

## Why This Matters

Every piece of hardware — keyboard, disk, network card, GPU — is accessed through a **device driver**, a kernel module that translates generic operations (open, read, write, ioctl) into device-specific register writes and DMA transfers. Without drivers, the OS is just an expensive calculator. Understanding the driver model is essential for systems programming, embedded development, and kernel hacking.

## What Is a Device Driver?

A device driver is kernel code that implements a standard interface for a specific hardware device. The kernel presents every device as a **file** (in `/dev/`), and userspace programs access hardware through the same `open()`, `read()`, `write()`, `ioctl()` system calls they use for regular files.

```
  Userspace             Kernel                    Hardware
 ┌──────────┐    ┌──────────────────────┐    ┌──────────────┐
 │ open()   │───►│  VFS dispatches to   │───►│  Device      │
 │ read()   │    │  driver via          │    │  Registers   │
 │ write()  │    │  file_operations     │    │  / DMA       │
 │ ioctl()  │    │  table               │    │  / Port I/O  │
 └──────────┘    └──────────────────────┘    └──────────────┘
```

## Device Categories

### Character Drivers (Byte Stream)

Character devices transfer data as a **stream of bytes**, like a serial port or keyboard. No buffering by the kernel — each `read()`/`write()` goes directly to the device.

- **Examples:** `/dev/ttyS0` (serial), `/dev/input/mice` (mouse), `/dev/random`
- **Access pattern:** Sequential byte-by-byte
- **No seek:** `lseek()` doesn't apply

### Block Drivers (Sector-Based)

Block devices transfer data in **fixed-size blocks** (typically 512 bytes or 4 KB). The kernel maintains a **page cache** (buffer cache) between the driver and userspace, so `read()`/`write()` go through cached copies.

- **Examples:** `/dev/sda` (disk), `/dev/nvme0n1` (NVMe SSD)
- **Access pattern:** Random access by block number
- **Seek:** `lseek()` works — you can read block 100 before block 1

### Network Drivers (Packet-Based)

Network devices don't have a `/dev/` file. Instead, they register a **network interface** (`eth0`, `wlan0`) and use a different API: sk_buff (socket buffers) instead of file operations.

- **Examples:** `eth0` (Ethernet), `wlan0` (WiFi), `lo` (loopback)
- **Access pattern:** Packet-based send/receive

## The file_operations Structure

The heart of a Linux character/block driver is the `file_operations` struct:

```c
struct file_operations {
    struct module *owner;
    loff_t (*llseek)(struct file *, loff_t, int);
    ssize_t (*read)(struct file *, char __user *, size_t, loff_t *);
    ssize_t (*write)(struct file *, const char __user *, size_t, loff_t *);
    int (*open)(struct inode *, struct file *);
    int (*release)(struct inode *, struct file *);
    long (*unlocked_ioctl)(struct file *, unsigned int, unsigned long);
    /* ... */
};
```

When userspace calls `read(fd, buf, len)`, the kernel looks up `fd` → finds the `file` struct → finds its `f_op` pointer → calls `f_op->read(fd, buf, len)`.

## Interrupt Handling — Top Half vs Bottom Half

When hardware raises an interrupt, the kernel needs to respond quickly but can't do everything in the interrupt handler (other interrupts are disabled during it).

**Top half** (interrupt handler):
- Runs in interrupt context (hardirq)
- Must be fast — acknowledge the interrupt, read data from hardware, schedule bottom half
- Cannot sleep, cannot use mutexes

**Bottom half** (deferred work):
- Runs in process or softirq context
- Can do heavy processing, allocate memory, acquire locks
- Three mechanisms:

| Mechanism | Context | Use Case |
|-----------|---------|----------|
| Softirq | Softirq (BH disabled) | Network stack, block layer — performance-critical |
| Tasklet | Softirq (serialized per-tasklet) | Simple deferred work, old-style |
| Workqueue | Process context | Can sleep, use mutexes — most flexible |

```
  Hardware interrupt
        │
        ▼
  ┌─────────────┐
  │  Top Half    │ ← fast: read data, ack interrupt
  │  (hardirq)   │
  └──────┬───────┘
         │ schedule
         ▼
  ┌─────────────┐
  │  Bottom Half │ ← heavy: process data, notify userspace
  │  (softirq /  │
  │   workqueue) │
  └─────────────┘
```

## DMA (Direct Memory Access)

For high-speed data transfer, the driver tells the device to read/write memory directly, bypassing the CPU:

1. Driver allocates a DMA buffer in physical memory
2. Driver programs the device with the buffer's physical address
3. Device transfers data directly to/from the buffer
4. Device raises an interrupt when done
5. Driver processes the data

This is essential for disk and network drivers — without DMA, the CPU would be bottlenecked by copying every byte.

## Device Discovery

How does the kernel find hardware?

- **PCI/PCIe:** The kernel scans the PCI bus, reads device/vendor IDs from config space, and matches them against driver `pci_device_id` tables
- **Device Tree (DT):** ARM and embedded systems describe hardware in a tree structure (`.dtb` file) — the kernel walks it to find devices and their properties
- **ACPI:** x86 systems use ACPI tables for device enumeration and power management
- **USB:** Devices announce themselves on connection; the kernel matches against driver `usb_device_id` tables
- **Plug and Play:** Drivers register to handle specific hardware IDs; the kernel binds them on discovery

## Build It

See `code/main.c` for a simulated driver framework:

- `DeviceDriver` struct with function pointers (open, read, write, ioctl, close)
- `CharDriver` — simulated serial port: writes go to a circular buffer, reads drain it
- `BlockDriver` — simulated disk: sector-based read/write with a backing array
- `NetDriver` — simulated NIC: send/receive packets as byte arrays
- Interrupt handler simulation: top half triggers bottom half via a callback
- `register_driver()` / `unregister_driver()` — simulated driver registration

## Use It

Real-world driver usage:

- **Every hardware interaction** goes through a driver — `read()` on a terminal, `write()` to a disk, `sendto()` on a socket
- **`/dev/null`, `/dev/zero`, `/dev/random`** — character drivers that generate or discard data
- **NVMe drivers** — block drivers that queue I/O requests with multi-queue (blk-mq) for SSDs
- **eBPF** — modern Linux attaches BPF programs to tracepoints in drivers for observability

## Ship It

The simulated driver framework in `code/main.c` demonstrates the driver interface pattern: a struct of function pointers that the kernel dispatches through. This pattern appears in every driver, every OS.

## Exercises

### Level 1 — Recall

What is the difference between a character driver and a block driver? What does the `file_operations` struct do? Why can't you sleep in a top-half interrupt handler?

### Level 2 — Application

Extend the `CharDriver` in `code/main.c` to support `ioctl` commands: one to set the baud rate (store it in the driver struct), one to query the number of bytes in the buffer. Write a test program that opens the simulated device, sends data, queries the buffer size, and reads the data back.

### Level 3 — Build

Implement a complete block device driver simulator that maintains an array of 1 KB sectors. Support:
- `read_sector(n, buf)` — read sector n into buf
- `write_sector(n, buf)` — write buf to sector n
- A simple bitmap allocator that tracks which sectors are in use
- A `format()` operation that initializes the superblock and free list

Write a test that creates a simulated filesystem: format the device, write an "inode table" to sectors 1-10, allocate and write file data to subsequent sectors, then read it all back and verify.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Character device | "Byte stream device" | Device accessed as a sequential stream of bytes |
| Block device | "Disk-like device" | Device accessed in fixed-size blocks, random access |
| file_operations | "The driver's API" | Struct of function pointers that implements open/read/write/ioctl for a device |
| Top half | "Interrupt handler" | Fast interrupt-context code that acknowledges hardware and schedules deferred work |
| Bottom half | "Deferred work" | Post-interrupt processing in softirq or workqueue context |
| DMA | "Direct memory access" | Hardware transfers data directly to/from RAM without CPU involvement |

## Further Reading

- Corbet, Kroah-Hartman, *Linux Device Drivers, 3rd Edition* (free at ldd3.net)
- `man 4 tty`, `man 4 sd`, `man 4 netdevice`
- Linux kernel source: `drivers/char/`, `drivers/block/`, `drivers/net/`
- `Documentation/driver-api/` in the kernel source tree
