# minit Release Template

## Verification

- `cargo fmt --check`
- `cargo test`
- `cargo build -p minitd --target x86_64-unknown-linux-musl`
- `cargo build -p minitctl --target x86_64-unknown-linux-musl`
- `powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-release.ps1 -Kernel <bzImage> -BusyBoxPath <busybox>`
- `powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-security.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\verify-alpine-minirootfs.ps1 -Kernel <bzImage>`

## Artifacts

- `minitd`
- `minitctl`
- example unit files
- release package zip
- `SHA256SUMS`
- GitHub artifact attestation for tag builds
- optional local `.asc` detached signatures

## Signing

Tag pushes matching `v*` run `.github/workflows/release.yml`. The workflow builds release artifacts, publishes the zip and checksum file, and creates GitHub artifact attestations using the repository workflow identity.

Maintainers with a local GPG key can additionally create detached signatures:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\sign-release.ps1 -OutputDir tools\release\artifacts -GpgKey <key-id>
```

## Known Limits

- Linux-only normal mode.
- cgroups v2 required.
- No cgroups v1 support.
- No device manager, network manager, journal replacement, or user session manager.
- Advanced unit sandboxing remains intentionally fail-closed unless implemented.
