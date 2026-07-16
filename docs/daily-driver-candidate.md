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

The gate verifies formatting, unit tests, Linux builds, release packaging and checksums, initramfs generation, service lifecycle, restart policy, target boot, mount handling, events, long-running supervision, repeated boot/shutdown loops, stuck-process handling, and shutdown escalation.

## Emergency Rescue Path

Keep a separate known-good boot entry and initramfs. For testing, always keep a rescue shell path that does not depend on `minit.normal=1`.

Rollback path:

1. Boot the previous entry or rescue shell.
2. Restore the previous initramfs.
3. Remove `minit.normal=1` and any experimental `minit.*` arguments from the failing boot entry.
4. Reboot into the known-good entry before trying a new package.

## Remaining Limits

- cgroups v2 only.
- Linux normal mode only.
- No device manager.
- No journal replacement.
- No user session manager.
- Security options fail closed unless explicitly implemented.
- Broader distro install validation is still limited to the included VM profile and smokes.
