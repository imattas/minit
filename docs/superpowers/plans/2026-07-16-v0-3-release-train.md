# v0.3 Release Train Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the next experimental release train with VM-proven boot graph behavior, richer operator commands, hardening proof, and a tagged `v0.3.0-experimental` release.

**Architecture:** Keep shared wire models and state summaries in `minit-core`, daemon behavior in `minitd`, operator formatting in `minitctl`, and release proof in PowerShell VM gates. Every feature lands with a failing test first, then minimal implementation, then full release verification before push or tag.

**Tech Stack:** Rust workspace, serde JSON IPC, clap CLI, PowerShell QEMU harness, WSL cpio initramfs builder, GitHub CLI.

## Global Constraints

- Normal mode requires Linux with cgroups v2.
- The release gate is `tools\verify-release.ps1`.
- Windows local fuzz smokes may skip unless LLVM ASAN runtime is installed; dependency audit still runs.
- Release claims must match what the release gate actually proves.
- Do not tag `v0.3.0-experimental` until full source, security, and VM gates pass.

---

### Task 1: VM Dependency Failure Smokes

**Files:**
- Modify: `crates/minitd/src/lib.rs`
- Modify: `tools/vm/run-minit-qemu.ps1`
- Modify: `tools/verify-release.ps1`
- Create: `config/examples/wanted-failure.target.toml`
- Create: `config/examples/required-failure.target.toml`
- Create: `config/examples/optional-fail.service.toml`
- Create: `config/examples/required-fail.service.toml`

**Interfaces:**
- Produces kernel args `minit.smoke_wanted_failure=<target>` and `minit.smoke_required_failure=<target>`.
- Produces VM harness flags `-ExpectWantedFailureTarget` and `-ExpectRequiredFailureTarget`.

- [x] Add failing config parsing tests for both new smoke args.
- [x] Add failing QEMU harness expectations for wanted and required target failure behavior.
- [x] Add example units that intentionally fail.
- [x] Wire startup commands that start the target and query target plus failed service status.
- [x] Add both VM smokes to `tools\verify-release.ps1`.
- [x] Run focused tests, then full release gate.

### Task 2: Machine-Readable Graph Output

**Files:**
- Modify: `crates/minitctl/src/lib.rs`

**Interfaces:**
- Produces `minitctl graph --json <unit>`.

- [x] Add failing CLI parse test for `graph --json multi-user.target`.
- [x] Add failing render test expecting JSON with `unit` and `batches`.
- [x] Implement output mode on graph command only.
- [x] Run `cargo test -p minitctl graph`.

### Task 3: Boot Timeline Command

**Files:**
- Modify: `crates/minit-core/src/ipc.rs`
- Modify: `crates/minitd/src/control.rs`
- Modify: `crates/minitctl/src/lib.rs`
- Modify: `crates/minitd/src/lib.rs`

**Interfaces:**
- Produces `ControlRequest::BootTimeline`.
- Produces `ControlResponse::BootTimeline { events: Vec<DiagnosticEvent> }`.
- Produces `minitctl boot-timeline`.

- [x] Add failing IPC round-trip test.
- [x] Add failing CLI parse/render tests.
- [x] Record boot timeline events in daemon event buffer.
- [x] Implement daemon response.
- [x] Add a VM smoke if output is stable enough for release gate.

### Task 4: Explain Runtime Context

**Files:**
- Modify: `crates/minit-core/src/manager.rs`
- Modify: `crates/minitd/src/control.rs`
- Modify: `crates/minitctl/src/lib.rs`

**Interfaces:**
- Keeps `ControlResponse::Explanation { unit, lines }`.
- Adds lines for state, failed dependencies, last exit, restart attempts, and cgroup path where available.

- [x] Add failing tests for explain output on failed optional and required units.
- [x] Extend manager explain summaries from existing status and dependency data.
- [x] Keep output line-oriented for current CLI compatibility.

### Task 5: Recent Logs Command

**Files:**
- Modify: `crates/minit-core/src/ipc.rs`
- Modify: `crates/minitd/src/control.rs`
- Modify: `crates/minitctl/src/lib.rs`

**Interfaces:**
- Produces `ControlRequest::Logs { unit: String }`.
- Produces `ControlResponse::Logs { unit: String, lines: Vec<String> }`.
- Produces `minitctl logs <unit>`.

- [x] Add failing IPC and CLI tests.
- [x] Start with a bounded in-memory message buffer, not persistent logging.
- [x] Include lifecycle messages now; process stdout/stderr capture can follow.

### Task 6: Distro/Profile and Recovery Docs

**Files:**
- Modify: `docs/profiles.md`
- Modify: `docs/install.md`
- Modify: `docs/daily-driver-candidate.md`
- Modify or add profile TOML under `config/profiles`.

**Interfaces:**
- Documents current Alpine/minimal profile limits and rollback.

- [ ] Add tests if profile unit counts change.
- [ ] Keep unsupported services explicit.
- [ ] Do not claim full distro daily-driver readiness.

### Task 7: Hardening VM Proof

**Files:**
- Modify: `crates/minitd/src/lib.rs`
- Modify: `tools/vm/run-minit-qemu.ps1`
- Modify: `tools/verify-release.ps1`
- Add hardening smoke units under `config/examples`.

**Interfaces:**
- Produces VM smokes for control socket permissions, `no_new_privileges`, and UID/GID where feasible.

- [ ] Add failing config parsing tests.
- [ ] Add controlled VM checks that emit clear proof strings.
- [ ] Add release gate steps only for stable checks.

### Task 8: Bounded Parallel Batch Start

**Files:**
- Modify: `crates/minitd/src/control.rs`
- Potentially modify: `crates/minitd/src/runtime.rs`

**Interfaces:**
- Keeps target start external behavior.
- Produces deterministic batch ordering and bounded concurrent starts where runtime can safely support it.

- [ ] Add failing tests proving independent units in one batch are submitted through a batch interface.
- [ ] Add a conservative batch-start runtime hook with sequential default.
- [ ] Only use actual thread concurrency if service state mutation remains safe and deterministic.

### Task 9: v0.3 Release

**Files:**
- Create: `docs/releases/v0.3.0-experimental.md`
- Modify: `README.md`
- Modify: `Cargo.toml`
- Modify crate manifests if version is workspace-local.

**Interfaces:**
- Produces Git tag `v0.3.0-experimental`.
- Produces GitHub pre-release.

- [ ] Update version and release notes.
- [ ] Run `cargo fmt --check`, `cargo test`, `tools\verify-security.ps1`, and full VM `tools\verify-release.ps1`.
- [ ] Commit release prep.
- [ ] Tag and push.
- [ ] Verify GitHub release workflow.
