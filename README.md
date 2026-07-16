# minit

`minit` is a Rust Linux init and service manager experiment targeting modern normal Linux distributions.

Current milestone: minimal VM/initramfs boot with `minitd` as PID 1.

Normal mode will require Linux with cgroups v2. Rescue/initramfs mode is degraded and only intended to mount basic filesystems, start a shell or getty, reap children, and shut down cleanly.

This repository is not daily-driver-ready yet.

## Release verification

Run the source-only gate:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-release.ps1
```

Run the full local VM gate when a Linux kernel and BusyBox binary are available:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-release.ps1 -Kernel C:\minit-vm\bzImage -BusyBoxPath C:\minit-vm\busybox
```

The full gate checks formatting, tests, Linux `musl` builds, normal-mode VM status, service restart, cgroup cleanup, restart policy, and managed shutdown stop behavior.
