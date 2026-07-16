# Distro Profiles

`minit` profiles are experimental unit sets for disposable VM evaluation. They are not a promise that an arbitrary Linux distro can replace its native init system yet.

## Profile Types

### Alpine Minirootfs

`config/profiles/alpine-minirootfs` is the first reproducible distro-rootfs validation profile. It intentionally uses only commands available in Alpine's minirootfs so the VM gate does not depend on package installation.

It includes:

- `network.service`: long-running placeholder for profile boot ordering.
- `demo-sleep`: long-running service with restart policy and `no_new_privileges`.
- `alpine-smoke.target`: target that groups the two services.

Run it with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\verify-alpine-minirootfs.ps1 -Kernel C:\minit-vm\bzImage
```

The verifier downloads and SHA256-checks Alpine's minirootfs, builds a `minit` initramfs from that root filesystem, and boots `alpine-smoke.target` in QEMU.

### Debian Minirootfs

`tools/vm/verify-debian-minirootfs.ps1` validates a local Debian root filesystem input without downloading one. Pass exactly one of:

- `-RootfsTar <path>` for a local Debian rootfs tarball such as one produced by `debootstrap --make-tarball` or another trusted build pipeline.
- `-RootfsDir <path>` for an already extracted Debian root filesystem directory.

Run it with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\verify-debian-minirootfs.ps1 `
  -Kernel C:\minit-vm\bzImage `
  -RootfsTar C:\minit-vm\debian-rootfs.tar
```

The verifier builds `minitd` and `minitctl` for `x86_64-unknown-linux-musl`, injects them plus the selected profile into the disposable rootfs initramfs, then boots QEMU status, list, and clean-shutdown smokes. By default it uses `config/profiles/minimal-distro` and checks `sshd.service` is visible to `minitctl`; it does not start distro services or claim the rootfs is install-ready.

### Arch Rootfs

`tools/vm/verify-arch-rootfs.ps1` validates a local Arch root filesystem input without downloading one. Pass exactly one of:

- `-RootfsTar <path>` for a local Arch rootfs tarball. The WSL `tar` used by the script must support the tarball compression format.
- `-RootfsDir <path>` for an already extracted Arch root filesystem directory.

Run it with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\verify-arch-rootfs.ps1 `
  -Kernel C:\minit-vm\bzImage `
  -RootfsTar C:\minit-vm\arch-rootfs.tar.zst
```

The Arch verifier uses the same smoke sequence as the Debian verifier: status, list, and clean shutdown against the selected profile. It is a disposable rootfs compatibility gate, not an installer or package manager integration test.

### Minimal Distro Template

`config/profiles/minimal-distro` is a distro-style template for a root filesystem that already has the referenced binaries and configuration files.

It includes:

- `networking.service`: baseline loopback setup through `/sbin/ip`.
- `sshd.service`: OpenSSH daemon profile that requires `networking.service`.
- `dbus.service`: D-Bus system bus profile.
- `getty@ttyS0.service`: serial login profile.
- `display-login.target`: target that groups networking, dbus, sshd, and serial login.

This profile validates the unit model and dependency graph. Real boot proof requires an initramfs or VM image containing `/sbin/ip`, `/usr/sbin/sshd`, `/usr/bin/dbus-daemon`, `/sbin/getty`, and their normal distro configuration.

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
- There is no device manager, journal replacement, user session manager, or service-file converter in this profile.
- Unsupported security settings fail closed at unit parse or start time instead of being ignored.
- Debian and Arch rootfs verifiers require local rootfs inputs; the release gate does not download or depend on large distro images.
- Debian and Arch rootfs verifiers currently prove `minitd` normal-mode boot, `minitctl status`, `minitctl list`, and clean VM shutdown only. Starting real distro services remains a later profile-specific gate.

## Recovery Rule

Only test profiles from a boot entry with a known-good fallback. Keep a previous initramfs, keep a rescue shell path, and remove `minit.normal=1` from the failing boot entry to return to the prior init path.
