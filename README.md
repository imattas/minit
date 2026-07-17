# minit

`minit` is a Rust Linux init and service manager experiment targeting modern normal Linux distributions.

Current milestone: VM-proven v0.4.0 release with `minitd` as PID 1, cgroups v2 supervision, `minitctl` status/list/start/stop/restart/explain/graph/events/logs/boot-timeline, JSON dependency graph output, target boot, failed boot-target rescue fallback, mount/swap units, diagnostic events, hardening proof, seccomp deny-write proof, conservative unit conversion helpers, release packaging, and QEMU smoke coverage.

Normal mode will require Linux with cgroups v2. Rescue/initramfs mode is degraded and only intended to mount basic filesystems, start a shell or getty, reap children, and shut down cleanly.

This repository is not daily-driver-ready yet.

Release notes and known limitations for the latest release are in [docs/releases/v0.4.0.md](docs/releases/v0.4.0.md).

The v1.0.0 readiness gate is documented in [docs/v1-readiness.md](docs/v1-readiness.md). Source-only v1 checks are useful for development, but v1.0.0 must not be tagged until the full VM and distro-rootfs readiness gate has fresh evidence.

## Release verification

Run the source-only gate:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-release.ps1
```

Run the full local VM gate when a Linux kernel and BusyBox binary are available:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-release.ps1 -Kernel C:\minit-vm\bzImage -BusyBoxPath C:\minit-vm\busybox
```

Run the extended VM stress loop for release-candidate validation:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-release.ps1 -Kernel C:\minit-vm\bzImage -BusyBoxPath C:\minit-vm\busybox -ExtendedVmStress -StressBootCount 25
```

The full gate checks formatting, tests, Linux `musl` builds, release packaging/checksums, normal-mode VM status, service lifecycle, cgroup cleanup, restart policy, target boot, failed boot-target recovery, dependency graph reporting, boot timeline output, recent lifecycle logs, dependency failure handling, hardening proof, seccomp deny-write proof, mount handling, diagnostic events, long-running supervision, repeated boot/shutdown, stuck stop escalation, and managed shutdown behavior.
