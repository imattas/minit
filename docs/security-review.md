# Security Review

This review tracks the current PID 1 trust boundary and the v0.5 hardening controls.

## Implemented Controls

- Unit TOML uses strict schema deserialization and rejects unknown fields.
- Service units require an absolute `exec.start[0]`; target units do not execute processes.
- Unit names are restricted to safe path characters before cgroup paths are derived.
- Unsupported security options fail closed instead of being silently ignored.
- `security.environment` entries are validated as explicit `KEY=value` pairs.
- Spawned services run with an explicitly cleared environment plus configured allowlisted entries.
- Linux service spawning applies configured numeric or `root` GID/UID before `no_new_privileges`.
- `security.seccomp = "deny-write"` installs a Linux seccomp filter before `exec` that returns `EPERM` for `write(2)`.
- cgroups v2 resource controls write `memory.max`, `cpu.max`, and `pids.max` before process attachment.
- Control socket directory and socket permissions are constrained to owner access.
- Unit parsing and IPC decoding have fuzz harnesses under `fuzz/`.

## Accepted Risks

- User and group lookup supports only `root` and numeric IDs. Name-service lookup is intentionally not in PID 1 yet.
- Seccomp support is intentionally limited to the `deny-write` profile. It does not block `writev`, `pwrite64`, file opening, networking, or filesystem mutation through other syscalls.
- Unsupported seccomp profiles, `private_tmp`, read-only path policy, and read-write path policy are rejected until implemented.
- The control protocol is local Unix socket JSON; there is no remote transport.
- cgroup resource values are validated by the kernel when written, not by a full local grammar.
- The fuzz harnesses are run by the scheduled and pull-request security workflow as bounded smokes.
- Dependency advisories are checked by `cargo audit` in the security workflow.

## Local Fuzz Commands

```powershell
cargo install cargo-audit
cargo install cargo-fuzz
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-security.ps1
```

On Windows, the local script runs dependency audit and skips fuzz by default because `cargo fuzz` requires the LLVM ASAN runtime. Run the fuzz smokes on Linux or pass `-RequireFuzz` on Windows after installing the ASAN runtime. The GitHub security workflow runs fuzz on Ubuntu.

## Follow-Up Hardening

- Expand bounded fuzz smokes into longer overnight fuzz campaigns with corpus retention.
- Add named user/group lookup outside PID 1 or in a tightly scoped helper.
- Implement or keep rejecting filesystem sandboxing options.
- Expand the seccomp profile library beyond the current minimal `deny-write` smoke profile.
