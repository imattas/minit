# minit v1.0.0 Readiness Gate

`minit` v1.0.0 is only releasable when the claim is backed by a repeatable local gate. The gate is intentionally stricter than the normal release verifier because v1.0.0 is the first cautious daily-driver-candidate milestone.

## Required Command

Run the full v1 gate with a Linux kernel and BusyBox binary:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-v1-readiness.ps1 `
  -Kernel C:\minit-vm\bzImage `
  -BusyBoxPath C:\minit-vm\busybox `
  -VmTimeoutSeconds 30
```

The full gate runs:

- source release verification: formatting, tests, Linux `musl` builds, release packaging, and checksums.
- package install and rollback validation in a disposable root tree. In full mode, the installed layout is also used to build and boot an initramfs smoke.
- normal-mode VM verification with the extended boot-loop stress gate.
- security verification through `tools\verify-security.ps1`.
- Alpine minirootfs distro-rootfs boot validation unless `-SkipAlpine` is explicitly supplied.
- full-disk VM validation with a generated ext4 root image, `minitd` as PID 1 after `switch_root`, `minitctl status`, `list`, `restart`, `events`, `logs`, boot-timeline output, clean shutdown, and preserved serial transcripts under `tools\vm\artifacts\full-disk-transcripts`.
- optional Debian and Arch rootfs gates when `-RequireDebian` or `-RequireArch` is supplied.

The script writes machine-readable evidence to `tools\release\v1-readiness-evidence.json`.

VM smoke steps use bounded retries in `tools\verify-release.ps1` because QEMU launch/capture on developer hosts can fail before `minitd` starts. Source, build, package, and security steps do not retry. A VM step is still blocked if all retry attempts fail.

The full-disk gate downloads and extracts the pinned Alpine `linux-virt` package so the disk boot uses a kernel and module tree that can discover and mount the generated ext4 disk. The `-Kernel` input is still required for the existing initramfs and distro-rootfs smokes.

## Source-Only Preflight

Use source-only mode while developing. This does not prove v1.0.0 readiness:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-v1-readiness.ps1 -SourceOnly
```

Source-only mode is useful before pushing a phase, but release notes must not call a build v1.0.0-ready from source-only evidence.

## Optional Distro Gates

Run Debian validation when a local Debian rootfs is available:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-v1-readiness.ps1 `
  -Kernel C:\minit-vm\bzImage `
  -BusyBoxPath C:\minit-vm\busybox `
  -RequireDebian `
  -DebianRootfsTar C:\minit-vm\debian-rootfs.tar
```

Run Arch validation when a local Arch rootfs is available:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-v1-readiness.ps1 `
  -Kernel C:\minit-vm\bzImage `
  -BusyBoxPath C:\minit-vm\busybox `
  -RequireArch `
  -ArchRootfsTar C:\minit-vm\arch-rootfs.tar.zst
```

## Release Rule

Before tagging `v1.0.0`, the maintainer must attach or summarize fresh evidence from a full `tools\verify-v1-readiness.ps1` run. If any required local input is missing, v1.0.0 is blocked rather than downgraded silently.
