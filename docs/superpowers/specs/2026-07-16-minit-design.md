# minit Design Spec

Date: 2026-07-16

## 1. Purpose

`minit` is a Linux init and service manager intended to become a serious daily-driver-capable replacement init system for normal Linux distributions. It aims for strong supervision, dependency-based parallel boot, clear diagnostics, reliable shutdown, and practical desktop/server usability while keeping PID 1 small and understandable.

The project is not a generic Unix init. The normal operating mode is modern Linux with cgroups v2 available and writable. A degraded rescue/initramfs mode exists only to boot a minimal environment, start a shell or getty, reap children, and shut down cleanly when the full cgroups v2 environment is not available.

## 2. Non-Goals

- Do not build a broad portability layer for non-Linux systems.
- Do not support cgroups v1.
- Do not make degraded rescue mode a daily-driver mode.
- Do not put persistent logging, timers, user sessions, network policy, or device policy inside PID 1.
- Do not clone systemd unit syntax or systemd's full PID 1 feature surface.
- Do not claim distro-readiness before VM boot, shutdown, service supervision, cgroup cleanup, and common service profiles are proven.

## 3. Repository Layout

The repository should use a Rust workspace with these long-term boundaries:

```text
minit/
  Cargo.toml
  README.md
  docs/
    design/
    specs/
    threat-model/
    superpowers/
      specs/
      plans/
  crates/
    minit-core/
    minitd/
    minitctl/
    minit-logd/
    minit-timed/
    minit-userd/
    minit-devd/
    minit-netd/
    minit-testkit/
  config/
    examples/
    targets/
    services/
  packaging/
    arch/
    alpine/
    gentoo/
  tests/
    integration/
    vm/
    fixtures/
  tools/
    vm/
    convert-systemd/
    graph/
```

Each crate must have a narrow purpose. Shared models and logic belong in `minit-core`; Linux PID 1 behavior belongs in `minitd`; user-facing commands belong in `minitctl`; optional background services live in their own crates.

## 4. Operating Modes

### 4.1 Normal Mode

Normal mode is the only daily-driver target.

Requirements:

- Linux kernel.
- cgroups v2 mounted and writable.
- `/proc`, `/sys`, `/dev`, and `/run` available or mountable by `minitd` early in boot.
- `minitd` runs as PID 1.
- Services run in minit-owned cgroups.
- Service cleanup, restart policy, resource control, and process ownership are based on cgroup membership.

If cgroups v2 setup fails in normal mode, `minitd` must refuse to continue as a daily-driver init and enter a clearly reported failure or rescue path.

### 4.2 Rescue/Initramfs Degraded Mode

Rescue mode is only for minimal boot and recovery.

Allowed behavior:

- mount `/proc`, `/sys`, `/dev`, and `/run` when possible;
- start a configured shell or getty;
- reap child processes correctly;
- handle halt, reboot, and poweroff cleanly;
- expose enough diagnostics to explain why normal mode was not entered.

Unavailable behavior:

- no cgroup-backed service ownership;
- no reliable cleanup of multi-process services beyond best-effort process group/session signaling;
- no resource limits through cgroups;
- no normal dependency graph boot;
- no daily-driver service supervision guarantees;
- no claim that common desktop/server service stacks are supported.

Rescue mode must be intentionally small. It must not become a second service manager.

## 5. Component Architecture

### 5.1 `minitd`

`minitd` is the PID 1 daemon. It is responsible for:

- early filesystem setup required for init operation;
- selecting normal or rescue mode;
- loading and validating unit definitions;
- dependency graph planning;
- service activation and shutdown orchestration;
- cgroups v2 setup and service cgroup lifecycle;
- child process spawning and reaping;
- signal handling;
- restart policy enforcement;
- bounded in-memory diagnostic state;
- IPC with `minitctl`;
- clean halt, reboot, and poweroff.

`minitd` must not own:

- persistent log storage;
- timer scheduling beyond immediate service restart delays;
- user session management;
- network configuration;
- device policy;
- graphical/session policy;
- package-manager integration;
- broad unit conversion logic.

### 5.2 `minit-core`

`minit-core` is a shared Rust crate for logic that can be tested outside PID 1:

- TOML service file schemas;
- parsed unit models;
- validation errors;
- dependency graph construction;
- cycle detection;
- boot plan generation;
- state machine types;
- restart policy evaluation;
- cgroup path naming rules;
- IPC request/response schemas;
- diagnostic and timeline event types.

`minit-core` must avoid direct PID 1 side effects. It may contain Linux-specific data models, but process creation, cgroup writes, signal delivery, and filesystem mounting belong outside it.

### 5.3 `minitctl`

`minitctl` is the primary operator CLI. It communicates with `minitd` over a Unix domain socket, expected under `/run/minit/minitd.sock`.

Initial commands:

```text
minitctl status
minitctl status <unit>
minitctl list-units
minitctl start <unit>
minitctl stop <unit>
minitctl restart <unit>
minitctl reload <unit>
minitctl explain <unit>
minitctl graph <target>
minitctl boot-timeline
minitctl daemon-reload
minitctl poweroff
minitctl reboot
```

Logging command:

```text
minitctl logs <service>
```

`minitctl logs` must query `minit-logd` when available. If `minit-logd` is unavailable, it may show the bounded recent buffer kept by `minitd` and must clearly report that persistent logs are unavailable.

### 5.4 Optional Helper Daemons

Optional helpers communicate with `minitd` through explicit APIs and must fail independently where possible.

- `minit-logd`: persistent logging, indexing, rotation, log querying, export.
- `minit-timed`: timer and cron-style activation; requests service starts through `minitd`.
- `minit-userd`: per-user service/session manager; not part of PID 1.
- `minit-devd`: device event bridge; integrates with udev/eudev/mdev-style sources without making PID 1 a device manager.
- `minit-netd`: network readiness coordination helper; not a full network manager.
- `minit-testkit`: test utilities for service graphs, process supervision, cgroup assertions, and VM boot tests.

## 6. Unit Model

Initial unit kinds:

- `service`
- `target`
- `mount`
- `swap`

Possible later unit kinds:

- `timer`
- `socket`
- `path`
- `user-service`

Later unit kinds should be added only when the responsible helper daemon and PID 1 boundary are clear.

## 7. TOML Service Format

Service files are human-readable TOML. The v1 format should be smaller and stricter than systemd's unit model.

Example:

```toml
[unit]
name = "sshd"
description = "OpenSSH daemon"
kind = "service"

[exec]
start = ["/usr/bin/sshd", "-D"]
reload = ["/bin/kill", "-HUP", "$MAINPID"]
stop = ["/bin/kill", "TERM", "$MAINPID"]
working_directory = "/"

[dependencies]
after = ["network-online.target"]
before = []
requires = ["network.target"]
wants = []
conflicts = []

[restart]
policy = "on-failure"
limit = "5/min"
backoff = "exponential"
max_delay = "5min"

[cgroup]
cpu_weight = 100
memory_max = "512M"
pids_max = 512

[security]
user = "root"
group = "root"
no_new_privileges = true
private_tmp = true
readonly_paths = ["/usr"]
readwrite_paths = ["/var/lib/sshd"]
environment = ["RUST_LOG=info"]
```

Validation requirements:

- command paths must be absolute unless explicitly resolved by a documented search policy;
- dependency references must resolve or produce actionable errors;
- restart limits must parse into bounded rate-limit rules;
- resource settings must be validated before service start;
- security settings unsupported by the current build or kernel must fail closed unless marked as explicitly optional;
- unknown keys must be warnings in early development and should become configurable strict errors before distro packaging.

## 8. Dependency Model

Dependency semantics:

- `after`: ordering only; does not pull in the referenced unit.
- `before`: inverse ordering only; does not pull in the referenced unit.
- `requires`: hard dependency; required unit failure prevents dependent startup or causes dependent stop according to the active job policy.
- `wants`: soft dependency; referenced unit is started when possible, but failure does not fail the dependent.
- `conflicts`: units cannot be active together.
- `target`: grouping and boot milestone.

Graph requirements:

- detect cycles before activation;
- explain cycles in operator-readable form;
- support dry-run job planning;
- support deterministic ordering for otherwise equal jobs;
- support parallel activation when ordering and requirement constraints allow it;
- record why a unit was started, skipped, blocked, failed, or stopped.

`minitctl explain <unit>` must expose the dependency reason chain, current blockers, recent failures, restart state, and cgroup/process summary.

`minitctl graph <target>` must emit a machine-readable graph format first, with optional pretty output later.

## 9. Process Supervision

In normal mode, each service must be supervised through its own cgroup.

Service start flow:

1. Validate service configuration.
2. Create service cgroup.
3. Prepare stdio handling.
4. Prepare UID/GID, working directory, environment, rlimits, and security settings.
5. Spawn process.
6. Place process into the service cgroup as early and reliably as possible.
7. Track main PID and cgroup membership.
8. Record state transition and timeline event.

Service stop flow:

1. Mark service stopping.
2. Run configured `exec.stop` command when present.
3. Send `SIGTERM` to the service cgroup.
4. Wait for configured or default grace period.
5. Send `SIGKILL` to remaining service cgroup processes.
6. Remove the cgroup when empty.
7. Record final state and diagnostics.

Child reaping requirements:

- `minitd` must continuously reap children.
- exits must be correlated with owning services where possible.
- unknown or orphaned child exits must be logged in bounded diagnostics.
- PID reuse must not corrupt service state.

Restart policies:

- `never`
- `on-failure`
- `always`
- `on-abnormal`

Restart behavior:

- classify exit status and signal termination;
- enforce rate limits such as `5/min`;
- support fixed and exponential backoff;
- cap exponential backoff using `max_delay`;
- expose throttled state through `minitctl status` and `minitctl explain`;
- prevent restart storms from pinning CPU or blocking boot forever.

## 10. cgroups v2 Integration

`minitd` owns a subtree under `/sys/fs/cgroup`, with an initial shape like:

```text
/sys/fs/cgroup/minit/
  init.scope/
  system.slice/
    sshd.service/
    dbus.service/
  user.slice/
```

Rules:

- normal mode requires cgroups v2;
- service identity is cgroup identity, not process name matching;
- cleanup is cgroup-based;
- resource control is declarative;
- cgroup setup failure is fatal in normal mode unless explicitly entering rescue mode;
- no cgroups v1 compatibility layer exists;
- cgroup paths must be deterministic and escaped safely.

Initial resource controls:

- `cpu.weight`
- `memory.max`
- `pids.max`
- process count reporting

Later resource controls may include IO and cpuset support after the core boot and supervision path is stable.

## 11. Logging Model

PID 1 must not become a persistent journal.

Initial behavior:

- `minitd` captures service stdout/stderr through pipes or configured stdio policy.
- `minitd` forwards logs to `minit-logd` when available.
- `minitd` keeps only bounded recent in-memory diagnostics needed for `status`, `explain`, and early boot troubleshooting.
- `minit-logd` owns persistent storage, indexing, rotation, search, export, and retention.

Failure behavior:

- services must not fail solely because `minit-logd` is unavailable unless their unit explicitly requires persistent logging;
- log backpressure must not deadlock PID 1;
- dropped logs must be counted and surfaced.

## 12. Shutdown and Reboot

Shutdown must be graph-aware and data-loss-resistant.

Flow:

1. Reject new start jobs.
2. Record shutdown intent and deadline.
3. Compute reverse dependency stop order.
4. Stop regular services.
5. Stop logging late enough to capture shutdown diagnostics.
6. Stop network and device helpers after dependents.
7. Deactivate swap units.
8. Unmount filesystems in safe order.
9. Escalate remaining service cgroups from `SIGTERM` to `SIGKILL`.
10. Sync filesystems.
11. Call the requested reboot, poweroff, or halt syscall.

Shutdown requirements:

- shutdown must make progress even when services ignore signals;
- stuck stop commands must be time bounded;
- remaining processes must be reported before final kill where possible;
- rescue mode shutdown must still reap and terminate direct children cleanly.

## 13. Security Model

The first security goal is predictable least surprise, not maximum sandbox breadth.

Initial controls:

- run as configured user/group;
- supplementary group handling;
- rlimits;
- environment filtering;
- working directory;
- umask;
- `no_new_privileges`;
- basic cgroup resource limits.

Near-term controls:

- capability bounding;
- private tmp through mount namespaces;
- readonly/readwrite path policy through mount namespaces;
- restricted device access where practical;
- seccomp profiles after service execution is stable.

Security requirements:

- invalid security configuration fails closed;
- unsupported security configuration is reported clearly;
- `minitd` should avoid parsing untrusted complex formats in PID 1;
- helper daemons must not gain implicit authority merely because they are part of the project;
- IPC commands that mutate system state require root or a documented authorization mechanism added later.

## 14. Diagnostics and Operator UX

`minit` must be easy to debug from a broken boot or degraded system.

Required diagnostics:

- `minitctl status`: system state, boot target, failed units, degraded/rescue flag.
- `minitctl status <unit>`: unit state, main PID, cgroup, restart state, recent exits.
- `minitctl explain <unit>`: why the unit is in its current state.
- `minitctl graph <target>`: dependency graph for the target.
- `minitctl boot-timeline`: ordered boot events and durations.
- `minitctl logs <service>`: recent or persistent logs depending on `minit-logd`.

Error messages must name the unit, file path, field, failing value, and suggested fix when known.

## 15. Compatibility Path

Compatibility is a tool-layer concern, not a PID 1 concern.

Later tools may convert basic units from:

- systemd service files;
- OpenRC scripts;
- simple runit/s6 service directories;
- dinit service files.

Conversion tools must mark unsupported features explicitly and should prefer partially converted files with review comments over silent behavior changes.

The v1 implementation does not need to run systemd units directly.

## 16. Testing Strategy

### 16.1 Unit Tests

Required coverage:

- TOML parsing;
- validation errors;
- dependency graph construction;
- cycle detection;
- deterministic job ordering;
- restart policy and backoff;
- service state machine transitions;
- cgroup path escaping;
- IPC schema round trips.

### 16.2 Integration Tests

Required coverage:

- spawn and reap behavior;
- service start/stop/restart;
- failed service throttling;
- process cleanup;
- signal handling;
- IPC command behavior;
- log forwarding fallback behavior.

### 16.3 VM Tests

Required coverage:

- rescue/initramfs boot starts a shell or getty;
- rescue/initramfs boot shuts down cleanly;
- normal mode boots with cgroups v2;
- getty/login path works;
- DBus can run as a supervised service;
- networking service can run as a supervised service;
- sshd can run as a supervised service;
- display manager can be attempted in a daily-driver profile;
- broken services do not wedge boot;
- shutdown kills leaked child processes;
- repeated boot/shutdown loops do not leak persistent state.

### 16.4 Hardening Tests

Required before experimental distro packaging:

- fuzz service parser;
- fuzz dependency graph inputs;
- chaos-test crashing services;
- test restart storms;
- test shutdown under stuck processes;
- check PID 1 file descriptor leaks;
- check PID 1 memory growth across boot loops;
- perform a security review of PID 1 attack surface and helper daemon boundaries.

## 17. Daily-Driver Readiness Path

Daily-driver readiness is reached in stages:

1. VM boots to rescue shell with `minitd` as PID 1.
2. VM boots normal mode with cgroups v2.
3. Basic service lifecycle works.
4. Dependency graph and targets work.
5. Parallel boot works.
6. cgroup cleanup and restart throttling work.
7. getty/login works.
8. DBus works.
9. networking works.
10. sshd works.
11. mount/swap units and clean shutdown work.
12. desktop display manager profile works in VM.
13. boot timeline and explain diagnostics are useful for failures.
14. hardening tests pass.
15. experimental packaging exists for at least one rolling distro.

No release should be described as daily-driver-ready until steps 1 through 14 are routinely passing in automated or documented manual VM validation.

## 18. Comparison With Existing Init Systems

### systemd

`minit` should compete with systemd on reliability, cgroup-backed supervision, dependency handling, boot quality, shutdown behavior, and diagnostics. It should not copy systemd's broad PID 1 responsibility set. Persistent logging, timers, user sessions, networking, and device policy remain outside PID 1.

### OpenRC

`minit` should preserve the readability and modularity people like in OpenRC while improving process ownership, restart behavior, cgroup cleanup, parallel graph planning, and structured diagnostics.

### runit

`minit` should learn from runit's simple supervision model, but it needs stronger dependency, target, cgroup, and distro boot orchestration for normal desktop/server systems.

### s6

`minit` should respect s6's small-tools philosophy and robustness, while presenting a more approachable default UX and TOML service model for distro users.

### dinit

dinit is the closest philosophical peer. `minit` differs by making Linux+cgroups v2 the hard normal-mode baseline and by emphasizing debug commands, conversion tooling, and daily-driver distro validation from the beginning.

## 19. Phased Delivery Roadmap

### Phase 1: Design and Workspace

Create the Rust workspace, crate boundaries, documentation structure, coding conventions, and initial CI/test commands. No runtime claims yet.

### Phase 2: Minimal PID 1 VM Boot

Implement `minitd` as PID 1 in a minimal VM/initramfs. Mount `/proc`, `/sys`, `/dev`, and `/run`; start a shell or getty; reap children; handle halt, reboot, and poweroff.

### Phase 3: Service Parser and Basic Lifecycle

Implement TOML parsing, validation, and a foreground service lifecycle for one service without the full dependency engine.

### Phase 4: Basic `minitctl`

Implement Unix socket IPC and `status`, `start`, `stop`, and `restart`.

### Phase 5: Dependency Engine and Targets

Implement `requires`, `wants`, `after`, `before`, `conflicts`, target activation, dry-run planning, and deterministic graph behavior.

### Phase 6: Parallel Boot and Explanations

Implement parallel job execution where dependency constraints allow it. Add cycle explanations, `minitctl explain`, `minitctl graph`, and boot timeline events.

### Phase 7: cgroups v2 Supervision

Implement service cgroups, cgroup cleanup, resource controls, ownership reporting, signal escalation, and normal-mode cgroup failure behavior.

### Phase 8: Restart Policies

Implement `never`, `on-failure`, `always`, `on-abnormal`, rate limiting, fixed backoff, exponential backoff, and throttled state reporting.

### Phase 9: Logging and `minit-logd`

Implement stdout/stderr capture, bounded PID 1 buffers, external log forwarding, persistent `minit-logd` storage, and `minitctl logs`.

### Phase 10: Mount/Swap Units and Shutdown Ordering

Implement mount units, swap units, reverse shutdown ordering, unmount/deactivate flow, and robust final signal escalation.

### Phase 11: Daily-Driver VM Profile

Build VM service profiles for getty/login, DBus, networking, sshd, and a display manager. Validate boot, status, logs, restart, and shutdown behavior.

### Phase 12: Hardening

Add fuzzing, chaos tests, boot/shutdown loop tests, memory/fd leak checks, restart storm tests, and security review findings.

### Phase 13: Compatibility Tools

Add basic conversion tools for simple systemd/OpenRC/runit/s6/dinit service definitions. Unsupported features must be explicit.

### Phase 14: Experimental Packaging

Create experimental packages for Arch, Alpine, and Gentoo after VM validation is credible. Packages must include rollback/recovery guidance.

## 20. Risks and Hard Problems

The hard parts are:

- PID 1 correctness during unusual process and signal states;
- avoiding PID reuse bugs;
- cgroups v2 setup differences across distros;
- avoiding boot deadlocks from dependency mistakes;
- preventing restart storms;
- shutdown ordering without data loss;
- making useful diagnostics when boot is partially broken;
- keeping helper daemon boundaries clean;
- supporting DBus, networking, login, and display managers without absorbing their responsibilities;
- packaging in a way that does not leave users with an unbootable system.

The project should bias toward fewer features with strong tests over broad feature claims.

## 21. Approval Boundary

This spec authorizes design and implementation planning only. It does not authorize writing runtime code yet.

After this spec is approved, the next step is a separate detailed implementation plan using a subagent-driven phase-by-phase workflow.
