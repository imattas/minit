# minit Release Template

## Verification

- `cargo fmt --check`
- `cargo test`
- `cargo build -p minitd --target x86_64-unknown-linux-musl`
- `cargo build -p minitctl --target x86_64-unknown-linux-musl`
- `powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-release.ps1 -Kernel <bzImage> -BusyBoxPath <busybox>`

## Artifacts

- `minitd`
- `minitctl`
- example unit files
- release package zip
- `SHA256SUMS`

## Known Limits

- Linux-only normal mode.
- cgroups v2 required.
- No cgroups v1 support.
- No device manager, network manager, journal replacement, or user session manager.
- Advanced unit sandboxing remains intentionally fail-closed unless implemented.
