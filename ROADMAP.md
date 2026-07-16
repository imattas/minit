# minit Roadmap

`minit` is currently an experimental Rust Linux init and service manager. The project has a verified first milestone: `minitd` can boot as PID 1 in a VM/initramfs, expose a Unix control socket, run `minitctl`, start/stop/restart services, supervise through cgroups v2, clean up service cgroups, apply basic restart policy, stop managed services during shutdown, and pass the full release verification gate.

This roadmap is intentionally strict. A phase is not complete until the code, tests, Linux builds, and VM evidence are in place.

## Current Baseline

Completed:

- Rust workspace with `minit-core`, `minitd`, `minitctl`, and `minit-testkit`.
- Rescue/initramfs mode with basic filesystem setup, shell/getty fallback, child reaping, and shutdown.
- Normal mode boot in QEMU with cgroups v2 mounted.
- Unit loading from TOML examples.
- Unix control socket and `minitctl status/start/stop/restart`.
- Service cgroups under `/sys/fs/cgroup/minit`.
- Start, stop, restart, cgroup cleanup, restart policy, and shutdown-stop VM smokes.
- Release verification script: `tools/verify-release.ps1`.
- GitHub CI for formatting, tests, and Linux `musl` builds.

Current release gate:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-release.ps1 -Kernel C:\minit-vm\bzImage -BusyBoxPath C:\minit-vm\busybox
```

## Release Definitions

### v0.1 Experimental VM Release

Goal: a clearly labeled experimental release that proves the core model in QEMU.

Must have:

- Passing full release gate.
- README warning that it is not daily-driver-ready.
- Minimal release notes describing verified capabilities and known gaps.
- Reproducible build instructions for `minitd`, `minitctl`, and the VM initramfs.
- No known panic path in normal VM smokes.

### v0.2 Supervision Release

Goal: make service supervision behavior predictable under failure, stuck services, and repeated restart loops.

Must have:

- SIGTERM before SIGKILL stop flow.
- Configurable stop timeout.
- Restart backoff instead of immediate retry only.
- Restart-rate limiting with test coverage for storm prevention.
- `minitctl status` includes restart attempt count, last exit status, and cgroup path.
- VM tests for crash loop, ignored SIGTERM, and shutdown with stuck service.

### v0.3 Boot Graph Release

Goal: move from single-service control toward target-based boot orchestration.

Must have:

- Target units.
- Dependency graph execution in normal mode.
- Parallel start where dependency ordering allows it.
- Deterministic failure propagation for `requires`, `wants`, `before`, and `after`.
- `minitctl explain <unit>` showing dependency blockers and reason chain.
- VM boot profile that starts a small multi-unit target.

### v0.4 Distro Profile Release

Goal: prove `minit` can manage real distro-style services in a VM.

Must have:

- Profiles for getty/login, sshd, networking, dbus, and a simple display/login target.
- Packaging scripts for at least one test distro image.
- Conversion notes for simple systemd/OpenRC service files.
- Boot, status, restart, and shutdown validation for each profile.
- Clear list of unsupported unit features.

### v0.5 Hardening Release

Goal: reduce PID 1 attack surface and fail closed on unsafe configuration.

Must have:

- Strict unit schema validation.
- Clear handling for unsupported security options.
- UID/GID switching.
- Environment allowlist/parsing rules.
- Resource limits through cgroups v2.
- Control socket permission tests.
- Fuzzing for unit parsing and IPC decoding.
- Security review document with accepted risks and follow-up issues.

### v0.6 Storage and Mount Release

Goal: support essential boot storage flows without taking on broad system policy.

Must have:

- Mount units.
- Swap units.
- Ordered unmount/deactivate during shutdown.
- Failure behavior for required and optional mounts.
- VM tests for mount success, mount failure, and clean shutdown.

### v0.7 Observability Release

Goal: make failures understandable without requiring a debugger.

Must have:

- Structured diagnostic events.
- Recent event buffer available through `minitctl`.
- `minitctl status` shows cgroup, main PID, state, last exit, restart counters, and failure reason.
- `minitctl explain` for graph and lifecycle state.
- Boot timeline summary.
- Shutdown timeline summary.

### v0.8 Installer and Packaging Release

Goal: make evaluation possible without hand-built artifacts.

Must have:

- Packaged binaries.
- Example install layout.
- Initramfs integration notes.
- Rollback instructions.
- VM image or reproducible image build script.
- Signed or checksummed release artifacts.

### v1.0 Daily-Driver Candidate

Goal: a cautious daily-driver candidate for advanced users in disposable or recoverable systems.

Must have:

- All previous phase gates passing.
- Repeated boot/shutdown loop testing.
- Long-running service supervision test.
- Crash-loop and stuck-process tests.
- Security hardening review completed.
- At least one distro VM profile proven across install, boot, service management, and rollback.
- Clear emergency rescue path.
- No known data-loss shutdown issue.

## Engineering Workstreams

### Service Supervision

Next work:

- Add graceful stop escalation: SIGTERM, wait, SIGKILL.
- Track last exit status and signal.
- Track restart attempt windows by time, not only count.
- Add restart backoff and max delay.
- Prevent restart storms from blocking PID 1 responsiveness.

Acceptance:

- Unit tests for all restart policies.
- VM tests for failure, crash loop, ignored termination, and clean shutdown.

### Dependency and Target Management

Next work:

- Add target unit type.
- Execute graph start plans in normal mode.
- Add parallel start scheduler with deterministic ordering.
- Implement failure propagation rules.

Acceptance:

- Graph tests for target startup and failure.
- VM boots a multi-service target.
- `minitctl explain` identifies blockers.

### cgroups v2

Next work:

- Add resource controls: memory, CPU, pids.
- Report cgroup paths and current membership.
- Fail normal mode clearly if cgroups v2 is unavailable or unwritable.
- Add cgroup cleanup tests for crash and shutdown paths.

Acceptance:

- Unit tests for resource-control writes.
- VM test proves service cgroup creation, attachment, cleanup, and reporting.

### Security

Next work:

- Enforce UID/GID switching.
- Enforce `no_new_privileges` with direct VM proof.
- Validate environment entries.
- Fail closed on unsupported security options.
- Fuzz unit parsing and IPC decoding.
- Document the PID 1 trust boundary.

Acceptance:

- Security options either work or produce explicit errors.
- No unsafe unit name reaches filesystem-backed paths.
- Control socket remains owner-only.
- Fuzz targets run in CI or documented local gate.

### Shutdown

Next work:

- Add global shutdown deadline.
- Stop services in reverse dependency order.
- Escalate remaining cgroups.
- Add mount/swap deactivation when those units exist.
- Keep diagnostics available late in shutdown.

Acceptance:

- VM test for stuck service shutdown.
- VM test for reverse-order shutdown.
- No indefinite shutdown hang in known smoke tests.

### CLI and UX

Next work:

- Add `minitctl list`.
- Add `minitctl explain`.
- Add `minitctl logs` or event-buffer view.
- Improve error messages for missing units and invalid requests.
- Add machine-readable output mode.

Acceptance:

- CLI tests for every command.
- Commands remain usable when `minitd` is unavailable.
- Output includes enough context to debug boot failures.

### Packaging and Release

Next work:

- Add release notes template.
- Add artifact checksum generation.
- Add reproducible VM image build.
- Add installation and rollback docs.
- Keep CI and local full VM gate aligned.

Acceptance:

- A release can be built from a clean checkout using documented commands.
- VM artifacts are reproducible or clearly described as local smoke artifacts.

## Non-Goals for Now

- No cgroups v1 support.
- No attempt to replace full systemd scope in PID 1.
- No journal replacement in PID 1.
- No network manager in PID 1.
- No device manager in PID 1.
- No user session manager until system service management is mature.

## Immediate Next Steps

1. Cut `v0.1.0-experimental` only after the current full release gate passes from a clean checkout.
2. Add release notes and known limitations for v0.1.
3. Start v0.2 with graceful stop escalation and stuck-service VM proof.
4. Add restart backoff and rate-window enforcement.
5. Add status fields for last exit, restart count, and cgroup path.
6. Keep every phase push gated by `tools\verify-release.ps1`.

## Quality Bar

Every phase should preserve these rules:

- PID 1 must stay small and understandable.
- Normal mode requires Linux with cgroups v2.
- Unsafe configuration must fail closed.
- VM evidence matters more than unit tests alone.
- Release claims must match what the release gate actually proves.
