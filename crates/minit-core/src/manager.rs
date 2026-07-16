use crate::ipc::{UnitState, UnitStatus};
use crate::unit::{RestartPolicy, UnitDefinition, UnitValidationError};
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartPlan {
    pub unit: String,
    pub argv: Vec<String>,
    pub working_directory: Option<String>,
    pub no_new_privileges: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestartDecision {
    pub unit: String,
    pub restart: bool,
    pub delay: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRecord {
    pub definition: UnitDefinition,
    pub state: UnitState,
    pub main_pid: Option<u32>,
    pub restart_attempts: u32,
    pub last_exit_status: Option<String>,
    pub cgroup_path: Option<String>,
    restart_events: VecDeque<Duration>,
}

#[derive(Debug)]
pub struct ServiceManager {
    units: BTreeMap<String, ServiceRecord>,
    clock_base: Instant,
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self {
            units: BTreeMap::new(),
            clock_base: Instant::now(),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ServiceManagerError {
    #[error("unit {0} already exists")]
    DuplicateUnit(String),
    #[error("unit {0} was not found")]
    UnknownUnit(String),
    #[error("invalid unit: {0}")]
    InvalidUnit(#[from] UnitValidationError),
}

impl ServiceManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_unit(&mut self, definition: UnitDefinition) -> Result<(), ServiceManagerError> {
        definition.validate()?;
        let name = definition.unit.name.clone();
        if self.units.contains_key(&name) {
            return Err(ServiceManagerError::DuplicateUnit(name));
        }

        self.units.insert(
            name,
            ServiceRecord {
                definition,
                state: UnitState::Inactive,
                main_pid: None,
                restart_attempts: 0,
                last_exit_status: None,
                cgroup_path: None,
                restart_events: VecDeque::new(),
            },
        );
        Ok(())
    }

    pub fn status(&self, unit: Option<&str>) -> Result<Vec<UnitStatus>, ServiceManagerError> {
        match unit {
            Some(name) => {
                let record = self.record(name)?;
                Ok(vec![record.to_status()])
            }
            None => Ok(self.units.values().map(ServiceRecord::to_status).collect()),
        }
    }

    pub fn active_unit_names(&self) -> Vec<String> {
        self.units
            .iter()
            .filter(|(_, record)| record.state == UnitState::Active)
            .map(|(name, _)| name.clone())
            .rev()
            .collect()
    }

    pub fn plan_start(&mut self, unit: &str) -> Result<StartPlan, ServiceManagerError> {
        self.plan_start_inner(unit, true)
    }

    pub fn plan_restart(&mut self, unit: &str) -> Result<StartPlan, ServiceManagerError> {
        self.plan_start_inner(unit, false)
    }

    fn plan_start_inner(
        &mut self,
        unit: &str,
        reset_restart_attempts: bool,
    ) -> Result<StartPlan, ServiceManagerError> {
        let record = self.record_mut(unit)?;
        record.state = UnitState::Starting;
        record.main_pid = None;
        if reset_restart_attempts {
            record.restart_attempts = 0;
            record.restart_events.clear();
        }
        Ok(StartPlan {
            unit: unit.to_string(),
            argv: record.definition.exec.start.clone(),
            working_directory: record.definition.exec.working_directory.clone(),
            no_new_privileges: record.definition.security.no_new_privileges,
        })
    }

    pub fn mark_active(&mut self, unit: &str, main_pid: u32) -> Result<(), ServiceManagerError> {
        let record = self.record_mut(unit)?;
        record.state = UnitState::Active;
        record.main_pid = Some(main_pid);
        Ok(())
    }

    pub fn set_cgroup_path(
        &mut self,
        unit: &str,
        path: impl Into<String>,
    ) -> Result<(), ServiceManagerError> {
        let record = self.record_mut(unit)?;
        record.cgroup_path = Some(path.into());
        Ok(())
    }

    pub fn stop_timeout(&self, unit: &str) -> Result<Duration, ServiceManagerError> {
        Ok(self.record(unit)?.definition.exec.stop_timeout_duration())
    }

    pub fn record_exit(
        &mut self,
        pid: u32,
        successful: bool,
    ) -> Result<Option<RestartDecision>, ServiceManagerError> {
        let status = match successful {
            true => "exit 0",
            false => "exit nonzero",
        };
        self.record_exit_with_status_at(pid, successful, status, self.clock_base.elapsed())
    }

    pub fn record_exit_with_status(
        &mut self,
        pid: u32,
        successful: bool,
        status: impl Into<String>,
    ) -> Result<Option<RestartDecision>, ServiceManagerError> {
        self.record_exit_with_status_at(pid, successful, status, self.clock_base.elapsed())
    }

    pub fn record_exit_at(
        &mut self,
        pid: u32,
        successful: bool,
        now: Duration,
    ) -> Result<Option<RestartDecision>, ServiceManagerError> {
        let status = match successful {
            true => "exit 0",
            false => "exit nonzero",
        };
        self.record_exit_with_status_at(pid, successful, status, now)
    }

    pub fn record_exit_with_status_at(
        &mut self,
        pid: u32,
        successful: bool,
        status: impl Into<String>,
        now: Duration,
    ) -> Result<Option<RestartDecision>, ServiceManagerError> {
        let Some((unit, record)) = self
            .units
            .iter_mut()
            .find(|(_, record)| record.main_pid == Some(pid))
        else {
            return Ok(None);
        };

        record.main_pid = None;
        record.last_exit_status = Some(status.into());
        let should_restart = match record.definition.restart.policy() {
            RestartPolicy::Never => false,
            RestartPolicy::Always => true,
            RestartPolicy::OnFailure => !successful,
        };

        if let Some(window) = record.definition.restart.limit_window() {
            while record
                .restart_events
                .front()
                .is_some_and(|attempt| now.saturating_sub(*attempt) >= window)
            {
                record.restart_events.pop_front();
            }
        }

        let under_limit = record.definition.restart.limit_count().is_none_or(|limit| {
            if record.definition.restart.limit_window().is_some() {
                record.restart_events.len() < limit as usize
            } else {
                record.restart_attempts < limit
            }
        });

        if should_restart && under_limit {
            record.restart_attempts += 1;
            record.restart_events.push_back(now);
            let delay = record
                .definition
                .restart
                .backoff_delay(record.restart_attempts);
            record.state = UnitState::Starting;
            Ok(Some(RestartDecision {
                unit: unit.clone(),
                restart: true,
                delay,
            }))
        } else {
            record.state = match successful {
                true => UnitState::Inactive,
                false => UnitState::Failed,
            };
            Ok(Some(RestartDecision {
                unit: unit.clone(),
                restart: false,
                delay: Duration::ZERO,
            }))
        }
    }

    pub fn mark_inactive(&mut self, unit: &str) -> Result<(), ServiceManagerError> {
        let record = self.record_mut(unit)?;
        record.state = UnitState::Inactive;
        record.main_pid = None;
        Ok(())
    }

    pub fn mark_failed(&mut self, unit: &str) -> Result<(), ServiceManagerError> {
        let record = self.record_mut(unit)?;
        record.state = UnitState::Failed;
        record.main_pid = None;
        Ok(())
    }

    fn record(&self, unit: &str) -> Result<&ServiceRecord, ServiceManagerError> {
        self.units
            .get(unit)
            .ok_or_else(|| ServiceManagerError::UnknownUnit(unit.to_string()))
    }

    fn record_mut(&mut self, unit: &str) -> Result<&mut ServiceRecord, ServiceManagerError> {
        self.units
            .get_mut(unit)
            .ok_or_else(|| ServiceManagerError::UnknownUnit(unit.to_string()))
    }
}

impl ServiceRecord {
    fn to_status(&self) -> UnitStatus {
        UnitStatus {
            unit: self.definition.unit.name.clone(),
            state: self.state.clone(),
            main_pid: self.main_pid,
            description: match self.definition.unit.description.is_empty() {
                true => None,
                false => Some(self.definition.unit.description.clone()),
            },
            restart_attempts: self.restart_attempts,
            last_exit_status: self.last_exit_status.clone(),
            cgroup_path: self.cgroup_path.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::parse_unit_toml;

    const UNIT: &str = r#"
[unit]
name = "sshd"
description = "OpenSSH daemon"

[exec]
start = ["/usr/bin/sshd", "-D"]
"#;

    fn parsed_unit() -> UnitDefinition {
        parse_unit_toml(UNIT).unwrap()
    }

    #[test]
    fn add_unit_exposes_inactive_status() {
        let mut manager = ServiceManager::new();

        manager.add_unit(parsed_unit()).unwrap();
        let statuses = manager.status(Some("sshd")).unwrap();

        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].unit, "sshd");
        assert_eq!(statuses[0].state, UnitState::Inactive);
        assert_eq!(statuses[0].description.as_deref(), Some("OpenSSH daemon"));
    }

    #[test]
    fn duplicate_units_are_rejected() {
        let mut manager = ServiceManager::new();

        manager.add_unit(parsed_unit()).unwrap();
        let error = manager.add_unit(parsed_unit()).unwrap_err();

        assert_eq!(
            error,
            ServiceManagerError::DuplicateUnit("sshd".to_string())
        );
    }

    #[test]
    fn plan_start_returns_argv_and_marks_starting() {
        let mut manager = ServiceManager::new();
        manager.add_unit(parsed_unit()).unwrap();

        let plan = manager.plan_start("sshd").unwrap();
        let statuses = manager.status(Some("sshd")).unwrap();

        assert_eq!(plan.unit, "sshd");
        assert_eq!(plan.argv, vec!["/usr/bin/sshd", "-D"]);
        assert_eq!(plan.working_directory, None);
        assert!(!plan.no_new_privileges);
        assert_eq!(statuses[0].state, UnitState::Starting);
    }

    #[test]
    fn mark_active_records_main_pid() {
        let mut manager = ServiceManager::new();
        manager.add_unit(parsed_unit()).unwrap();
        manager.plan_start("sshd").unwrap();

        manager.mark_active("sshd", 123).unwrap();
        let statuses = manager.status(Some("sshd")).unwrap();

        assert_eq!(statuses[0].state, UnitState::Active);
        assert_eq!(statuses[0].main_pid, Some(123));
    }

    #[test]
    fn status_includes_restart_attempts_last_exit_and_cgroup_path() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "crashy"

[exec]
start = ["/bin/false"]

[restart]
policy = "on-failure"
limit = "2/min"
"#,
        )
        .unwrap();
        let mut manager = ServiceManager::new();
        manager.add_unit(unit).unwrap();
        manager
            .set_cgroup_path("crashy", "/sys/fs/cgroup/minit/crashy")
            .unwrap();
        manager.plan_start("crashy").unwrap();
        manager.mark_active("crashy", 42).unwrap();

        manager
            .record_exit_with_status(42, false, "exit 1")
            .unwrap()
            .unwrap();
        let statuses = manager.status(Some("crashy")).unwrap();

        assert_eq!(statuses[0].restart_attempts, 1);
        assert_eq!(statuses[0].last_exit_status.as_deref(), Some("exit 1"));
        assert_eq!(
            statuses[0].cgroup_path.as_deref(),
            Some("/sys/fs/cgroup/minit/crashy")
        );
    }

    #[test]
    fn stop_timeout_comes_from_unit_definition() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "slow-stop"

[exec]
start = ["/bin/sleep", "100"]
stop_timeout = "2s"
"#,
        )
        .unwrap();
        let mut manager = ServiceManager::new();
        manager.add_unit(unit).unwrap();

        assert_eq!(
            manager.stop_timeout("slow-stop").unwrap(),
            Duration::from_secs(2)
        );
    }

    #[test]
    fn unknown_unit_returns_error() {
        let manager = ServiceManager::new();

        let error = manager.status(Some("missing")).unwrap_err();

        assert_eq!(
            error,
            ServiceManagerError::UnknownUnit("missing".to_string())
        );
    }

    #[test]
    fn mark_failed_clears_main_pid() {
        let mut manager = ServiceManager::new();
        manager.add_unit(parsed_unit()).unwrap();
        manager.plan_start("sshd").unwrap();
        manager.mark_active("sshd", 123).unwrap();

        manager.mark_failed("sshd").unwrap();
        let statuses = manager.status(Some("sshd")).unwrap();

        assert_eq!(statuses[0].state, UnitState::Failed);
        assert_eq!(statuses[0].main_pid, None);
    }

    #[test]
    fn active_unit_names_returns_active_units_in_reverse_name_order() {
        let mut manager = ServiceManager::new();
        manager.add_unit(parsed_unit()).unwrap();
        let mut second = parsed_unit();
        second.unit.name = "z-last".to_string();
        manager.add_unit(second).unwrap();
        manager.plan_start("sshd").unwrap();
        manager.mark_active("sshd", 123).unwrap();
        manager.plan_start("z-last").unwrap();
        manager.mark_active("z-last", 124).unwrap();

        assert_eq!(
            manager.active_unit_names(),
            vec!["z-last".to_string(), "sshd".to_string()]
        );
    }

    #[test]
    fn record_exit_restarts_on_failure_within_limit() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "crashy"

[exec]
start = ["/bin/false"]

[restart]
policy = "on-failure"
limit = "2/min"
"#,
        )
        .unwrap();
        let mut manager = ServiceManager::new();
        manager.add_unit(unit).unwrap();
        manager.plan_start("crashy").unwrap();
        manager.mark_active("crashy", 42).unwrap();

        let decision = manager.record_exit(42, false).unwrap().unwrap();

        assert_eq!(
            decision,
            RestartDecision {
                unit: "crashy".to_string(),
                restart: true,
                delay: Duration::ZERO,
            }
        );
        let statuses = manager.status(Some("crashy")).unwrap();
        assert_eq!(statuses[0].state, UnitState::Starting);
    }

    #[test]
    fn record_exit_marks_failed_after_restart_limit() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "crashy"

[exec]
start = ["/bin/false"]

[restart]
policy = "on-failure"
limit = "1/min"
"#,
        )
        .unwrap();
        let mut manager = ServiceManager::new();
        manager.add_unit(unit).unwrap();
        manager.plan_start("crashy").unwrap();
        manager.mark_active("crashy", 42).unwrap();
        assert!(manager.record_exit(42, false).unwrap().unwrap().restart);
        manager.plan_restart("crashy").unwrap();
        manager.mark_active("crashy", 43).unwrap();

        let decision = manager.record_exit(43, false).unwrap().unwrap();

        assert_eq!(
            decision,
            RestartDecision {
                unit: "crashy".to_string(),
                restart: false,
                delay: Duration::ZERO,
            }
        );
        let statuses = manager.status(Some("crashy")).unwrap();
        assert_eq!(statuses[0].state, UnitState::Failed);
    }

    #[test]
    fn record_exit_restart_limit_expires_after_window() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "crashy"

[exec]
start = ["/bin/false"]

[restart]
policy = "on-failure"
limit = "1/min"
"#,
        )
        .unwrap();
        let mut manager = ServiceManager::new();
        manager.add_unit(unit).unwrap();
        manager.plan_start("crashy").unwrap();
        manager.mark_active("crashy", 42).unwrap();
        assert!(
            manager
                .record_exit_at(42, false, std::time::Duration::from_secs(0))
                .unwrap()
                .unwrap()
                .restart
        );
        manager.plan_restart("crashy").unwrap();
        manager.mark_active("crashy", 43).unwrap();
        assert!(
            !manager
                .record_exit_at(43, false, std::time::Duration::from_secs(30))
                .unwrap()
                .unwrap()
                .restart
        );

        manager.plan_restart("crashy").unwrap();
        manager.mark_active("crashy", 44).unwrap();
        let decision = manager
            .record_exit_at(44, false, std::time::Duration::from_secs(61))
            .unwrap()
            .unwrap();

        assert!(decision.restart);
    }

    #[test]
    fn restart_decision_includes_fixed_backoff_delay() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "crashy"

[exec]
start = ["/bin/false"]

[restart]
policy = "on-failure"
limit = "3/min"
backoff = "fixed"
"#,
        )
        .unwrap();
        let mut manager = ServiceManager::new();
        manager.add_unit(unit).unwrap();
        manager.plan_start("crashy").unwrap();
        manager.mark_active("crashy", 42).unwrap();

        let decision = manager
            .record_exit_at(42, false, std::time::Duration::from_secs(0))
            .unwrap()
            .unwrap();

        assert_eq!(decision.delay, std::time::Duration::from_secs(1));
    }

    #[test]
    fn restart_decision_caps_exponential_backoff_delay() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "crashy"

[exec]
start = ["/bin/false"]

[restart]
policy = "on-failure"
limit = "5/min"
backoff = "exponential"
max_delay = "2s"
"#,
        )
        .unwrap();
        let mut manager = ServiceManager::new();
        manager.add_unit(unit).unwrap();
        manager.plan_start("crashy").unwrap();
        manager.mark_active("crashy", 42).unwrap();
        assert_eq!(
            manager
                .record_exit_at(42, false, std::time::Duration::from_secs(0))
                .unwrap()
                .unwrap()
                .delay,
            std::time::Duration::from_secs(1)
        );
        manager.plan_restart("crashy").unwrap();
        manager.mark_active("crashy", 43).unwrap();
        assert_eq!(
            manager
                .record_exit_at(43, false, std::time::Duration::from_secs(1))
                .unwrap()
                .unwrap()
                .delay,
            std::time::Duration::from_secs(2)
        );
        manager.plan_restart("crashy").unwrap();
        manager.mark_active("crashy", 44).unwrap();

        let decision = manager
            .record_exit_at(44, false, std::time::Duration::from_secs(2))
            .unwrap()
            .unwrap();

        assert_eq!(decision.delay, std::time::Duration::from_secs(2));
    }
}
