# Filesystem Deep-Cut Notes

- ext4: journaling + extent-based allocation.
- btrfs: copy-on-write trees, snapshots, checksummed metadata/data.
- ZFS: pooled storage, end-to-end checksums, integrated volume management.
- Tradeoff: CoW features improve reliability but can affect write amplification.
