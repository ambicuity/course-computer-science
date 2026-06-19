# Lesson 02: The Boot Process — BIOS, UEFI, GRUB

## Overview

When you press the power button, the CPU has no operating system, no file system, no concept of files at all. It has firmware burned into a chip and a hardwired instruction to start executing from a fixed address. Everything that happens between "power on" and "login prompt" is the boot process. This lesson traces that path.

---

## The Big Picture

```
Power On
  │
  ▼
┌─────────────────┐
│  Firmware       │  BIOS or UEFI
│  (POST, init)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Bootloader     │  GRUB, systemd-boot, Windows Boot Manager
│  (menu, config) │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Kernel         │  Loads into memory, initializes subsystems
│  (linux, vmlinuz│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Init System    │  systemd, SysVinit, openrc
│  (PID 1)        │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Login Shell    │  User space ready
└─────────────────┘
```

---

## BIOS Boot (Legacy)

BIOS (Basic Input/Output System) is the older firmware standard, dating back to the IBM PC in 1981.

### Step-by-step:

1. **POST (Power-On Self-Test)** — Firmware tests CPU, RAM, and basic hardware. You hear beeps; you see text on screen.
2. **Boot device selection** — BIOS reads the boot order from CMOS settings (hard drive, USB, network).
3. **MBR loading** — BIOS reads the first 512 bytes of the selected disk: the Master Boot Record.

```
MBR Layout (512 bytes)
┌─────────────────────────────────────┐
│  Boot code         (440 bytes)      │  ← Executable code
├─────────────────────────────────────┤
│  Disk signature     (4 bytes)       │
├─────────────────────────────────────┤
│  Null               (2 bytes)       │
├─────────────────────────────────────┤
│  Partition table    (64 bytes)      │  ← 4 entries × 16 bytes
│  (4 partitions max)                 │
├─────────────────────────────────────┤
│  Magic: 0x55AA      (2 bytes)       │  ← Valid MBR marker
└─────────────────────────────────────┘
```

4. **Boot sector execution** — BIOS jumps to the boot code in the MBR. This 440-byte stub's only job is to find and load the next stage.
5. **Bootloader** — GRUB stage 1 (in MBR) loads stage 1.5 and stage 2 from disk.
6. **Kernel loading** — The bootloader loads the kernel image into memory and jumps to it.

### Inspecting the MBR

You can examine the raw MBR of a disk. The following command reads the first 512 bytes and displays them in hex:

```bash
# Read MBR and display hex dump (CAUTION: read-only, but be careful with dd)
sudo dd if=/dev/sda bs=512 count=1 2>/dev/null | xxd | head -20
```

Expected output structure:

```
00000000: eb63 9000 0000 0000 0000 0000 0000 0000  .c..............
00000010: 0000 0000 0000 0000 0000 0000 0000 0000  ................
...
000001b0: 0000 0000 0000 0000 a1b2 c3d4 0000 8020  ...............  ← partition table starts
...
000001f0: 0000 0000 0000 0000 0000 0000 0000 55aa  ..............U. ← magic number
```

### Limitations of BIOS + MBR

- Maximum disk size: 2 TB (uses 32-bit sector addressing with 512-byte sectors)
- Maximum 4 primary partitions
- No networking, no mouse support, no graphical interface
- Runs in 16-bit real mode

---

## UEFI Boot (Modern)

UEFI (Unified Extensible Firmware Interface) replaces BIOS with a far more capable firmware.

### Key differences from BIOS:

| Feature | BIOS | UEFI |
|---|---|---|
| Mode | 16-bit real mode | 32/64-bit protected mode |
| Disk limit | 2 TB (MBR) | 9.4 ZB (GPT) |
| Partitions | 4 primary | 128 (GPT) |
| Boot code | 440 bytes in MBR | EFI System Partition (FAT32) |
| Interface | Text only | Graphical, mouse support |
| Security | None | Secure Boot (signature verification) |

### Step-by-step UEFI boot:

1. **Firmware init** — UEFI firmware runs POST and initializes hardware.
2. **EFI System Partition (ESP)** — Firmware mounts a special FAT32 partition (usually `/dev/sda1`, mounted at `/boot/efi`). This partition holds boot manager and bootloader EFI binaries.
3. **Boot manager** — UEFI's built-in boot manager reads NVRAM variables to determine boot order.
4. **Bootloader** — Loads an EFI binary (e.g., `grubx64.efi`, `shimx64.efi`).
5. **Kernel loading** — The bootloader loads the kernel and initramfs, then hands control to the kernel.

### Inspecting UEFI variables

On a UEFI system, you can inspect firmware variables:

```bash
# List UEFI variables (requires efivarfs mounted)
ls /sys/firmware/efi/efivars/ | head -20

# Show boot order
cat /sys/firmware/efi/efivars/BootOrder-* 2>/dev/null | hexdump -C

# Show firmware version
cat /sys/firmware/efi/efivars/FwVer-* 2>/dev/null | hexdump -C
```

### Checking if your system uses UEFI

```bash
# If this directory exists and has files, you're booted in UEFI mode
[ -d /sys/firmware/efi ] && echo "UEFI" || echo "Legacy BIOS"

# Check partition table type
sudo fdisk -l /dev/sda | grep "Disklabel type"
# Output: Disklabel type: gpt  (UEFI) or Disklabel type: dos  (MBR/BIOS)
```

### GPT Partition Layout

```
┌──────────────────────────────────────────────────┐
│  Protective MBR (sector 0)                       │
├──────────────────────────────────────────────────┤
│  GPT Header (sector 1)                           │
│  - disk GUID, partition table location, CRC      │
├──────────────────────────────────────────────────┤
│  Partition entries (sectors 2-33)                │
│  - up to 128 entries × 128 bytes each            │
├──────────────────────────────────────────────────┤
│  Partition 1: EFI System Partition (FAT32)       │
├──────────────────────────────────────────────────┤
│  Partition 2: /boot                              │
├──────────────────────────────────────────────────┤
│  Partition 3: / (root)                           │
├──────────────────────────────────────────────────┤
│  ...                                             │
├──────────────────────────────────────────────────┤
│  Backup partition table (last sectors)           │
└──────────────────────────────────────────────────┘
```

---

## GRUB: The Grand Unified Bootloader

GRUB is the most common bootloader on Linux systems.

### GRUB Configuration

The main configuration file is `/boot/grub/grub.cfg` (GRUB 2) or `/boot/grub2/grub.cfg`:

```bash
# Display GRUB configuration
cat /boot/grub/grub.cfg 2>/dev/null | head -50

# Or on RHEL/Fedora:
cat /boot/grub2/grub.cfg 2>/dev/null | head -50
```

A typical menu entry looks like this:

```
menuentry 'Ubuntu' --class ubuntu {
    set root='hd0,gpt2'
    linux   /vmlinuz-5.15.0-generic root=/dev/sda3 ro quiet splash
    initrd  /initrd.img-5.15.0-generic
}
```

### GRUB Stages

```
┌───────────────┐
│  Stage 1      │  Lives in MBR (BIOS) or EFI binary (UEFI)
│  (446 bytes   │  Loads Stage 1.5
│   or .efi)    │
└───────┬───────┘
        │
        ▼
┌───────────────┐
│  Stage 1.5    │  Lives in the gap between MBR and first partition
│  (filesystem  │  Contains drivers for ext2/3/4, XFS, etc.
│   drivers)    │  Enables reading the actual /boot partition
└───────┬───────┘
        │
        ▼
┌───────────────┐
│  Stage 2      │  /boot/grub/grub.cfg
│  (menu,       │  Displays boot menu, loads kernel + initrd
│   config)     │  Passes boot parameters to kernel
└───────────────┘
```

### Useful GRUB commands

```bash
# List GRUB menu entries
sudo grep -E "^menuentry|^submenu" /boot/grub/grub.cfg

# Set default boot entry
sudo grub-set-default 0

# Regenerate config after changes
sudo update-grub          # Debian/Ubuntu
sudo grub2-mkconfig -o /boot/grub2/grub.cfg  # RHEL/Fedora

# Enter GRUB rescue mode (if bootloader is broken)
# At GRUB prompt:
grub> ls                        # List partitions
grub> set root=(hd0,gpt2)      # Set boot partition
grub> linux /vmlinuz root=/dev/sda3
grub> initrd /initrd.img
grub> boot
```

---

## Boot Parameters

The kernel accepts parameters passed by the bootloader. These control kernel behavior at startup.

### Common boot parameters

```bash
# View current boot parameters
cat /proc/cmdline

# Example output:
# BOOT_IMAGE=/vmlinuz-5.15.0-generic root=/dev/sda3 ro quiet splash
```

Key parameters:

| Parameter | Purpose |
|---|---|
| `root=/dev/sda3` | Root filesystem location |
| `ro` | Mount root read-only initially |
| `quiet` | Suppress kernel log messages |
| `splash` | Show splash screen |
| `single` or `s` | Single-user/rescue mode |
| `init=/bin/sh` | Override init system (rescue shell) |
| `nomodeset` | Don't load video drivers (fix display issues) |
| `mem=512M` | Limit visible RAM |

### Rescue mode example

If your system won't boot normally, you can edit the GRUB entry at boot time:

```bash
# At GRUB menu, press 'e' to edit, then add to the linux line:
init=/bin/sh

# Press Ctrl+X to boot. You'll get a root shell without init system.
# Remount root as read-write to make changes:
mount -o remount,rw /
```

---

## Kernel Boot Sequence

Once the bootloader hands control to the kernel, the kernel takes over:

```
Bootloader jumps to kernel entry point
  │
  ▼
┌──────────────────────────────────┐
│  Decompress kernel image         │  (vmlinuz is gzip/lz4 compressed)
│  (arch/x86/boot/compressed/)     │
└───────────────┬──────────────────┘
                │
                ▼
┌──────────────────────────────────┐
│  Setup GDT (Global Descriptor    │  Define memory segments
│  Table) and IDT (Interrupt       │  Set up interrupt handlers
│  Descriptor Table)               │
└───────────────┬──────────────────┘
                │
                ▼
┌──────────────────────────────────┐
│  Enable paging                   │  Activate virtual memory
│  Setup page tables               │
└───────────────┬──────────────────┘
                │
                ▼
┌──────────────────────────────────┐
│  Call start_kernel()             │  C code entry point
│  (init/main.c)                   │
│  - setup memory zones            │
│  - initialize scheduler          │
│  - init IRQ subsystem            │
│  - calibrate delay loop          │
│  - call rest_init()              │
└───────────────┬──────────────────┘
                │
                ▼
┌──────────────────────────────────┐
│  kernel_init()                   │
│  - load initramfs               │
│  - mount root filesystem         │
│  - exec /sbin/init (PID 1)      │  ← Transitions to user space
└──────────────────────────────────┘
```

### Viewing the boot log

```bash
# Show kernel messages from boot
dmesg | head -40

# Show timestamps with boot messages
dmesg --time-format iso | head -20

# Show messages from current boot only (systemd)
journalctl -b 0 | head -50

# Show messages from previous boot
journalctl -b -1 | head -50

# Measure boot time
systemd-analyze
systemd-analyze blame | head -15
```

Example `dmesg` output:

```
[    0.000000] Linux version 5.15.0-generic (buildd@lcy01)
[    0.000000] Command line: BOOT_IMAGE=/vmlinuz root=/dev/sda3 ro quiet
[    0.000000] BIOS-provided physical RAM map:
[    0.000000]  BIOS-e820: [mem 0x00000000-0x0009fbff] usable
[    0.000000]  BIOS-e820: [mem 0x000f0000-0x000fffff] reserved
[    0.004000] Memory: 8192000K/8388608K available
[    0.342000] Calibrating delay loop... 5986.13 BogoMIPS
[    0.500000] pid_max: default: 32768 minimum: 301
[    1.200000] EXT4-fs (sda3): mounted filesystem with ordered data mode
[    2.100000] systemd[1]: Started Journal Service.
```

---

## Build It

The scripts and commands in this lesson let you inspect every stage of the boot process on a running Linux system. You don't need to build anything—just run the commands and observe the output.

The key commands to try:

```bash
# What firmware mode am I in?
[ -d /sys/firmware/efi ] && echo "UEFI" || echo "BIOS"

# What did the kernel see at boot?
dmesg | head -30

# What boot parameters were used?
cat /proc/cmdline

# What's in my GRUB config?
grep -E "^menuentry|^submenu" /boot/grub/grub.cfg 2>/dev/null

# How long did boot take?
systemd-analyze

# What services started, and in what order?
systemd-analyze critical-chain
```

---

## Use It

Every Linux system you will ever touch boots this way. When a server won't start, the answer is usually in one of these stages:

- **No POST** → hardware failure, check RAM/CPU
- **GRUB rescue prompt** → bootloader broken, partition table changed, disk failure
- **Kernel panic** → wrong root parameter, missing driver, corrupted initramfs
- **Emergency mode** → root filesystem won't mount, fsck needed
- **Slow boot** → `systemd-analyze blame` to find the offending service

Understanding the boot process turns "it won't start" from a mystery into a checklist.

---

## Ship It

Boot process reference (one-page summary):

```
POWER ON
  │
  ▼
POST (hardware test)
  │
  ├── BIOS path:  read MBR (512 bytes) → stage 1 bootloader
  │
  └── UEFI path:  read NVRAM → mount ESP → load .efi binary
  │
  ▼
GRUB stage 1 → stage 1.5 → stage 2 (grub.cfg)
  │
  ▼
Kernel loads: decompress → GDT/IDT → paging → start_kernel()
  │
  ▼
init (PID 1) → services → login
```

---

## Exercises

### Level 1 — Recall

1. What are the first 512 bytes of a disk called, and what is stored there?
2. What file contains GRUB's boot menu configuration?
3. What command shows kernel messages from the boot process?

### Level 2 — Comprehension

1. Explain why BIOS cannot boot from a disk larger than 2 TB. Relate this to the MBR structure.
2. Why does UEFI use a FAT32 partition (the ESP) instead of embedding boot code in the MBR?
3. Run `cat /proc/cmdline` on a Linux system and explain what each parameter does.

### Level 3 — Application

1. Your server displays "GRUB rescue" after a disk replacement. The new disk has the same partitions but the bootloader is gone. Write the exact GRUB rescue commands to manually boot a kernel located at `(hd0,gpt2)/vmlinuz` with root on `/dev/sda3`.
2. Run `systemd-analyze blame` and identify the three slowest services. For each one, explain whether it's safe to disable and what the consequence would be.
3. A colleague says "UEFI is just BIOS with a graphical menu." Write a paragraph correcting this misconception, listing at least four technical differences.
