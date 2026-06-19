#!/usr/bin/env bash
set -euo pipefail

echo "Boot chain overview"
echo "1) Firmware init (BIOS/UEFI)"
echo "2) Bootloader stage (GRUB/systemd-boot)"
echo "3) Kernel load + initramfs"
echo "4) PID 1 starts user space"
