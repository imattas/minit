# minit Service Parser Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first TOML service parser and validation layer in `minit-core`, plus example service fixtures that later `minitd` and `minitctl` code can consume.

**Architecture:** Keep parsing and validation pure in `minit-core`. Do not start services in this slice. Represent the v1 TOML service shape from the approved spec with strict typed structs and actionable validation errors.

**Tech Stack:** Rust 2021, `serde`, `toml`, `thiserror`, Cargo tests.

## Global Constraints

- Project name: `minit`.
- Service files use readable TOML.
- PID 1 stays small.
- Runtime PID 1 code remains in `crates/minitd`; parser logic remains in `crates/minit-core`.
- Unknown runtime behavior must not be added in this slice.
- Every behavior-bearing Rust task must use TDD.

---

### Task 1: TOML Service Model

**Files:**
- Modify: `crates/minit-core/Cargo.toml`
- Modify: `crates/minit-core/src/lib.rs`
- Create: `crates/minit-core/src/unit.rs`

**Interfaces:**
- Produces: `minit_core::unit::UnitDefinition`.
- Produces: `minit_core::unit::UnitSection`.
- Produces: `minit_core::unit::ExecSection`.
- Produces: `minit_core::unit::DependencySection`.
- Produces: `minit_core::unit::RestartSection`.
- Produces: `minit_core::unit::SecuritySection`.
- Produces: `minit_core::unit::parse_unit_toml(input: &str) -> Result<UnitDefinition, UnitParseError>`.

**Steps:**
- [ ] Write tests in `crates/minit-core/src/unit.rs` proving the approved `sshd` TOML shape parses into typed fields.
- [ ] Run `cargo test -p minit-core unit::tests::parses_basic_service_unit` and verify it fails on missing parser/types.
- [ ] Add `toml.workspace = true` to the root manifest and `toml.workspace = true` to `minit-core`.
- [ ] Implement serde-backed structs and `parse_unit_toml`.
- [ ] Run `cargo test -p minit-core unit`.
- [ ] Commit with `feat: add toml unit parser`.

### Task 2: Service Validation

**Files:**
- Modify: `crates/minit-core/src/unit.rs`

**Interfaces:**
- Produces: `UnitDefinition::validate(&self) -> Result<(), UnitValidationError>`.
- Produces: `UnitValidationError`.

**Steps:**
- [ ] Write tests that reject an empty unit name, non-absolute `exec.start[0]`, and an empty `exec.start`.
- [ ] Run the validation tests and verify they fail on missing validation.
- [ ] Implement validation with field-specific errors containing the bad field path.
- [ ] Run `cargo test -p minit-core unit`.
- [ ] Commit with `feat: validate service unit definitions`.

### Task 3: Example Service Fixtures

**Files:**
- Create: `config/examples/sshd.service.toml`
- Create: `config/examples/getty.service.toml`
- Create: `config/examples/rescue-shell.service.toml`

**Interfaces:**
- Produces real TOML examples parseable by `parse_unit_toml`.

**Steps:**
- [ ] Add a test in `unit.rs` that parses all files under `config/examples/*.service.toml`.
- [ ] Run the test and verify it fails because examples do not exist.
- [ ] Create the three example service files.
- [ ] Run `cargo test -p minit-core unit`.
- [ ] Commit with `docs: add initial service examples`.

### Task 4: Parser Verification

**Files:**
- Modify only if verification reveals issues.

**Steps:**
- [ ] Run `cargo test`.
- [ ] Run `cargo build -p minitd --target x86_64-unknown-linux-musl`.
- [ ] Commit fixes only if required.
