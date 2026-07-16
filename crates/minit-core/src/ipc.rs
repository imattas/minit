use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_CONTROL_SOCKET: &str = "/run/minit/minitd.sock";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlRequest {
    Status { unit: Option<String> },
    List,
    Explain { unit: String },
    Events,
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
    pub restart_attempts: u32,
    pub last_exit_status: Option<String>,
    pub cgroup_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlResponse {
    Status {
        units: Vec<UnitStatus>,
    },
    Explanation {
        unit: String,
        lines: Vec<String>,
    },
    Events {
        events: Vec<crate::diagnostics::DiagnosticEvent>,
    },
    Accepted {
        message: String,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Error)]
pub enum WireError {
    #[error("control message is empty")]
    Empty,
    #[error("control message is missing trailing newline")]
    MissingNewline,
    #[error("invalid control message: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn encode_request(request: &ControlRequest) -> Result<String, WireError> {
    encode_json_line(request)
}

pub fn decode_request(line: &str) -> Result<ControlRequest, WireError> {
    decode_json_line(line)
}

pub fn encode_response(response: &ControlResponse) -> Result<String, WireError> {
    encode_json_line(response)
}

pub fn decode_response(line: &str) -> Result<ControlResponse, WireError> {
    decode_json_line(line)
}

fn encode_json_line<T: Serialize>(value: &T) -> Result<String, WireError> {
    let mut line = serde_json::to_string(value)?;
    line.push('\n');
    Ok(line)
}

fn decode_json_line<T: DeserializeOwned>(line: &str) -> Result<T, WireError> {
    if line.is_empty() {
        return Err(WireError::Empty);
    }
    let Some(payload) = line.strip_suffix('\n') else {
        return Err(WireError::MissingNewline);
    };
    if payload.is_empty() {
        return Err(WireError::Empty);
    }
    Ok(serde_json::from_str(payload)?)
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
                restart_attempts: 0,
                last_exit_status: None,
                cgroup_path: None,
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: ControlResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, response);
    }

    #[test]
    fn events_request_and_response_round_trip() {
        let request = ControlRequest::Events;
        let response = ControlResponse::Events {
            events: vec![crate::diagnostics::DiagnosticEvent::sequenced(
                1,
                "control",
                "started sshd",
            )],
        };

        assert_eq!(
            decode_request(&encode_request(&request).unwrap()).unwrap(),
            request
        );
        assert_eq!(
            decode_response(&encode_response(&response).unwrap()).unwrap(),
            response
        );
    }

    #[test]
    fn list_request_round_trips() {
        let request = ControlRequest::List;

        assert_eq!(
            decode_request(&encode_request(&request).unwrap()).unwrap(),
            request
        );
    }

    #[test]
    fn request_wire_messages_are_newline_delimited() {
        let request = ControlRequest::Stop {
            unit: "sshd".to_string(),
        };

        let line = encode_request(&request).unwrap();

        assert!(line.ends_with('\n'));
        assert_eq!(decode_request(&line).unwrap(), request);
    }

    #[test]
    fn response_wire_messages_are_newline_delimited() {
        let response = ControlResponse::Accepted {
            message: "queued start for sshd".to_string(),
        };

        let line = encode_response(&response).unwrap();

        assert!(line.ends_with('\n'));
        assert_eq!(decode_response(&line).unwrap(), response);
    }

    #[test]
    fn wire_decode_rejects_missing_newline() {
        let err = decode_request(r#"{"type":"status","unit":null}"#).unwrap_err();

        assert!(matches!(err, WireError::MissingNewline));
    }

    #[test]
    fn wire_decode_rejects_empty_message() {
        let err = decode_request("").unwrap_err();

        assert!(matches!(err, WireError::Empty));
    }
}
