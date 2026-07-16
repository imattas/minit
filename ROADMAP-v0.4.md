# minit v0.4 Roadmap

`v0.4.0-experimental` should move `minit` from VM-proven core supervision into broader operator usefulness and safer distro evaluation. It remains experimental and must not claim daily-driver readiness.

## Release Goals

- Start independent target units through real bounded parallel execution while preserving deterministic dependency ordering and failure reporting.
- Capture service stdout/stderr into a bounded local log path and expose it through `minitctl logs` and `minitctl logs --follow`.
- Add stronger sandboxing proof, starting with seccomp where the host kernel supports it and explicit fail-closed behavior where it does not.
- Expand disposable distro-rootfs validation beyond the current Alpine minirootfs gate.
- Improve recovery mode and rollback documentation for failed boot targets and failed installs.
- Add first-pass conversion helpers for simple systemd, OpenRC, runit, and s6 service definitions.
- Make tagged experimental GitHub releases automatically use checked-in release notes and pre-release status.

## Task Order

1. Release workflow cleanup.
2. Persistent lifecycle log file and `logs --follow`.
3. Bounded parallel target execution.
4. Seccomp and sandbox proof.
5. Distro-rootfs validation expansion.
6. Recovery mode hardening and rollback docs.
7. Unit conversion helpers.
8. `v0.4.0-experimental` release notes, version bump, tag, and GitHub pre-release verification.

## Release Gate

The release is blocked until these pass:

```powershell
cargo fmt --check
cargo test
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-security.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-release.ps1 -Kernel C:\minit-vm\bzImage -BusyBoxPath C:\minit-vm\busybox -VmTimeoutSeconds 30
```

## Non-Goals

- No host install recommendation.
- No full systemd replacement claim.
- No user session manager.
- No device manager.
- No unsupported sandbox option should silently succeed.
