# minit

`minit` is a Rust Linux init and service manager experiment targeting modern normal Linux distributions.

Current milestone: minimal VM/initramfs boot with `minitd` as PID 1.

Normal mode will require Linux with cgroups v2. Rescue/initramfs mode is degraded and only intended to mount basic filesystems, start a shell or getty, reap children, and shut down cleanly.

This repository is not daily-driver-ready yet.
