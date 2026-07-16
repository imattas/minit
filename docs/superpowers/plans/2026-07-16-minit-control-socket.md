# minit Control Socket Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `minitd` expose its existing control protocol through a real normal-mode Unix socket so `minitctl` can talk to a running daemon.

**Architecture:** Keep protocol request/response handling in `crates/minitd/src/control.rs`. Add a small stateful control service around `minit_core::manager::ServiceManager`, then add a Linux-only Unix socket listener that delegates each accepted stream to the existing buffered I/O adapter. Normal-mode entry wiring stays explicit so rescue boot behavior remains unchanged.

**Tech Stack:** Rust 2021, `std::os::unix::net::UnixListener`, existing `minit-core` IPC types, Cargo tests, Linux musl target check.

## Global Constraints

- Project name: `minit`.
- PID 1 daemon name: `minitd`.
- Normal mode is Linux-only with cgroups v2 as a hard baseline.
- Rescue/initramfs mode remains degraded boot only.
- PID 1 stays small.
- Do not put persistent logging, timers, user sessions, network policy, or device policy inside PID 1.
- Every behavior-bearing Rust task must use TDD.
- Host builds on Windows must not require Linux-only syscalls.

---

### Task 1: Stateful Control Dispatch

**Files:**
- Modify: `crates/minitd/src/control.rs`

**Interfaces:**
- Consumes: `minit_core::manager::ServiceManager`.
- Produces: `minitd::control::ControlService`.
- Produces: `ControlService::new(services: ServiceManager) -> Self`.
- Produces: `ControlService::handle_request(&mut self, request: ControlRequest) -> ControlResponse`.

- [ ] **Step 1: Write the failing test**

Add tests proving `ControlService` returns registered unit status and maps unknown lifecycle requests to protocol errors.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p minitd control_service`

Expected: FAIL because `ControlService` does not exist.

- [ ] **Step 3: Write minimal implementation**

Add `ControlService { services: ServiceManager }`, delegate status to `services.status`, and keep start/stop/restart as explicit not-implemented errors.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p minitd control`

Expected: PASS.

- [ ] **Step 5: Commit**

Commit with `feat: add stateful minitd control service`.

### Task 2: Linux Unix Socket Listener

**Files:**
- Modify: `crates/minitd/src/control.rs`

**Interfaces:**
- Produces: Linux-only `minitd::control::ControlSocketConfig`.
- Produces: Linux-only `minitd::control::run_control_socket_once(config: &ControlSocketConfig, service: &mut ControlService) -> Result<(), ControlError>`.

- [ ] **Step 1: Write the failing host-safe test**

Add a test for `ControlSocketConfig::default()` exposing `/run/minit/minitd.sock`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p minitd control_socket_config_uses_default_socket`

Expected: FAIL because `ControlSocketConfig` does not exist.

- [ ] **Step 3: Write minimal implementation**

Add the config type for all platforms and the real Unix listener behind `cfg(target_os = "linux")`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p minitd control`

Expected: PASS.

- [ ] **Step 5: Commit**

Commit with `feat: add minitd control socket listener`.

### Task 3: Normal-Mode Entry Wiring

**Files:**
- Modify: `crates/minitd/src/lib.rs`

**Interfaces:**
- Produces: `minitd::should_enter_normal(args, is_pid_one, kernel_cmdline) -> bool`.
- Produces: `minitd::run_normal_entrypoint() -> i32`.

- [ ] **Step 1: Write the failing test**

Add a test proving `--normal` suppresses the PID-1 rescue fallback, while rescue flags still win.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p minitd normal_flag_selects_normal_mode`

Expected: FAIL because the normal-mode selector does not exist.

- [ ] **Step 3: Write minimal implementation**

Add selector helpers and make `run_with_args` call normal mode when rescue is not selected. On non-Linux hosts, `run_normal_entrypoint` returns `0` for smoke tests. On Linux, it starts an empty `ControlService` and one-shot control socket listener.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p minitd`

Expected: PASS.

- [ ] **Step 5: Commit**

Commit with `feat: wire minitd normal control entrypoint`.

### Task 4: Verification

**Files:**
- Modify only if verification reveals issues.

- [ ] Run `cargo test`.
- [ ] Run `cargo build -p minitd --target x86_64-unknown-linux-musl`.
- [ ] Run `cargo build -p minitctl --target x86_64-unknown-linux-musl`.
- [ ] Commit fixes only if required.
