# minit Daily-Driver Candidate Notes

This repository can produce a cautious daily-driver candidate for advanced users in disposable or recoverable systems. It is not a general systemd replacement.

## Required Gate

Run the full release gate before calling a build daily-driver-candidate ready:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-release.ps1 `
  -Kernel C:\minit-vm\bzImage `
  -BusyBoxPath C:\minit-vm\busybox `
  -VmTimeoutSeconds 30
```

The gate verifies formatting, unit tests, Linux builds, release packaging and checksums, initramfs generation, service lifecycle, restart policy, target boot, failed boot target rescue fallback, required-vs-wanted dependency failure behavior, mount handling, events, recent lifecycle logs, graph output, boot timeline output, long-running supervision, repeated boot/shutdown loops, stuck-process handling, and shutdown escalation.

For release-candidate stress validation, run the same gate with a longer boot loop:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-release.ps1 `
  -Kernel C:\minit-vm\bzImage `
  -BusyBoxPath C:\minit-vm\busybox `
  -VmTimeoutSeconds 30 `
  -ExtendedVmStress `
  -StressBootCount 25
```

Run the Alpine minirootfs distro-rootfs gate before publishing a daily-driver candidate:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\verify-alpine-minirootfs.ps1 `
  -Kernel C:\minit-vm\bzImage
```

When local Debian or Arch rootfs inputs are available, run the optional distro-rootfs gates too:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\verify-debian-minirootfs.ps1 `
  -Kernel C:\minit-vm\bzImage `
  -RootfsTar C:\minit-vm\debian-rootfs.tar

powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\verify-arch-rootfs.ps1 `
  -Kernel C:\minit-vm\bzImage `
  -RootfsTar C:\minit-vm\arch-rootfs.tar.zst
```

The Debian and Arch gates intentionally require local rootfs tarballs or extracted root directories. They verify normal-mode boot, `minitctl status`, `minitctl list`, and clean shutdown for the selected profile; they do not start real distro daemons or prove install readiness.

## Release Artifact Integrity

Tag builds produce checksums and GitHub artifact attestations. Maintainers can also create local GPG detached signatures:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\sign-release.ps1 -OutputDir tools\release\artifacts -GpgKey <key-id>
```

## Emergency Rescue Path

Keep a separate known-good boot entry and initramfs. For testing, always keep a rescue shell path that does not depend on `minit.normal=1`.

Rollback path:

1. Boot the previous entry or rescue shell.
2. Restore the previous initramfs.
3. Remove `minit.normal=1`, `minit.boot_target=...`, and any experimental `minit.*` arguments from the failing boot entry.
4. Reboot into the known-good entry before trying a new package.

If a configured `minit.boot_target=<target>` fails during boot, `minitd` stops managed units and enters rescue mode. That fallback is a last-resort recovery path, not a substitute for keeping a separate known-good boot entry.

## Remaining Limits

- cgroups v2 only.
- Linux normal mode only.
- No device manager.
- No journal replacement.
- No user session manager.
- Recent logs are a bounded in-memory lifecycle buffer, not persistent stdout/stderr capture.
- `minitctl boot-timeline` reports current boot milestones, not per-unit startup duration analysis.
- Security options fail closed unless explicitly implemented.
- Broader distro install validation is still limited to disposable VM profiles and smokes.
- Debian and Arch validation is optional unless release notes explicitly name the local rootfs input that was tested.
