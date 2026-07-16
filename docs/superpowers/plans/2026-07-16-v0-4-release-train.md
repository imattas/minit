# v0.4 Release Train Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `v0.4.0-experimental` release train with better release automation, useful log following, real bounded parallel boot starts, stronger hardening proof, broader distro validation, recovery hardening, conversion helpers, and a tagged GitHub pre-release.

**Architecture:** Keep wire-compatible operator features in `minit-core` IPC types, daemon state and service execution in `minitd`, CLI formatting and polling in `minitctl`, distro/release proof in PowerShell VM gates, and docs in top-level release files. Each task must be independently tested and pushed before the next phase.

**Tech Stack:** Rust workspace, clap CLI, serde JSON IPC, Linux cgroups v2, PowerShell QEMU harness, GitHub CLI and Actions.

## Global Constraints

- Normal mode requires Linux with cgroups v2.
- Release claims must match what `tools\verify-release.ps1` actually proves.
- Keep unsupported sandbox options fail-closed unless implemented and VM-proven.
- Preserve deterministic target dependency ordering even when starts run concurrently.
- Do not tag `v0.4.0-experimental` until source, security, and full VM gates pass.

---

### Task 1: Release Workflow Cleanup

**Files:**
- Modify: `.github/workflows/release.yml`
- Modify: `docs/superpowers/plans/2026-07-16-v0-4-release-train.md`

**Interfaces:**
- Produces GitHub releases with `--prerelease`.
- Uses `docs/releases/<tag>.md` as release notes when the file exists.

- [x] Add workflow shell logic that sets `NOTES_FILE=docs/releases/$GITHUB_REF_NAME.md`.
- [x] Create releases with `--notes-file "$NOTES_FILE"` when the notes file exists.
- [x] Fall back to `--generate-notes` only when no checked-in notes file exists.
- [x] Always pass `--prerelease` for experimental tags.
- [x] Run a YAML/script sanity check and commit.

### Task 2: Persistent Logs and Follow Mode

**Files:**
- Modify: `crates/minit-core/src/ipc.rs`
- Modify: `crates/minitctl/src/lib.rs`
- Modify: `crates/minitd/src/control.rs`
- Potentially modify: `crates/minitd/src/runtime.rs`
- Modify: `tools/vm/run-minit-qemu.ps1`
- Modify: `tools/verify-release.ps1`

**Interfaces:**
- Produces `minitctl logs --follow <unit>`.
- Extends `ControlRequest::Logs { unit, follow }`.
- Keeps existing one-shot `minitctl logs <unit>` behavior.

- [x] Add failing IPC test for `Logs { unit, follow: true }`.
- [x] Add failing CLI parse test for `logs --follow demo-sleep`.
- [x] Add render/transport test proving follow mode polls without changing one-shot output.
- [x] Add bounded persistent log file storage under `/run/minit/logs` for lifecycle lines first.
- [x] Capture service stdout/stderr only after lifecycle file storage is stable.
- [x] Add VM smoke for one-shot persisted lifecycle logs.
- [x] Add VM smoke for bounded follow output with timeout.
- [x] Run focused tests, then full release gate, then commit and push.

### Task 3: Bounded Parallel Target Starts

**Files:**
- Modify: `crates/minitd/src/control.rs`
- Modify: `crates/minitd/src/runtime.rs`
- Modify: `tools/vm/run-minit-qemu.ps1`
- Modify: `tools/verify-release.ps1`

**Interfaces:**
- Keeps target start response text stable.
- Adds bounded concurrent start execution for independent service units in one graph batch.

- [x] Add failing tests proving concurrency limit is honored.
- [x] Add failing tests proving required dependency failure remains deterministic.
- [x] Split plan/start from state mutation if needed so runtime can start independent processes safely.
- [x] Implement conservative bounded worker count with stable result ordering.
- [x] Add VM proof with two independent services in one batch starting through the real target path.
- [x] Run focused tests, then full release gate, then commit and push.

### Task 4: Seccomp and Sandbox Proof

**Files:**
- Modify: `crates/minit-core/src/unit.rs`
- Modify: `crates/minitd/src/runtime.rs`
- Add: `config/examples/seccomp-deny-write.service.toml`
- Modify: `tools/vm/run-minit-qemu.ps1`
- Modify: `tools/verify-release.ps1`
- Modify: `docs/security-review.md`

**Interfaces:**
- Adds a minimal seccomp profile option when supported.
- Keeps unsupported security features fail-closed.

- [x] Add failing parser tests for a minimal deny-write seccomp profile.
- [x] Add failing runtime tests for unsupported seccomp configuration off Linux.
- [x] Implement Linux seccomp setup only where kernel/runtime support exists.
- [x] Add VM service proving denied syscall behavior.
- [x] Update security docs with exact supported and unsupported options.
- [x] Run focused tests, then full release gate, then commit and push.

### Task 5: Distro Rootfs Validation Expansion

**Files:**
- Add: `tools/vm/verify-debian-minirootfs.ps1`
- Add: `tools/vm/verify-arch-rootfs.ps1`
- Modify: `docs/profiles.md`
- Modify: `docs/daily-driver-candidate.md`

**Interfaces:**
- Produces disposable Debian and Arch rootfs gates when local rootfs inputs are available.
- Does not make release gate depend on large downloads unless explicitly provided.

- [ ] Add scripts that validate required inputs and fail clearly when missing.
- [ ] Reuse existing profile initramfs layout where possible.
- [ ] Add smoke commands for status, list, and shutdown.
- [ ] Document exact input requirements and unsupported distro services.
- [ ] Run available local distro gates, then commit and push.

### Task 6: Recovery Mode Hardening

**Files:**
- Modify: `crates/minitd/src/lib.rs`
- Modify: `docs/install.md`
- Modify: `docs/daily-driver-candidate.md`
- Modify: `tools/vm/run-minit-qemu.ps1`
- Modify: `tools/verify-release.ps1`

**Interfaces:**
- Produces clearer failed-target recovery behavior.
- Documents rollback-safe install/uninstall path.

- [ ] Add failing tests for failed boot target falling back to rescue behavior.
- [ ] Add VM smoke for failed boot target recovery.
- [ ] Add docs for rollback and emergency shell access.
- [ ] Run focused tests, then full release gate, then commit and push.

### Task 7: Unit Conversion Helpers

**Files:**
- Add: `crates/minitctl/src/convert.rs`
- Modify: `crates/minitctl/src/lib.rs`
- Modify: `docs/unit-conversion.md`

**Interfaces:**
- Produces `minitctl convert --from systemd <path>`.
- Produces explicit warnings for unsupported fields.

- [ ] Add failing parse tests for simple systemd service conversion.
- [ ] Add failing tests for unsupported systemd fields producing warnings.
- [ ] Add OpenRC/runit/s6 skeleton detection with explicit unsupported warnings.
- [ ] Keep generated TOML reviewable and conservative.
- [ ] Document examples and limitations.
- [ ] Run focused tests, then full release gate, then commit and push.

### Task 8: v0.4 Release

**Files:**
- Create: `docs/releases/v0.4.0-experimental.md`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-07-16-v0-4-release-train.md`

**Interfaces:**
- Produces Git tag `v0.4.0-experimental`.
- Produces GitHub pre-release with checked-in notes.

- [ ] Update version and release notes.
- [ ] Run `cargo fmt --check`, `cargo test`, `tools\verify-security.ps1`, and full VM `tools\verify-release.ps1`.
- [ ] Commit release prep.
- [ ] Tag and push.
- [ ] Verify GitHub release workflow, CI, and Security.
