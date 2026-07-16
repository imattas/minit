# minitctl Basic CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first `minitctl` binary with typed command parsing and a conservative `status` command that reports when `minitd` IPC is unavailable.

**Architecture:** Keep command parsing in the `minitctl` crate. Do not invent daemon behavior or fake service state. Until IPC exists, `status` must clearly report that `minitd` is unavailable.

**Tech Stack:** Rust 2021, `clap` derive, Cargo tests.

## Global Constraints

- CLI name: `minitctl`.
- PID 1 daemon name: `minitd`.
- PID 1 stays small; CLI code must not move into `minitd`.
- Do not fake service state before IPC exists.
- Every behavior-bearing Rust task must use TDD.

---

### Task 1: CLI Crate and Commands

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/minitctl/Cargo.toml`
- Create: `crates/minitctl/src/lib.rs`
- Create: `crates/minitctl/src/main.rs`

**Interfaces:**
- Produces: `minitctl::Cli`.
- Produces: `minitctl::Command`.
- Produces: `minitctl::run_with_args<I, S>(args: I) -> i32`.

**Steps:**
- [ ] Write tests that parse `minitctl status`, `minitctl status sshd`, `minitctl start sshd`, `minitctl stop sshd`, and `minitctl restart sshd`.
- [ ] Run the tests and verify they fail because the crate does not exist.
- [ ] Add the crate and `clap` workspace dependency.
- [ ] Implement typed command parsing.
- [ ] Run `cargo test -p minitctl`.
- [ ] Commit with `feat: add minitctl command parser`.

### Task 2: Status Unavailable Output

**Files:**
- Modify: `crates/minitctl/src/lib.rs`

**Interfaces:**
- Produces: `minitctl::render_status_unavailable(unit: Option<&str>) -> String`.

**Steps:**
- [ ] Write tests for global and per-unit unavailable status output.
- [ ] Run the tests and verify they fail on missing renderer.
- [ ] Implement the renderer and wire `run_with_args` to print it for `status`.
- [ ] Run `cargo test -p minitctl`.
- [ ] Run `cargo test`.
- [ ] Commit with `feat: report unavailable minitd status`.
