// Build a Container Runtime
// Run: go build -o minicontainer main.go && ./minicontainer spec.json
// Note: Linux only — uses clone(), cgroups v2, namespace flags
//
// Architecture:
//   Container Spec (JSON) → Cgroup Setup → Namespace Clone → Process Start → Wait
//
// Implements a minimal container runtime with namespace isolation and cgroup limits.

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"syscall"
)

// =============================================================================
// Step 1: Container Spec Parser
// =============================================================================

type ContainerSpec struct {
	Rootfs   string            `json:"rootfs"`
	Args     []string          `json:"args"`
	Env      map[string]string `json:"env"`
	Hostname string            `json:"hostname"`
	Limits   ResourceLimits    `json:"limits"`
}

type ResourceLimits struct {
	MemoryLimitMB int `json:"memory_limit_mb"`
	CPUShares     int `json:"cpu_shares"`
	PidLimit      int `json:"pid_limit"`
}

func loadSpec(path string) (*ContainerSpec, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("reading spec: %w", err)
	}
	var spec ContainerSpec
	if err := json.Unmarshal(data, &spec); err != nil {
		return nil, fmt.Errorf("parsing spec: %w", err)
	}
	return &spec, nil
}

// =============================================================================
// Step 2: Cgroup Setup
// =============================================================================

const cgroupRoot = "/sys/fs/cgroup"

func setupCgroup(containerID string, limits ResourceLimits) (string, error) {
	cgroupPath := filepath.Join(cgroupRoot, "mini-container", containerID)

	if err := os.MkdirAll(cgroupPath, 0755); err != nil {
		return "", fmt.Errorf("creating cgroup: %w", err)
	}

	if limits.MemoryLimitMB > 0 {
		limitBytes := limits.MemoryLimitMB * 1024 * 1024
		memPath := filepath.Join(cgroupPath, "memory.max")
		if err := os.WriteFile(memPath, []byte(strconv.Itoa(limitBytes)), 0644); err != nil {
			return "", fmt.Errorf("setting memory limit: %w", err)
		}
	}

	if limits.CPUShares > 0 {
		cpuPath := filepath.Join(cgroupPath, "cpu.weight")
		if err := os.WriteFile(cpuPath, []byte(strconv.Itoa(limits.CPUShares)), 0644); err != nil {
			return "", fmt.Errorf("setting CPU shares: %w", err)
		}
	}

	if limits.PidLimit > 0 {
		pidPath := filepath.Join(cgroupPath, "pids.max")
		if err := os.WriteFile(pidPath, []byte(strconv.Itoa(limits.PidLimit)), 0644); err != nil {
			return "", fmt.Errorf("setting PID limit: %w", err)
		}
	}

	return cgroupPath, nil
}

func addToCgroup(cgroupPath string, pid int) error {
	procsPath := filepath.Join(cgroupPath, "cgroup.procs")
	return os.WriteFile(procsPath, []byte(strconv.Itoa(pid)), 0644)
}

func cleanupCgroup(cgroupPath string) error {
	return os.RemoveAll(cgroupPath)
}

// =============================================================================
// Step 3: Container Runtime
// =============================================================================

func runContainer(spec *ContainerSpec) error {
	containerID := "container-001"

	cgroupPath, err := setupCgroup(containerID, spec.Limits)
	if err != nil {
		return fmt.Errorf("cgroup setup: %w", err)
	}
	defer cleanupCgroup(cgroupPath)

	cmd := exec.Command(spec.Args[0], spec.Args[1:]...)
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	cmd.SysProcAttr = &syscall.SysProcAttr{
		Cloneflags: syscall.CLONE_NEWPID | syscall.CLONE_NEWNS | syscall.CLONE_NEWUTS,
	}

	env := []string{"container=true", fmt.Sprintf("hostname=%s", spec.Hostname)}
	for k, v := range spec.Env {
		env = append(env, fmt.Sprintf("%s=%s", k, v))
	}
	cmd.Env = env

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("starting container: %w", err)
	}

	fmt.Printf("Container %s started (PID %d)\n", containerID, cmd.Process.Pid)

	if err := addToCgroup(cgroupPath, cmd.Process.Pid); err != nil {
		cmd.Process.Kill()
		return fmt.Errorf("adding to cgroup: %w", err)
	}

	return cmd.Wait()
}

func main() {
	if len(os.Args) < 2 {
		fmt.Println("Usage: minicontainer <spec.json>")
		fmt.Println("\nExample spec.json:")
		fmt.Println(`  {"rootfs":"/","args":["/bin/sh","-c","echo hello"],"env":{},"hostname":"mycontainer","limits":{"memory_limit_mb":128,"cpu_shares":1024,"pid_limit":100}}`)
		os.Exit(1)
	}

	spec, err := loadSpec(os.Args[1])
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error loading spec: %v\n", err)
		os.Exit(1)
	}

	if err := runContainer(spec); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}
