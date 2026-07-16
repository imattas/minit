use serde::Deserialize;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitDefinition {
    pub unit: UnitSection,
    #[serde(default)]
    pub exec: ExecSection,
    #[serde(default)]
    pub dependencies: DependencySection,
    #[serde(default)]
    pub restart: RestartSection,
    #[serde(default)]
    pub resources: ResourceSection,
    #[serde(default)]
    pub mount: MountSection,
    #[serde(default)]
    pub swap: SwapSection,
    #[serde(default)]
    pub security: SecuritySection,
}

impl UnitDefinition {
    pub fn validate(&self) -> Result<(), UnitValidationError> {
        if self.unit.name.trim().is_empty() {
            return Err(UnitValidationError::EmptyField { field: "unit.name" });
        }
        if !is_safe_unit_name(&self.unit.name) {
            return Err(UnitValidationError::UnsafeUnitName {
                value: self.unit.name.clone(),
            });
        }

        if !self.is_service() && !self.is_target() && !self.is_mount() && !self.is_swap() {
            return Err(UnitValidationError::UnsupportedUnitKind {
                value: self.unit.kind.clone(),
            });
        }

        if self.is_service() {
            let Some(program) = self.exec.start.first() else {
                return Err(UnitValidationError::EmptyField {
                    field: "exec.start",
                });
            };

            if !program.starts_with('/') {
                return Err(UnitValidationError::NonAbsolutePath {
                    field: "exec.start[0]",
                    value: program.clone(),
                });
            }
        }

        if self.is_mount() {
            validate_required("mount.what", self.mount.what.as_deref())?;
            let Some(where_path) = self.mount.where_path.as_deref() else {
                return Err(UnitValidationError::EmptyField {
                    field: "mount.where",
                });
            };
            if !where_path.starts_with('/') {
                return Err(UnitValidationError::NonAbsolutePath {
                    field: "mount.where",
                    value: where_path.to_string(),
                });
            }
            validate_required("mount.fstype", self.mount.fstype.as_deref())?;
        }

        if self.is_swap() {
            let Some(path) = self.swap.path.as_deref() else {
                return Err(UnitValidationError::EmptyField { field: "swap.path" });
            };
            if !path.starts_with('/') {
                return Err(UnitValidationError::NonAbsolutePath {
                    field: "swap.path",
                    value: path.to_string(),
                });
            }
        }

        if self.security.private_tmp {
            return Err(UnitValidationError::UnsupportedSecurityOption {
                field: "security.private_tmp",
            });
        }
        if !self.security.readonly_paths.is_empty() {
            return Err(UnitValidationError::UnsupportedSecurityOption {
                field: "security.readonly_paths",
            });
        }
        if !self.security.readwrite_paths.is_empty() {
            return Err(UnitValidationError::UnsupportedSecurityOption {
                field: "security.readwrite_paths",
            });
        }
        for entry in &self.security.environment {
            validate_environment_entry(entry)?;
        }
        if let Some(user) = &self.security.user {
            validate_principal("security.user", user)?;
        }
        if let Some(group) = &self.security.group {
            validate_principal("security.group", group)?;
        }

        Ok(())
    }

    pub fn is_service(&self) -> bool {
        self.unit.kind == "service"
    }

    pub fn is_target(&self) -> bool {
        self.unit.kind == "target"
    }

    pub fn is_mount(&self) -> bool {
        self.unit.kind == "mount"
    }

    pub fn is_swap(&self) -> bool {
        self.unit.kind == "swap"
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitSection {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_unit_kind")]
    pub kind: String,
}

fn default_unit_kind() -> String {
    "service".to_string()
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecSection {
    #[serde(default)]
    pub start: Vec<String>,
    #[serde(default)]
    pub reload: Vec<String>,
    #[serde(default)]
    pub stop: Vec<String>,
    #[serde(default)]
    pub working_directory: Option<String>,
    pub stop_timeout: Option<String>,
}

impl ExecSection {
    pub fn stop_timeout_duration(&self) -> Duration {
        self.stop_timeout
            .as_deref()
            .and_then(parse_duration)
            .unwrap_or(Duration::from_millis(500))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencySection {
    #[serde(default)]
    pub after: Vec<String>,
    #[serde(default)]
    pub before: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub wants: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestartSection {
    pub policy: Option<String>,
    pub limit: Option<String>,
    pub backoff: Option<String>,
    pub max_delay: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceSection {
    pub memory_max: Option<String>,
    pub cpu_max: Option<String>,
    pub pids_max: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MountSection {
    pub what: Option<String>,
    #[serde(rename = "where")]
    pub where_path: Option<String>,
    pub fstype: Option<String>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default = "default_required")]
    pub required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwapSection {
    pub path: Option<String>,
    pub priority: Option<i32>,
    #[serde(default = "default_required")]
    pub required: bool,
}

fn default_required() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    Never,
    Always,
    OnFailure,
}

impl RestartSection {
    pub fn policy(&self) -> RestartPolicy {
        match self.policy.as_deref() {
            Some("always") => RestartPolicy::Always,
            Some("on-failure") => RestartPolicy::OnFailure,
            _ => RestartPolicy::Never,
        }
    }

    pub fn limit_count(&self) -> Option<u32> {
        self.limit
            .as_deref()
            .and_then(|limit| {
                limit
                    .split_once('/')
                    .map_or(Some(limit), |(count, _)| Some(count))
            })
            .and_then(|count| count.parse::<u32>().ok())
    }

    pub fn limit_window(&self) -> Option<Duration> {
        self.limit
            .as_deref()
            .and_then(|limit| limit.split_once('/').map(|(_, window)| window))
            .and_then(parse_duration)
    }

    pub fn backoff_delay(&self, attempt: u32) -> Duration {
        let delay = match self.backoff.as_deref() {
            Some("fixed") => Duration::from_secs(1),
            Some("exponential") => {
                let exponent = attempt.saturating_sub(1).min(31);
                Duration::from_secs(1u64 << exponent)
            }
            _ => Duration::ZERO,
        };
        let max_delay = self
            .max_delay
            .as_deref()
            .and_then(parse_duration)
            .unwrap_or(Duration::from_secs(30));
        delay.min(max_delay)
    }
}

fn parse_duration(value: &str) -> Option<Duration> {
    let value = value.trim();
    if value == "min" {
        return Some(Duration::from_secs(60));
    }
    if let Some(number) = value.strip_suffix("ms") {
        return number.parse::<u64>().ok().map(Duration::from_millis);
    }
    if let Some(number) = value.strip_suffix("min") {
        return number
            .parse::<u64>()
            .ok()
            .map(|value| Duration::from_secs(value * 60));
    }
    if let Some(number) = value.strip_suffix('m') {
        return number
            .parse::<u64>()
            .ok()
            .map(|value| Duration::from_secs(value * 60));
    }
    if let Some(number) = value.strip_suffix('s') {
        return number.parse::<u64>().ok().map(Duration::from_secs);
    }
    value.parse::<u64>().ok().map(Duration::from_secs)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecuritySection {
    pub user: Option<String>,
    pub group: Option<String>,
    #[serde(default)]
    pub no_new_privileges: bool,
    #[serde(default)]
    pub private_tmp: bool,
    #[serde(default)]
    pub readonly_paths: Vec<String>,
    #[serde(default)]
    pub readwrite_paths: Vec<String>,
    #[serde(default)]
    pub environment: Vec<String>,
}

#[derive(Debug, Error)]
pub enum UnitParseError {
    #[error("failed to parse unit TOML: {0}")]
    Toml(#[from] toml::de::Error),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UnitValidationError {
    #[error("{field} must not be empty")]
    EmptyField { field: &'static str },
    #[error("{field} must be an absolute path, got {value}")]
    NonAbsolutePath { field: &'static str, value: String },
    #[error("unit.name contains unsafe path characters: {value}")]
    UnsafeUnitName { value: String },
    #[error("unsupported unit kind: {value}")]
    UnsupportedUnitKind { value: String },
    #[error("unsupported security option: {field}")]
    UnsupportedSecurityOption { field: &'static str },
    #[error("invalid environment entry: {value}")]
    InvalidEnvironment { value: String },
    #[error("{field} must be \"root\" or a numeric id, got {value}")]
    InvalidPrincipal { field: &'static str, value: String },
}

impl UnitValidationError {
    pub fn field(&self) -> &'static str {
        match self {
            Self::EmptyField { field } => field,
            Self::NonAbsolutePath { field, .. } => field,
            Self::UnsafeUnitName { .. } => "unit.name",
            Self::UnsupportedUnitKind { .. } => "unit.kind",
            Self::UnsupportedSecurityOption { field } => field,
            Self::InvalidEnvironment { .. } => "security.environment",
            Self::InvalidPrincipal { field, .. } => field,
        }
    }
}

pub fn parse_unit_toml(input: &str) -> Result<UnitDefinition, UnitParseError> {
    Ok(toml::from_str(input)?)
}

pub fn is_safe_unit_name(unit: &str) -> bool {
    !unit.is_empty()
        && unit
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'@'))
}

fn validate_environment_entry(entry: &str) -> Result<(), UnitValidationError> {
    let Some((key, _value)) = entry.split_once('=') else {
        return Err(UnitValidationError::InvalidEnvironment {
            value: entry.to_string(),
        });
    };
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return Err(UnitValidationError::InvalidEnvironment {
            value: entry.to_string(),
        });
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(UnitValidationError::InvalidEnvironment {
            value: entry.to_string(),
        });
    }
    if !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        return Err(UnitValidationError::InvalidEnvironment {
            value: entry.to_string(),
        });
    }
    Ok(())
}

fn validate_required(field: &'static str, value: Option<&str>) -> Result<(), UnitValidationError> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(()),
        _ => Err(UnitValidationError::EmptyField { field }),
    }
}

fn validate_principal(field: &'static str, value: &str) -> Result<(), UnitValidationError> {
    if value == "root" || value.parse::<u32>().is_ok() {
        return Ok(());
    }
    Err(UnitValidationError::InvalidPrincipal {
        field,
        value: value.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SSHD_SERVICE: &str = r#"
[unit]
name = "sshd"
description = "OpenSSH daemon"
kind = "service"

[exec]
start = ["/usr/bin/sshd", "-D"]
reload = ["/bin/kill", "-HUP", "$MAINPID"]
stop = ["/bin/kill", "TERM", "$MAINPID"]
working_directory = "/"
stop_timeout = "750ms"

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

[security]
user = "root"
group = "root"
no_new_privileges = true
environment = ["RUST_LOG=info"]
"#;

    #[test]
    fn parses_basic_service_unit() {
        let unit = parse_unit_toml(SSHD_SERVICE).expect("unit should parse");

        assert_eq!(unit.unit.name, "sshd");
        assert_eq!(unit.unit.kind, "service");
        assert_eq!(unit.exec.start, vec!["/usr/bin/sshd", "-D"]);
        assert_eq!(
            unit.exec.stop_timeout_duration(),
            Duration::from_millis(750)
        );
        assert_eq!(unit.dependencies.after, vec!["network-online.target"]);
        assert_eq!(unit.restart.policy.as_deref(), Some("on-failure"));
        assert_eq!(unit.restart.policy(), RestartPolicy::OnFailure);
        assert_eq!(unit.restart.limit_count(), Some(5));
        assert_eq!(unit.security.user.as_deref(), Some("root"));
        assert!(unit.security.no_new_privileges);
    }

    #[test]
    fn target_units_parse_without_exec_section() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "multi-user.target"
kind = "target"
description = "Normal multi-user boot target"

[dependencies]
wants = ["getty.service"]
"#,
        )
        .expect("target unit should parse without exec");

        unit.validate().expect("target unit should validate");
        assert!(unit.is_target());
        assert!(!unit.is_service());
        assert_eq!(unit.dependencies.wants, vec!["getty.service"]);
    }

    #[test]
    fn restart_policy_defaults_to_never() {
        let unit = parse_unit_toml(SSHD_SERVICE).expect("unit should parse");

        assert_eq!(RestartSection::default().policy(), RestartPolicy::Never);
        assert_eq!(RestartSection::default().limit_count(), None);
        assert_eq!(unit.restart.limit_count(), Some(5));
    }

    #[test]
    fn validation_rejects_empty_unit_name() {
        let mut unit = parse_unit_toml(SSHD_SERVICE).expect("unit should parse");
        unit.unit.name.clear();

        let error = unit.validate().expect_err("empty unit name must fail");

        assert_eq!(error.field(), "unit.name");
    }

    #[test]
    fn validation_rejects_empty_start_command() {
        let mut unit = parse_unit_toml(SSHD_SERVICE).expect("unit should parse");
        unit.exec.start.clear();

        let error = unit.validate().expect_err("empty start command must fail");

        assert_eq!(error.field(), "exec.start");
    }

    #[test]
    fn validation_rejects_relative_start_program() {
        let mut unit = parse_unit_toml(SSHD_SERVICE).expect("unit should parse");
        unit.exec.start[0] = "sshd".to_string();

        let error = unit
            .validate()
            .expect_err("relative start program must fail");

        assert_eq!(error.field(), "exec.start[0]");
    }

    #[test]
    fn validation_rejects_unsafe_unit_name() {
        let mut unit = parse_unit_toml(SSHD_SERVICE).expect("unit should parse");
        unit.unit.name = "../escape".to_string();

        let error = unit.validate().expect_err("unsafe unit name must fail");

        assert_eq!(error.field(), "unit.name");
    }

    #[test]
    fn parsing_rejects_unknown_fields() {
        let error = parse_unit_toml(
            r#"
[unit]
name = "bad.service"
unknown = true

[exec]
start = ["/bin/true"]
"#,
        )
        .expect_err("unknown fields must fail closed");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn validation_rejects_unsupported_security_options() {
        let mut unit = parse_unit_toml(SSHD_SERVICE).expect("unit should parse");
        unit.security.private_tmp = true;

        let error = unit
            .validate()
            .expect_err("unsupported security option must fail");

        assert_eq!(error.field(), "security.private_tmp");
    }

    #[test]
    fn validation_rejects_invalid_environment_entries() {
        let mut unit = parse_unit_toml(SSHD_SERVICE).expect("unit should parse");
        unit.security.environment = vec!["BAD-NAME=value".to_string()];

        let error = unit
            .validate()
            .expect_err("invalid environment entry must fail");

        assert_eq!(error.field(), "security.environment");
    }

    #[test]
    fn validation_rejects_unsupported_user_names() {
        let mut unit = parse_unit_toml(SSHD_SERVICE).expect("unit should parse");
        unit.security.user = Some("daemon".to_string());

        let error = unit
            .validate()
            .expect_err("unsupported user names must fail closed");

        assert_eq!(error.field(), "security.user");
    }

    #[test]
    fn parses_cgroup_resource_limits() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "limited.service"

[exec]
start = ["/bin/sleep", "60"]

[resources]
memory_max = "64M"
cpu_max = "50000 100000"
pids_max = "32"
"#,
        )
        .expect("resource-limited unit should parse");

        assert_eq!(unit.resources.memory_max.as_deref(), Some("64M"));
        assert_eq!(unit.resources.cpu_max.as_deref(), Some("50000 100000"));
        assert_eq!(unit.resources.pids_max.as_deref(), Some("32"));
    }

    #[test]
    fn parses_mount_unit() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "var-log.mount"
kind = "mount"

[mount]
what = "tmpfs"
where = "/var/log"
fstype = "tmpfs"
options = ["nosuid", "nodev"]
"#,
        )
        .expect("mount unit should parse");

        unit.validate().expect("mount unit should validate");
        assert!(unit.is_mount());
        assert_eq!(unit.mount.what.as_deref(), Some("tmpfs"));
        assert_eq!(unit.mount.where_path.as_deref(), Some("/var/log"));
        assert_eq!(unit.mount.fstype.as_deref(), Some("tmpfs"));
        assert_eq!(unit.mount.options, vec!["nosuid", "nodev"]);
    }

    #[test]
    fn parses_swap_unit() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "scratch.swap"
kind = "swap"

[swap]
path = "/swapfile"
priority = 5
"#,
        )
        .expect("swap unit should parse");

        unit.validate().expect("swap unit should validate");
        assert!(unit.is_swap());
        assert_eq!(unit.swap.path.as_deref(), Some("/swapfile"));
        assert_eq!(unit.swap.priority, Some(5));
    }

    #[test]
    fn validation_rejects_mount_without_absolute_target() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "bad.mount"
kind = "mount"

[mount]
what = "tmpfs"
where = "var/log"
fstype = "tmpfs"
"#,
        )
        .expect("mount unit should parse");

        let error = unit
            .validate()
            .expect_err("relative mount target must fail");

        assert_eq!(error.field(), "mount.where");
    }

    #[test]
    fn parses_all_example_service_files() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let examples_dir = std::path::Path::new(manifest_dir).join("../../config/examples");
        let entries = std::fs::read_dir(&examples_dir).expect("config/examples should exist");
        let mut parsed = 0;

        for entry in entries {
            let path = entry.expect("example entry should be readable").path();
            if path.extension().and_then(|value| value.to_str()) != Some("toml") {
                continue;
            }

            let input = std::fs::read_to_string(&path).expect("example should be readable");
            let unit = parse_unit_toml(&input).expect("example should parse");
            unit.validate().expect("example should validate");
            parsed += 1;
        }

        assert_eq!(parsed, 12);
    }

    #[test]
    fn parses_all_profile_unit_files() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let profiles_dir = std::path::Path::new(manifest_dir).join("../../config/profiles");
        let mut profile_counts = std::collections::BTreeMap::new();

        for profile in std::fs::read_dir(&profiles_dir).expect("config/profiles should exist") {
            let profile = profile.expect("profile entry should be readable");
            if !profile.path().is_dir() {
                continue;
            }
            let profile_name = profile.file_name().to_string_lossy().to_string();
            let mut parsed = 0;
            for entry in std::fs::read_dir(profile.path()).expect("profile should be readable") {
                let path = entry.expect("profile unit should be readable").path();
                if path.extension().and_then(|value| value.to_str()) != Some("toml") {
                    continue;
                }

                let input =
                    std::fs::read_to_string(&path).expect("profile unit should be readable");
                let unit = parse_unit_toml(&input).expect("profile unit should parse");
                unit.validate().expect("profile unit should validate");
                parsed += 1;
            }
            profile_counts.insert(profile_name, parsed);
        }

        assert_eq!(profile_counts.get("minimal-distro"), Some(&5));
        assert_eq!(profile_counts.get("alpine-minirootfs"), Some(&3));
    }
}
