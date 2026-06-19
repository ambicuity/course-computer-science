#!/usr/bin/env bash
# Container primitives: cgroups and namespace interaction from the shell.
# Phase 07 — Operating Systems, Lesson 19
#
# Run: sudo bash run.sh
set -euo pipefail

echo "=== Container Primitives Shell Demo ==="
echo ""

# --- cgroups v2 (or v1 fallback) ---
echo "--- cgroups: Limit CPU and Memory ---"

CGROUP_NAME="shell_container_demo"
CGROUP_BASE="/sys/fs/cgroup"

if [ -d "$CGROUP_BASE/cpu" ]; then
    # cgroups v1
    echo "Using cgroups v1"

    # CPU: limit to 25% of one core
    CPU_CG="$CGROUP_BASE/cpu/$CGROUP_NAME"
    sudo mkdir -p "$CPU_CG"
    echo 25000 | sudo tee "$CPU_CG/cpu.cfs_quota_us" > /dev/null
    echo 100000 | sudo tee "$CPU_CG/cpu.cfs_period_us" > /dev/null
    echo "  CPU: limited to 25% of one core"

    # Memory: limit to 128MB
    MEM_CG="$CGROUP_BASE/memory/$CGROUP_NAME"
    sudo mkdir -p "$MEM_CG"
    echo 134217728 | sudo tee "$MEM_CG/memory.limit_in_bytes" > /dev/null
    echo "  Memory: limited to 128MB"

    # Run a process in this cgroup
    echo "  Running 'yes > /dev/null' in cgroup (PID added to tasks)..."
    yes > /dev/null &
    YPID=$!
    echo $YPID | sudo tee "$CPU_CG/tasks" > /dev/null
    echo $YPID | sudo tee "$MEM_CG/tasks" > /dev/null
    echo "  Process $YPID is now limited by cgroup"
    sleep 1
    kill $YPID 2>/dev/null || true
    wait $YPID 2>/dev/null || true

    # Cleanup
    sudo rmdir "$CPU_CG" 2>/dev/null || true
    sudo rmdir "$MEM_CG" 2>/dev/null || true
    echo "  cgroup cleaned up"

elif [ -d "$CGROUP_BASE/$CGROUP_BASE" ]; then
    # cgroups v2
    echo "Using cgroups v2"
    CG="$CGROUP_BASE/$CGROUP_NAME"
    sudo mkdir -p "$CG"
    echo "+cpu +memory" | sudo tee "$CGROUP_BASE/cgroup.subtree_control" > /dev/null
    echo 250000 1000000 | sudo tee "$CG/cpu.max" > /dev/null
    echo 134217728 | sudo tee "$CG/memory.max" > /dev/null
    echo "  CPU: 25% limit, Memory: 128MB limit"
    sudo rmdir "$CG" 2>/dev/null || true
fi

echo ""

# --- Namespace inspection ---
echo "--- Namespace Inspection ---"

echo "  Current process namespaces (from /proc/self/ns/):"
ls -la /proc/self/ns/ 2>/dev/null | grep -E '(pid|net|mnt|uts|ipc|user)' | awk '{print "    " $NF}'

echo ""
echo "  Namespace inode numbers (unshare = different namespace):"
for ns in pid net mnt uts ipc user cgroup; do
    if [ -e "/proc/self/ns/$ns" ]; then
        INO=$(readlink "/proc/self/ns/$ns" | grep -oP '\d+')
        echo "    $ns: $INO"
    fi
done

echo ""

# --- Unshare demo ---
echo "--- Unshare: Create New Namespace ---"

echo "  Creating a process in a new UTS namespace..."
sudo unshare --uts bash -c '
    hostname my-container-host
    echo "    Inside new UTS namespace:"
    echo "      hostname = $(hostname)"
    echo "      PID = $$"
'
echo "  Back in original namespace:"
echo "    hostname = $(hostname)"

echo ""

# --- Enter existing namespace ---
echo "--- nsenter: Enter an Existing Namespace ---"

echo "  Forking a background process in new namespaces..."
sudo unshare --pid --mount --fork --mount-proc bash -c '
    sleep 30
' &
BG_PID=$!
sleep 0.5

echo "  Background process PID (host view): $BG_PID"
echo "  Its namespaces:"
ls -la "/proc/$BG_PID/ns/" 2>/dev/null | grep -E '(pid|mnt)' | awk '{print "    " $NF}'

echo "  Entering its PID+mount namespace..."
sudo nsenter --pid --mount -t "$BG_PID" bash -c '
    echo "    Inside entered namespace:"
    echo "      My PID: $$"
    echo "      Processes visible:"
    ps aux 2>/dev/null | head -5 | sed "s/^/        /"
' || echo "    (nsenter failed — may need more privileges)"

kill $BG_PID 2>/dev/null || true
wait $BG_PID 2>/dev/null || true

echo ""
echo "Done."
