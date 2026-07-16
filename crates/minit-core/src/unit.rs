use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct UnitDefinition {
    pub unit: UnitSection,
    pub exec: ExecSection,
    #[serde(default)]
    pub dependencies: DependencySection,
    #[serde(default)]
    pub restart: RestartSection,
    #[serde(default)]
    pub security: SecuritySection,
}

impl UnitDefinition {
    pub fn validate(&self) -> Result<(), UnitValidationError> {
        if self.unit.name.trim().is_empty() {
            return Err(UnitValidationError::EmptyField { field: "unit.name" });
        }

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

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ExecSection {
    pub start: Vec<String>,
    #[serde(default)]
    pub reload: Vec<String>,
    #[serde(default)]
    pub stop: Vec<String>,
    #[serde(default)]
    pub working_directory: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
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
pub struct RestartSection {
    pub policy: Option<String>,
    pub limit: Option<String>,
    pub backoff: Option<String>,
    pub max_delay: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
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
}

impl UnitValidationError {
    pub fn field(&self) -> &'static str {
        match self {
            Self::EmptyField { field } => field,
            Self::NonAbsolutePath { field, .. } => field,
        }
    }
}

pub fn parse_unit_toml(input: &str) -> Result<UnitDefinition, UnitParseError> {
    Ok(toml::from_str(input)?)
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
private_tmp = true
readonly_paths = ["/usr"]
readwrite_paths = ["/var/lib/sshd"]
environment = ["RUST_LOG=info"]
"#;

    #[test]
    fn parses_basic_service_unit() {
        let unit = parse_unit_toml(SSHD_SERVICE).expect("unit should parse");

        assert_eq!(unit.unit.name, "sshd");
        assert_eq!(unit.unit.kind, "service");
        assert_eq!(unit.exec.start, vec!["/usr/bin/sshd", "-D"]);
        assert_eq!(unit.dependencies.after, vec!["network-online.target"]);
        assert_eq!(unit.restart.policy.as_deref(), Some("on-failure"));
        assert_eq!(unit.security.user.as_deref(), Some("root"));
        assert!(unit.security.no_new_privileges);
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

        assert_eq!(parsed, 3);
    }
}
