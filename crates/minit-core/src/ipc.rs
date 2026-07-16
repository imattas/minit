use serde::{Deserialize, Serialize};

pub const DEFAULT_CONTROL_SOCKET: &str = "/run/minit/minitd.sock";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlRequest {
    Status { unit: Option<String> },
    Start { unit: String },
    Stop { unit: String },
    Restart { unit: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitState {
    Unknown,
    Inactive,
    Starting,
    Active,
    Stopping,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitStatus {
    pub unit: String,
    pub state: UnitState,
    pub main_pid: Option<u32>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlResponse {
    Status { units: Vec<UnitStatus> },
    Accepted { message: String },
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serializes_as_tagged_json() {
        let request = ControlRequest::Start {
            unit: "sshd".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();

        assert_eq!(json, r#"{"type":"start","unit":"sshd"}"#);
    }

    #[test]
    fn response_round_trips_unit_status() {
        let response = ControlResponse::Status {
            units: vec![UnitStatus {
                unit: "sshd".to_string(),
                state: UnitState::Active,
                main_pid: Some(42),
                description: Some("OpenSSH daemon".to_string()),
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: ControlResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, response);
    }
}
