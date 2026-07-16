# minit Minimal Boot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first real `minit` milestone: a minimal Linux VM/initramfs can run `minitd` as PID 1, mount `/proc`, `/sys`, `/dev`, and `/run`, start a shell or getty, reap children, and shut down cleanly.

**Architecture:** Start with a small Rust workspace containing `minit-core`, `minitd`, and `minit-testkit`. Keep host-testable logic in `minit-core` and testable adapters in `minitd`; isolate Linux-only syscall code behind `cfg(target_os = "linux")` so development on Windows can still compile and test the shared crates. VM boot tooling lives under `tools/vm` and is allowed to skip when Linux VM dependencies are unavailable.

**Tech Stack:** Rust 2021, Cargo workspace resolver v2, `serde`, `toml`, `thiserror`, `nix` for Linux syscalls, `clap` later for CLI, PowerShell helpers for Windows host orchestration, QEMU/BusyBox/Linux kernel for VM verification when available.

## Global Constraints

- Project name: `minit`.
- PID 1 daemon name: `minitd`.
- Primary language: Rust.
- Target: normal Linux distros first, not a custom OS first.
- Normal mode: Linux-only with cgroups v2 as a hard baseline.
- Rescue/initramfs mode: degraded boot only, no daily-driver claims.
- PID 1 stays small.
- Optional daemons stay outside PID 1.
- Service files use readable TOML.
- Do not support cgroups v1.
- Do not build a non-Linux portability layer.
- Do not put persistent logging, timers, user sessions, network policy, or device policy inside PID 1.
- Every behavior-bearing Rust task must use TDD: write the failing test, verify it fails, implement, verify it passes.
- Host builds on Windows must not require Linux-only syscalls; Linux-only runtime code must be behind `cfg(target_os = "linux")`.
- Runtime PID 1 code must remain in `crates/minitd`; shared pure logic must remain in `crates/minit-core`.

---

## File Structure

- `Cargo.toml`: workspace manifest with all initial crate members and shared dependency versions.
- `README.md`: short project description, current milestone, and explicit non-ready warning.
- `.gitignore`: ignores build output, VM artifacts, and subagent scratch files.
- `crates/minit-core/Cargo.toml`: shared library manifest.
- `crates/minit-core/src/lib.rs`: exports shared modules.
- `crates/minit-core/src/boot.rs`: boot mode and rescue configuration models.
- `crates/minit-core/src/diagnostics.rs`: small diagnostic event model.
- `crates/minitd/Cargo.toml`: PID 1 daemon manifest.
- `crates/minitd/src/main.rs`: binary entrypoint.
- `crates/minitd/src/lib.rs`: exports testable modules.
- `crates/minitd/src/early_mounts.rs`: host-testable early mount planning and execution adapter.
- `crates/minitd/src/rescue.rs`: rescue command selection and run loop coordination.
- `crates/minitd/src/reaper.rs`: child reap event model and Linux wait-loop adapter.
- `crates/minitd/src/shutdown.rs`: shutdown action model and Linux syscall adapter.
- `crates/minit-testkit/Cargo.toml`: test helper crate manifest.
- `crates/minit-testkit/src/lib.rs`: fake syscall helpers used by `minitd` tests.
- `tools/vm/build-initramfs.ps1`: builds a minimal initramfs when required tools exist.
- `tools/vm/run-minit-qemu.ps1`: runs a QEMU smoke boot when required tools exist.
- `tests/vm/README.md`: explains VM prerequisites and the expected first milestone proof.

---

### Task 1: Rust Workspace Baseline

**Files:**
- Modify: `Cargo.toml`
- Modify: `README.md`
- Create: `.gitignore`
- Create: `crates/minit-core/Cargo.toml`
- Create: `crates/minit-core/src/lib.rs`
- Create: `crates/minitd/Cargo.toml`
- Create: `crates/minitd/src/main.rs`
- Create: `crates/minitd/src/lib.rs`
- Create: `crates/minit-testkit/Cargo.toml`
- Create: `crates/minit-testkit/src/lib.rs`

**Interfaces:**
- Produces: workspace crates `minit-core`, `minitd`, and `minit-testkit`.
- Produces: `minitd::version() -> &'static str`.
- Produces: `minit_core::PROJECT_NAME: &str`.
- Consumes: approved spec at `docs/superpowers/specs/2026-07-16-minit-design.md`.

- [ ] **Step 1: Write the baseline test**

Create `crates/minit-core/src/lib.rs` with:

```rust
pub const PROJECT_NAME: &str = "minit";

#[cfg(test)]
mod tests {
    use super::PROJECT_NAME;

    #[test]
    fn project_name_is_minit() {
        assert_eq!(PROJECT_NAME, "minit");
    }
}
```

Create `crates/minitd/src/lib.rs` with:

```rust
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_package_version() {
        assert_eq!(crate::version(), env!("CARGO_PKG_VERSION"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails because the workspace does not exist**

Run: `cargo test -p minit-core`

Expected: FAIL with a manifest or missing package error because the workspace manifests have not been created yet.

- [ ] **Step 3: Write workspace manifests and stubs**

Create root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
  "crates/minit-core",
  "crates/minitd",
  "crates/minit-testkit",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://example.invalid/minit"

[workspace.dependencies]
minit-core = { path = "crates/minit-core" }
minit-testkit = { path = "crates/minit-testkit" }
serde = { version = "1.0", features = ["derive"] }
thiserror = "2.0"
libc = "0.2"
nix = { version = "0.30", features = ["fs", "mount", "process", "signal", "user"] }
```

Create `crates/minit-core/Cargo.toml`:

```toml
[package]
name = "minit-core"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
serde.workspace = true
thiserror.workspace = true
```

Create `crates/minitd/Cargo.toml`:

```toml
[package]
name = "minitd"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
minit-core.workspace = true
thiserror.workspace = true

[target.'cfg(target_os = "linux")'.dependencies]
libc.workspace = true
nix.workspace = true

[dev-dependencies]
minit-testkit.workspace = true
```

Create `crates/minit-testkit/Cargo.toml`:

```toml
[package]
name = "minit-testkit"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
```

Create `crates/minitd/src/main.rs`:

```rust
fn main() {
    minitd::run();
}
```

Append to `crates/minitd/src/lib.rs`:

```rust
pub fn run() {
    println!("minitd {}", version());
}
```

Create `crates/minit-testkit/src/lib.rs`:

```rust
pub fn testkit_name() -> &'static str {
    "minit-testkit"
}
```

Create `.gitignore`:

```gitignore
/target/
/.superpowers/
/.worktrees/
/tests/vm/artifacts/
/tools/vm/artifacts/
*.log
*.img
*.cpio
*.cpio.gz
```

Write `README.md`:

```markdown
# minit

`minit` is a Rust Linux init and service manager experiment targeting modern normal Linux distributions.

Current milestone: minimal VM/initramfs boot with `minitd` as PID 1.

Normal mode will require Linux with cgroups v2. Rescue/initramfs mode is degraded and only intended to mount basic filesystems, start a shell or getty, reap children, and shut down cleanly.

This repository is not daily-driver-ready yet.
```

- [ ] **Step 4: Run tests**

Run: `cargo test`

Expected: PASS for `minit-core`, `minitd`, and `minit-testkit`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml README.md .gitignore crates/minit-core crates/minitd crates/minit-testkit
git commit -m "chore: scaffold minit rust workspace"
```

---

### Task 2: Core Boot and Diagnostic Models

**Files:**
- Modify: `crates/minit-core/src/lib.rs`
- Create: `crates/minit-core/src/boot.rs`
- Create: `crates/minit-core/src/diagnostics.rs`

**Interfaces:**
- Produces: `minit_core::boot::BootMode`.
- Produces: `minit_core::boot::RescueConfig`.
- Produces: `minit_core::boot::EarlyMount`.
- Produces: `minit_core::boot::default_early_mounts() -> Vec<EarlyMount>`.
- Produces: `minit_core::diagnostics::DiagnosticEvent`.

- [ ] **Step 1: Write failing tests for boot defaults**

Create `crates/minit-core/src/boot.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_early_mounts_cover_required_rescue_filesystems() {
        let mounts = default_early_mounts();
        let targets: Vec<&str> = mounts.iter().map(|mount| mount.target.as_str()).collect();

        assert_eq!(targets, vec!["/proc", "/sys", "/dev", "/run"]);
    }

    #[test]
    fn rescue_config_defaults_to_shell() {
        let config = RescueConfig::default();

        assert_eq!(config.command, vec!["/bin/sh"]);
        assert_eq!(config.mode, BootMode::Rescue);
    }
}
```

Create `crates/minit-core/src/diagnostics.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_event_formats_scope_and_message() {
        let event = DiagnosticEvent::new("boot", "mounted /proc");

        assert_eq!(event.scope, "boot");
        assert_eq!(event.message, "mounted /proc");
        assert_eq!(event.to_string(), "[boot] mounted /proc");
    }
}
```

- [ ] **Step 2: Run tests to verify red**

Run: `cargo test -p minit-core`

Expected: FAIL with unresolved types/functions such as `default_early_mounts`, `RescueConfig`, and `DiagnosticEvent`.

- [ ] **Step 3: Implement models**

Replace `crates/minit-core/src/boot.rs` with:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootMode {
    Normal,
    Rescue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RescueConfig {
    pub mode: BootMode,
    pub command: Vec<String>,
}

impl Default for RescueConfig {
    fn default() -> Self {
        Self {
            mode: BootMode::Rescue,
            command: vec!["/bin/sh".to_string()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EarlyMount {
    pub source: &'static str,
    pub target: &'static str,
    pub fstype: &'static str,
    pub flags: u64,
}

pub fn default_early_mounts() -> Vec<EarlyMount> {
    vec![
        EarlyMount { source: "proc", target: "/proc", fstype: "proc", flags: 0 },
        EarlyMount { source: "sysfs", target: "/sys", fstype: "sysfs", flags: 0 },
        EarlyMount { source: "devtmpfs", target: "/dev", fstype: "devtmpfs", flags: 0 },
        EarlyMount { source: "tmpfs", target: "/run", fstype: "tmpfs", flags: 0 },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_early_mounts_cover_required_rescue_filesystems() {
        let mounts = default_early_mounts();
        let targets: Vec<&str> = mounts.iter().map(|mount| mount.target).collect();

        assert_eq!(targets, vec!["/proc", "/sys", "/dev", "/run"]);
    }

    #[test]
    fn rescue_config_defaults_to_shell() {
        let config = RescueConfig::default();

        assert_eq!(config.command, vec!["/bin/sh"]);
        assert_eq!(config.mode, BootMode::Rescue);
    }
}
```

Replace `crates/minit-core/src/diagnostics.rs` with:

```rust
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticEvent {
    pub scope: String,
    pub message: String,
}

impl DiagnosticEvent {
    pub fn new(scope: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            scope: scope.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for DiagnosticEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "[{}] {}", self.scope, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_event_formats_scope_and_message() {
        let event = DiagnosticEvent::new("boot", "mounted /proc");

        assert_eq!(event.scope, "boot");
        assert_eq!(event.message, "mounted /proc");
        assert_eq!(event.to_string(), "[boot] mounted /proc");
    }
}
```

Update `crates/minit-core/src/lib.rs`:

```rust
pub mod boot;
pub mod diagnostics;

pub const PROJECT_NAME: &str = "minit";

#[cfg(test)]
mod tests {
    use super::PROJECT_NAME;

    #[test]
    fn project_name_is_minit() {
        assert_eq!(PROJECT_NAME, "minit");
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p minit-core`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/minit-core/src
git commit -m "feat: add boot and diagnostic core models"
```

---

### Task 3: Host-Testable Early Mount Execution

**Files:**
- Modify: `crates/minitd/src/lib.rs`
- Create: `crates/minitd/src/early_mounts.rs`
- Modify: `crates/minit-testkit/src/lib.rs`

**Interfaces:**
- Consumes: `minit_core::boot::default_early_mounts`.
- Produces: `minitd::early_mounts::MountExecutor` trait.
- Produces: `minitd::early_mounts::ensure_early_mounts<E: MountExecutor>(executor: &mut E) -> Result<Vec<DiagnosticEvent>, MountError>`.
- Produces: `minit_testkit::RecordingMountExecutor`.

- [ ] **Step 1: Write failing tests**

Create `crates/minitd/src/early_mounts.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use minit_core::boot::default_early_mounts;

    #[derive(Default)]
    struct FakeMountExecutor {
        calls: Vec<String>,
    }

    impl MountExecutor for FakeMountExecutor {
        fn ensure_dir(&mut self, path: &str) -> Result<(), MountError> {
            self.calls.push(format!("dir:{path}"));
            Ok(())
        }

        fn mount(&mut self, spec: &minit_core::boot::EarlyMount) -> Result<(), MountError> {
            self.calls.push(format!("mount:{}:{}", spec.fstype, spec.target));
            Ok(())
        }
    }

    #[test]
    fn ensure_early_mounts_creates_directories_before_mounting() {
        let mut executor = FakeMountExecutor::default();

        let events = ensure_early_mounts(&mut executor).expect("mounts should succeed");

        assert_eq!(executor.calls[0], "dir:/proc");
        assert_eq!(executor.calls[1], "mount:proc:/proc");
        assert_eq!(executor.calls.len(), default_early_mounts().len() * 2);
        assert!(events.iter().any(|event| event.message == "mounted /proc"));
    }
}
```

- [ ] **Step 2: Run test to verify red**

Run: `cargo test -p minitd early_mounts`

Expected: FAIL with unresolved `MountExecutor`, `MountError`, and `ensure_early_mounts`.

- [ ] **Step 3: Implement early mount abstraction**

Replace `crates/minitd/src/early_mounts.rs` with:

```rust
use minit_core::boot::{default_early_mounts, EarlyMount};
use minit_core::diagnostics::DiagnosticEvent;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MountError {
    #[error("failed to create mount point {path}: {message}")]
    CreateDir { path: String, message: String },
    #[error("failed to mount {target}: {message}")]
    Mount { target: String, message: String },
}

pub trait MountExecutor {
    fn ensure_dir(&mut self, path: &str) -> Result<(), MountError>;
    fn mount(&mut self, spec: &EarlyMount) -> Result<(), MountError>;
}

pub fn ensure_early_mounts<E: MountExecutor>(
    executor: &mut E,
) -> Result<Vec<DiagnosticEvent>, MountError> {
    let mut events = Vec::new();

    for spec in default_early_mounts() {
        executor.ensure_dir(spec.target)?;
        executor.mount(&spec)?;
        events.push(DiagnosticEvent::new("boot", format!("mounted {}", spec.target)));
    }

    Ok(events)
}

#[cfg(target_os = "linux")]
pub struct LinuxMountExecutor;

#[cfg(target_os = "linux")]
impl MountExecutor for LinuxMountExecutor {
    fn ensure_dir(&mut self, path: &str) -> Result<(), MountError> {
        std::fs::create_dir_all(path).map_err(|error| MountError::CreateDir {
            path: path.to_string(),
            message: error.to_string(),
        })
    }

    fn mount(&mut self, spec: &EarlyMount) -> Result<(), MountError> {
        use nix::mount::{mount, MsFlags};
        use std::ffi::OsStr;

        mount(
            Some(OsStr::new(spec.source)),
            spec.target,
            Some(OsStr::new(spec.fstype)),
            MsFlags::from_bits_truncate(spec.flags),
            None::<&OsStr>,
        )
        .map_err(|error| MountError::Mount {
            target: spec.target.to_string(),
            message: error.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minit_core::boot::default_early_mounts;

    #[derive(Default)]
    struct FakeMountExecutor {
        calls: Vec<String>,
    }

    impl MountExecutor for FakeMountExecutor {
        fn ensure_dir(&mut self, path: &str) -> Result<(), MountError> {
            self.calls.push(format!("dir:{path}"));
            Ok(())
        }

        fn mount(&mut self, spec: &EarlyMount) -> Result<(), MountError> {
            self.calls.push(format!("mount:{}:{}", spec.fstype, spec.target));
            Ok(())
        }
    }

    #[test]
    fn ensure_early_mounts_creates_directories_before_mounting() {
        let mut executor = FakeMountExecutor::default();

        let events = ensure_early_mounts(&mut executor).expect("mounts should succeed");

        assert_eq!(executor.calls[0], "dir:/proc");
        assert_eq!(executor.calls[1], "mount:proc:/proc");
        assert_eq!(executor.calls.len(), default_early_mounts().len() * 2);
        assert!(events.iter().any(|event| event.message == "mounted /proc"));
    }
}
```

Update `crates/minitd/src/lib.rs`:

```rust
pub mod early_mounts;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn run() {
    println!("minitd {}", version());
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_package_version() {
        assert_eq!(crate::version(), env!("CARGO_PKG_VERSION"));
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p minitd early_mounts`

Expected: PASS.

Run: `cargo test`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/minitd/src crates/minit-testkit/src
git commit -m "feat: add early mount execution adapter"
```

---

### Task 4: Rescue Command Selection and PID 1 Entry Flow

**Files:**
- Modify: `crates/minitd/src/lib.rs`
- Modify: `crates/minitd/src/main.rs`
- Create: `crates/minitd/src/rescue.rs`

**Interfaces:**
- Consumes: `minit_core::boot::RescueConfig`.
- Consumes: `minitd::early_mounts::ensure_early_mounts`.
- Produces: `minitd::rescue::RescueCommand`.
- Produces: `minitd::rescue::select_rescue_command(config: &RescueConfig, candidates: &[&str]) -> RescueCommand`.
- Produces: `minitd::run_with_args<I, S>(args: I) -> i32`.

- [ ] **Step 1: Write failing tests**

Create `crates/minitd/src/rescue.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use minit_core::boot::RescueConfig;

    #[test]
    fn selects_configured_shell_when_available() {
        let config = RescueConfig::default();

        let command = select_rescue_command(&config, &["/bin/sh"]);

        assert_eq!(command.argv, vec!["/bin/sh"]);
        assert!(!command.fallback_used);
    }

    #[test]
    fn falls_back_to_getty_when_shell_missing() {
        let config = RescueConfig::default();

        let command = select_rescue_command(&config, &["/sbin/getty"]);

        assert_eq!(command.argv, vec!["/sbin/getty", "console"]);
        assert!(command.fallback_used);
    }
}
```

- [ ] **Step 2: Run test to verify red**

Run: `cargo test -p minitd rescue`

Expected: FAIL with unresolved `RescueCommand` and `select_rescue_command`.

- [ ] **Step 3: Implement rescue selection and entry wiring**

Create `crates/minitd/src/rescue.rs`:

```rust
use minit_core::boot::RescueConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RescueCommand {
    pub argv: Vec<String>,
    pub fallback_used: bool,
}

pub fn select_rescue_command(config: &RescueConfig, candidates: &[&str]) -> RescueCommand {
    if let Some(program) = config.command.first() {
        if candidates.iter().any(|candidate| candidate == program) {
            return RescueCommand {
                argv: config.command.clone(),
                fallback_used: false,
            };
        }
    }

    if candidates.contains(&"/sbin/getty") {
        return RescueCommand {
            argv: vec!["/sbin/getty".to_string(), "console".to_string()],
            fallback_used: true,
        };
    }

    RescueCommand {
        argv: vec!["/bin/sh".to_string()],
        fallback_used: true,
    }
}

#[cfg(target_os = "linux")]
pub fn existing_rescue_candidates() -> Vec<&'static str> {
    ["/bin/sh", "/sbin/getty"]
        .into_iter()
        .filter(|path| std::path::Path::new(path).exists())
        .collect()
}

#[cfg(not(target_os = "linux"))]
pub fn existing_rescue_candidates() -> Vec<&'static str> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use minit_core::boot::RescueConfig;

    #[test]
    fn selects_configured_shell_when_available() {
        let config = RescueConfig::default();

        let command = select_rescue_command(&config, &["/bin/sh"]);

        assert_eq!(command.argv, vec!["/bin/sh"]);
        assert!(!command.fallback_used);
    }

    #[test]
    fn falls_back_to_getty_when_shell_missing() {
        let config = RescueConfig::default();

        let command = select_rescue_command(&config, &["/sbin/getty"]);

        assert_eq!(command.argv, vec!["/sbin/getty", "console"]);
        assert!(command.fallback_used);
    }
}
```

Update `crates/minitd/src/lib.rs`:

```rust
pub mod early_mounts;
pub mod rescue;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn run() {
    let exit_code = run_with_args(std::env::args());
    std::process::exit(exit_code);
}

pub fn run_with_args<I, S>(_args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    0
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_package_version() {
        assert_eq!(crate::version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn run_with_args_returns_success_for_host_smoke() {
        assert_eq!(crate::run_with_args(["minitd"]), 0);
    }
}
```

Keep `crates/minitd/src/main.rs`:

```rust
fn main() {
    minitd::run();
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p minitd`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/minitd/src
git commit -m "feat: add rescue command selection"
```

---

### Task 5: Child Reaping Model and Linux Adapter

**Files:**
- Modify: `crates/minitd/src/lib.rs`
- Create: `crates/minitd/src/reaper.rs`

**Interfaces:**
- Produces: `minitd::reaper::ReapEvent`.
- Produces: `minitd::reaper::ReapStatus`.
- Produces: `minitd::reaper::Reaper` trait.
- Produces: `minitd::reaper::drain_reap_events<R: Reaper>(reaper: &mut R) -> Result<Vec<ReapEvent>, ReapError>`.

- [ ] **Step 1: Write failing tests**

Create `crates/minitd/src/reaper.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct FakeReaper {
        events: Vec<ReapEvent>,
    }

    impl Reaper for FakeReaper {
        fn reap_once(&mut self) -> Result<Option<ReapEvent>, ReapError> {
            Ok(self.events.pop())
        }
    }

    #[test]
    fn drain_reap_events_collects_until_empty() {
        let mut reaper = FakeReaper {
            events: vec![
                ReapEvent { pid: 12, status: ReapStatus::Exited(0) },
                ReapEvent { pid: 11, status: ReapStatus::Signaled(15) },
            ],
        };

        let events = drain_reap_events(&mut reaper).expect("reap drain should succeed");

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].pid, 11);
        assert_eq!(events[1].pid, 12);
    }
}
```

- [ ] **Step 2: Run test to verify red**

Run: `cargo test -p minitd reaper`

Expected: FAIL with unresolved `Reaper`, `ReapEvent`, `ReapStatus`, and `drain_reap_events`.

- [ ] **Step 3: Implement reaper model**

Create `crates/minitd/src/reaper.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReapEvent {
    pub pid: i32,
    pub status: ReapStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReapStatus {
    Exited(i32),
    Signaled(i32),
    StillAlive,
}

#[derive(Debug, Error)]
pub enum ReapError {
    #[error("wait failed: {0}")]
    Wait(String),
}

pub trait Reaper {
    fn reap_once(&mut self) -> Result<Option<ReapEvent>, ReapError>;
}

pub fn drain_reap_events<R: Reaper>(reaper: &mut R) -> Result<Vec<ReapEvent>, ReapError> {
    let mut events = Vec::new();

    while let Some(event) = reaper.reap_once()? {
        events.push(event);
    }

    Ok(events)
}

#[cfg(target_os = "linux")]
pub struct LinuxReaper;

#[cfg(target_os = "linux")]
impl Reaper for LinuxReaper {
    fn reap_once(&mut self) -> Result<Option<ReapEvent>, ReapError> {
        use nix::errno::Errno;
        use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
        use nix::unistd::Pid;

        match waitpid(Pid::from_raw(-1), Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => Ok(None),
            Ok(WaitStatus::Exited(pid, code)) => Ok(Some(ReapEvent {
                pid: pid.as_raw(),
                status: ReapStatus::Exited(code),
            })),
            Ok(WaitStatus::Signaled(pid, signal, _)) => Ok(Some(ReapEvent {
                pid: pid.as_raw(),
                status: ReapStatus::Signaled(signal as i32),
            })),
            Ok(_) => Ok(None),
            Err(Errno::ECHILD) => Ok(None),
            Err(error) => Err(ReapError::Wait(error.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeReaper {
        events: Vec<ReapEvent>,
    }

    impl Reaper for FakeReaper {
        fn reap_once(&mut self) -> Result<Option<ReapEvent>, ReapError> {
            Ok(self.events.pop())
        }
    }

    #[test]
    fn drain_reap_events_collects_until_empty() {
        let mut reaper = FakeReaper {
            events: vec![
                ReapEvent { pid: 12, status: ReapStatus::Exited(0) },
                ReapEvent { pid: 11, status: ReapStatus::Signaled(15) },
            ],
        };

        let events = drain_reap_events(&mut reaper).expect("reap drain should succeed");

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].pid, 11);
        assert_eq!(events[1].pid, 12);
    }
}
```

Update `crates/minitd/src/lib.rs` to export:

```rust
pub mod reaper;
```

without removing the existing module exports.

- [ ] **Step 4: Run tests**

Run: `cargo test -p minitd reaper`

Expected: PASS.

Run: `cargo test`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/minitd/src
git commit -m "feat: add child reaping adapter"
```

---

### Task 6: Shutdown Action Model and Linux Adapter

**Files:**
- Modify: `crates/minitd/src/lib.rs`
- Create: `crates/minitd/src/shutdown.rs`

**Interfaces:**
- Produces: `minitd::shutdown::ShutdownAction`.
- Produces: `minitd::shutdown::ShutdownExecutor` trait.
- Produces: `minitd::shutdown::perform_shutdown<E: ShutdownExecutor>(executor: &mut E, action: ShutdownAction) -> Result<(), ShutdownError>`.

- [ ] **Step 1: Write failing tests**

Create `crates/minitd/src/shutdown.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeShutdownExecutor {
        calls: Vec<String>,
    }

    impl ShutdownExecutor for FakeShutdownExecutor {
        fn sync_filesystems(&mut self) -> Result<(), ShutdownError> {
            self.calls.push("sync".to_string());
            Ok(())
        }

        fn reboot(&mut self, action: ShutdownAction) -> Result<(), ShutdownError> {
            self.calls.push(format!("reboot:{action:?}"));
            Ok(())
        }
    }

    #[test]
    fn shutdown_syncs_before_poweroff() {
        let mut executor = FakeShutdownExecutor::default();

        perform_shutdown(&mut executor, ShutdownAction::Poweroff).expect("shutdown should succeed");

        assert_eq!(executor.calls, vec!["sync", "reboot:Poweroff"]);
    }
}
```

- [ ] **Step 2: Run test to verify red**

Run: `cargo test -p minitd shutdown`

Expected: FAIL with unresolved `ShutdownAction`, `ShutdownExecutor`, and `perform_shutdown`.

- [ ] **Step 3: Implement shutdown model**

Create `crates/minitd/src/shutdown.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownAction {
    Halt,
    Poweroff,
    Reboot,
}

#[derive(Debug, Error)]
pub enum ShutdownError {
    #[error("sync failed: {0}")]
    Sync(String),
    #[error("reboot syscall failed: {0}")]
    Reboot(String),
}

pub trait ShutdownExecutor {
    fn sync_filesystems(&mut self) -> Result<(), ShutdownError>;
    fn reboot(&mut self, action: ShutdownAction) -> Result<(), ShutdownError>;
}

pub fn perform_shutdown<E: ShutdownExecutor>(
    executor: &mut E,
    action: ShutdownAction,
) -> Result<(), ShutdownError> {
    executor.sync_filesystems()?;
    executor.reboot(action)
}

#[cfg(target_os = "linux")]
pub struct LinuxShutdownExecutor;

#[cfg(target_os = "linux")]
impl ShutdownExecutor for LinuxShutdownExecutor {
    fn sync_filesystems(&mut self) -> Result<(), ShutdownError> {
        unsafe {
            libc::sync();
        }
        Ok(())
    }

    fn reboot(&mut self, action: ShutdownAction) -> Result<(), ShutdownError> {
        let command = match action {
            ShutdownAction::Halt => libc::RB_HALT_SYSTEM,
            ShutdownAction::Poweroff => libc::RB_POWER_OFF,
            ShutdownAction::Reboot => libc::RB_AUTOBOOT,
        };

        let result = unsafe { libc::reboot(command) };
        if result == 0 {
            Ok(())
        } else {
            Err(ShutdownError::Reboot(std::io::Error::last_os_error().to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeShutdownExecutor {
        calls: Vec<String>,
    }

    impl ShutdownExecutor for FakeShutdownExecutor {
        fn sync_filesystems(&mut self) -> Result<(), ShutdownError> {
            self.calls.push("sync".to_string());
            Ok(())
        }

        fn reboot(&mut self, action: ShutdownAction) -> Result<(), ShutdownError> {
            self.calls.push(format!("reboot:{action:?}"));
            Ok(())
        }
    }

    #[test]
    fn shutdown_syncs_before_poweroff() {
        let mut executor = FakeShutdownExecutor::default();

        perform_shutdown(&mut executor, ShutdownAction::Poweroff).expect("shutdown should succeed");

        assert_eq!(executor.calls, vec!["sync", "reboot:Poweroff"]);
    }
}
```

Update `crates/minitd/src/lib.rs` to export:

```rust
pub mod shutdown;
```

without removing existing exports.

- [ ] **Step 4: Run tests**

Run: `cargo test -p minitd shutdown`

Expected: PASS.

Run: `cargo test`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/minitd/src
git commit -m "feat: add shutdown adapter"
```

---

### Task 7: Linux Rescue Runtime Wiring

**Files:**
- Modify: `crates/minitd/src/lib.rs`
- Modify: `crates/minitd/src/rescue.rs`

**Interfaces:**
- Consumes: `LinuxMountExecutor`, `LinuxReaper`, `LinuxShutdownExecutor`.
- Produces: Linux-only `minitd::rescue::run_linux_rescue() -> i32`.
- Host non-Linux `run_with_args` must still return success for smoke tests.

- [ ] **Step 1: Write failing host-safe test**

Append to `crates/minitd/src/lib.rs` tests:

```rust
#[test]
fn host_run_accepts_rescue_flag() {
    assert_eq!(crate::run_with_args(["minitd", "--rescue"]), 0);
}
```

- [ ] **Step 2: Run test to verify red**

Run: `cargo test -p minitd host_run_accepts_rescue_flag`

Expected: FAIL if `run_with_args` does not parse the rescue flag or returns the wrong code.

- [ ] **Step 3: Implement rescue runtime wiring**

Update `crates/minitd/src/lib.rs` so `run_with_args` detects `--rescue`:

```rust
pub mod early_mounts;
pub mod reaper;
pub mod rescue;
pub mod shutdown;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn run() {
    let exit_code = run_with_args(std::env::args());
    std::process::exit(exit_code);
}

pub fn run_with_args<I, S>(args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    let rescue_requested = args
        .iter()
        .any(|arg| arg == "--rescue" || arg == "minit.rescue=1");

    if rescue_requested {
        return run_rescue_entrypoint();
    }

    0
}

fn run_rescue_entrypoint() -> i32 {
    #[cfg(target_os = "linux")]
    {
        rescue::run_linux_rescue()
    }

    #[cfg(not(target_os = "linux"))]
    {
        0
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_package_version() {
        assert_eq!(crate::version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn run_with_args_returns_success_for_host_smoke() {
        assert_eq!(crate::run_with_args(["minitd"]), 0);
    }

    #[test]
    fn host_run_accepts_rescue_flag() {
        assert_eq!(crate::run_with_args(["minitd", "--rescue"]), 0);
    }

    #[test]
    fn host_run_accepts_kernel_rescue_arg() {
        assert_eq!(crate::run_with_args(["minitd", "minit.rescue=1"]), 0);
    }
}
```

Append Linux-only runtime to `crates/minitd/src/rescue.rs`:

```rust
#[cfg(target_os = "linux")]
pub fn run_linux_rescue() -> i32 {
    use crate::early_mounts::{ensure_early_mounts, LinuxMountExecutor};
    use crate::reaper::{drain_reap_events, LinuxReaper};
    use minit_core::boot::RescueConfig;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    let mut mount_executor = LinuxMountExecutor;
    if let Err(error) = ensure_early_mounts(&mut mount_executor) {
        eprintln!("minitd: early mount failed: {error}");
    }

    let config = RescueConfig::default();
    let candidates = existing_rescue_candidates();
    let command = select_rescue_command(&config, &candidates);

    let child_result = Command::new(&command.argv[0]).args(&command.argv[1..]).spawn();
    let mut child = match child_result {
        Ok(child) => child,
        Err(error) => {
            eprintln!("minitd: failed to start rescue command {:?}: {error}", command.argv);
            return 1;
        }
    };

    let mut reaper = LinuxReaper;
    loop {
        if let Err(error) = drain_reap_events(&mut reaper) {
            eprintln!("minitd: reap failed: {error}");
        }

        match child.try_wait() {
            Ok(Some(status)) => return status.code().unwrap_or(0),
            Ok(None) => thread::sleep(Duration::from_millis(100)),
            Err(error) => {
                eprintln!("minitd: failed to observe rescue command: {error}");
                return 1;
            }
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p minitd`

Expected on Windows host: PASS, with Linux-only runtime excluded by `cfg`.

Run: `cargo test`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/minitd/src
git commit -m "feat: wire rescue runtime entrypoint"
```

---

### Task 8: VM Tooling Skeleton and Verification Contract

**Files:**
- Create: `tools/vm/build-initramfs.ps1`
- Create: `tools/vm/run-minit-qemu.ps1`
- Create: `tests/vm/README.md`

**Interfaces:**
- Produces: `tools/vm/build-initramfs.ps1 -MinitdPath C:\minit-vm\minitd -BusyBoxPath C:\minit-vm\busybox -Output tools\vm\artifacts\minit-initramfs.cpio`.
- Produces: `tools/vm/run-minit-qemu.ps1 -Kernel C:\minit-vm\bzImage -Initramfs tools\vm\artifacts\minit-initramfs.cpio`.
- Scripts must fail with actionable messages when required tools or files are missing.

- [ ] **Step 1: Write failing script smoke check**

Run: `powershell -NoProfile -ExecutionPolicy Bypass -File tools/vm/build-initramfs.ps1 -Help`

Expected: FAIL because the script does not exist.

- [ ] **Step 2: Implement build script**

Create `tools/vm/build-initramfs.ps1`:

```powershell
param(
    [string]$MinitdPath,
    [string]$BusyBoxPath,
    [string]$Output,
    [switch]$Help
)

if ($Help) {
    Write-Output "Usage: build-initramfs.ps1 -MinitdPath <minitd> -BusyBoxPath <busybox> -Output <initramfs.cpio>"
    exit 0
}

if (-not $MinitdPath -or -not (Test-Path -LiteralPath $MinitdPath)) {
    Write-Error "MinitdPath is required and must point to a built Linux minitd binary."
    exit 2
}

if (-not $BusyBoxPath -or -not (Test-Path -LiteralPath $BusyBoxPath)) {
    Write-Error "BusyBoxPath is required and must point to a static busybox binary."
    exit 2
}

if (-not $Output) {
    Write-Error "Output is required."
    exit 2
}

$outputPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Output)
$outputDir = Split-Path -Parent $outputPath
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

$bash = Get-Command bash -ErrorAction SilentlyContinue
if (-not $bash) {
    Write-Error "bash with find and cpio is required to build the initramfs."
    exit 3
}

$root = Join-Path $PSScriptRoot "artifacts/initramfs-root"
if (Test-Path -LiteralPath $root) {
    Remove-Item -LiteralPath $root -Recurse -Force
}

New-Item -ItemType Directory -Force -Path $root, "$root/bin", "$root/sbin", "$root/proc", "$root/sys", "$root/dev", "$root/run" | Out-Null
Copy-Item -LiteralPath $MinitdPath -Destination "$root/init" -Force
Copy-Item -LiteralPath $BusyBoxPath -Destination "$root/bin/busybox" -Force

Push-Location $root
try {
    $bashOutput = $outputPath -replace '\\','/'
    & $bash.Source -lc "find . -print0 | cpio --null -o -H newc > '$bashOutput'"
} finally {
    Pop-Location
}

Write-Output "Wrote $Output"
```

Create `tools/vm/run-minit-qemu.ps1`:

```powershell
param(
    [string]$Kernel,
    [string]$Initramfs,
    [int]$TimeoutSeconds = 20,
    [switch]$Help
)

if ($Help) {
    Write-Output "Usage: run-minit-qemu.ps1 -Kernel <bzImage> -Initramfs <initramfs.cpio>"
    exit 0
}

if (-not $Kernel -or -not (Test-Path -LiteralPath $Kernel)) {
    Write-Error "Kernel is required and must point to a Linux kernel image."
    exit 2
}

if (-not $Initramfs -or -not (Test-Path -LiteralPath $Initramfs)) {
    Write-Error "Initramfs is required and must point to an initramfs image."
    exit 2
}

$qemu = Get-Command qemu-system-x86_64 -ErrorAction SilentlyContinue
if (-not $qemu) {
    Write-Error "qemu-system-x86_64 is required for VM verification."
    exit 3
}

$args = @(
    "-m", "256M",
    "-kernel", $Kernel,
    "-initrd", $Initramfs,
    "-append", "console=ttyS0 init=/init minit.rescue=1",
    "-nographic",
    "-no-reboot"
)

$process = Start-Process -FilePath $qemu.Source -ArgumentList $args -NoNewWindow -PassThru
try {
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        Stop-Process -Id $process.Id -Force
        Write-Error "QEMU timed out after $TimeoutSeconds seconds."
        exit 4
    }
    exit $process.ExitCode
} finally {
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
    }
}
```

Create `tests/vm/README.md`:

```markdown
# VM Tests

The first VM milestone is:

1. Boot a Linux kernel with an initramfs containing `minitd` as `/init`.
2. Run `minitd --rescue` as PID 1.
3. Mount `/proc`, `/sys`, `/dev`, and `/run`.
4. Start `/bin/sh` or `/sbin/getty console`.
5. Reap children.
6. Shut down or exit cleanly.

The PowerShell scripts in `tools/vm/` are verification helpers. They require a Linux kernel image, a static BusyBox binary, `cpio`, and `qemu-system-x86_64`.
```

- [ ] **Step 3: Run script help checks**

Run: `powershell -NoProfile -ExecutionPolicy Bypass -File tools/vm/build-initramfs.ps1 -Help`

Expected: PASS and prints usage.

Run: `powershell -NoProfile -ExecutionPolicy Bypass -File tools/vm/run-minit-qemu.ps1 -Help`

Expected: PASS and prints usage.

- [ ] **Step 4: Commit**

```bash
git add tools/vm tests/vm
git commit -m "test: add vm boot verification helpers"
```

---

### Task 9: Cross-Target Build and Milestone Verification

**Files:**
- Modify only if required by failed verification: `Cargo.toml`, `crates/minitd/src/*.rs`, `tools/vm/*.ps1`, `tests/vm/README.md`

**Interfaces:**
- Produces: documented verification result for host tests.
- Produces: Linux target build attempt for `minitd`.
- Produces: VM boot attempt when QEMU, kernel, BusyBox, cpio, and Linux Rust target are available.

- [ ] **Step 1: Run host tests**

Run: `cargo test`

Expected: PASS.

- [ ] **Step 2: Attempt Linux build**

Run: `rustup target add x86_64-unknown-linux-musl`

Expected: PASS if the target is available; if unavailable due network/toolchain environment, record the exact error.

Run: `cargo build -p minitd --target x86_64-unknown-linux-musl`

Expected: PASS if the Linux musl target and linker are installed. If the linker is unavailable on Windows, record the exact error and do not claim a VM-ready binary.

- [ ] **Step 3: Attempt VM smoke only when prerequisites exist**

Check:

```powershell
Get-Command qemu-system-x86_64 -ErrorAction SilentlyContinue
Get-Command bash -ErrorAction SilentlyContinue
```

Expected: If either command is missing, record VM verification as blocked by missing host tools.

If kernel, BusyBox, cpio, QEMU, and a Linux `minitd` binary are present, run:

```powershell
$linuxMinitd = "target\x86_64-unknown-linux-musl\debug\minitd"
$busybox = "C:\minit-vm\busybox"
$kernel = "C:\minit-vm\bzImage"
powershell -NoProfile -ExecutionPolicy Bypass -File tools/vm/build-initramfs.ps1 -MinitdPath $linuxMinitd -BusyBoxPath $busybox -Output tools/vm/artifacts/minit-initramfs.cpio
powershell -NoProfile -ExecutionPolicy Bypass -File tools/vm/run-minit-qemu.ps1 -Kernel $kernel -Initramfs tools/vm/artifacts/minit-initramfs.cpio
```

Expected: VM reaches rescue shell/getty or exits cleanly. If not, capture the exact failure and fix code or scripts before committing verification notes.

- [ ] **Step 4: Commit fixes only if changes were required**

```bash
git add Cargo.toml crates/minitd/src tools/vm tests/vm
git commit -m "fix: complete minimal boot verification"
```

Do not create an empty commit if no code changes were required.
