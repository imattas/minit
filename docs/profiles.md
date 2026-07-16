# Distro Profiles

`config/profiles/minimal-distro` is a first distro-style profile for disposable VM evaluation.

It includes:

- `networking.service`: baseline loopback setup.
- `sshd.service`: OpenSSH daemon profile.
- `dbus.service`: D-Bus system bus profile.
- `getty@ttyS0.service`: serial login profile.
- `display-login.target`: target that groups networking, dbus, sshd, and serial login.

These units are intentionally conservative. They validate the unit model and dependency graph, but real boot proof requires a VM image or initramfs containing the referenced distro binaries and configuration files.

## Validate

```powershell
cargo test -p minit-core parses_all_profile_unit_files
```

## Package a Profile Initramfs

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\build-profile-initramfs.ps1 `
  -Profile minimal-distro `
  -MinitdPath target\x86_64-unknown-linux-musl\debug\minitd `
  -MinitctlPath target\x86_64-unknown-linux-musl\debug\minitctl `
  -BusyBoxPath C:\minit-vm\busybox `
  -Output tools\vm\artifacts\minit-profile-initramfs.cpio
```

## Current Limits

- No unit-file converter is automatic yet.
- No broad systemd compatibility is claimed.
- The profile assumes distro-provided binaries exist at the configured absolute paths.
- The profile is intended for disposable VM validation before any host use.
## Alpine Minirootfs

`config/profiles/alpine-minirootfs` is the first reproducible distro-rootfs validation profile. It intentionally uses only commands available in Alpine's minirootfs so the VM gate does not depend on package installation.

Run it with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\verify-alpine-minirootfs.ps1 -Kernel C:\minit-vm\bzImage
```

The verifier downloads and SHA256-checks Alpine's minirootfs, builds a `minit` initramfs from that root filesystem, and boots `alpine-smoke.target` in QEMU.
