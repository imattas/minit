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
}
