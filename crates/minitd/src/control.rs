use minit_core::ipc::{
    decode_request, encode_response, ControlRequest, ControlResponse, WireError,
};
use minit_core::manager::ServiceManager;
use std::io::{self, BufRead, Write};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ControlError {
    #[error("invalid control request: {0}")]
    Request(#[from] WireError),
    #[error("failed to encode control response: {0}")]
    Response(WireError),
    #[error("control I/O failed: {0}")]
    Io(#[from] io::Error),
}

pub struct ControlService {
    services: ServiceManager,
}

impl ControlService {
    pub fn new(services: ServiceManager) -> Self {
        Self { services }
    }

    pub fn handle_request(&mut self, request: ControlRequest) -> ControlResponse {
        match request {
            ControlRequest::Status { unit } => match self.services.status(unit.as_deref()) {
                Ok(units) => ControlResponse::Status { units },
                Err(error) => ControlResponse::Error {
                    message: error.to_string(),
                },
            },
            ControlRequest::Start { unit } => not_implemented("start", &unit),
            ControlRequest::Stop { unit } => not_implemented("stop", &unit),
            ControlRequest::Restart { unit } => not_implemented("restart", &unit),
        }
    }
}

pub fn handle_control_request(request: ControlRequest) -> ControlResponse {
    match request {
        ControlRequest::Status { .. } => ControlResponse::Status { units: Vec::new() },
        ControlRequest::Start { unit } => not_implemented("start", &unit),
        ControlRequest::Stop { unit } => not_implemented("stop", &unit),
        ControlRequest::Restart { unit } => not_implemented("restart", &unit),
    }
}

pub fn handle_control_line(line: &str) -> Result<String, ControlError> {
    let request = decode_request(line)?;
    let response = handle_control_request(request);
    encode_response(&response).map_err(ControlError::Response)
}

pub fn handle_control_io<R, W>(reader: &mut R, writer: &mut W) -> Result<(), ControlError>
where
    R: BufRead,
    W: Write,
{
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let response_line = handle_control_line(&line)?;
    writer.write_all(response_line.as_bytes())?;
    writer.flush()?;
    Ok(())
}

fn not_implemented(command: &str, unit: &str) -> ControlResponse {
    ControlResponse::Error {
        message: format!("{command} is not implemented yet for {unit}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minit_core::ipc::{decode_response, encode_request, UnitState};
    use minit_core::manager::ServiceManager;
    use minit_core::unit::parse_unit_toml;

    #[test]
    fn status_request_returns_empty_status_until_manager_exists() {
        let response = handle_control_request(ControlRequest::Status { unit: None });

        assert_eq!(response, ControlResponse::Status { units: Vec::new() });
    }

    #[test]
    fn lifecycle_requests_return_explicit_unimplemented_errors() {
        let response = handle_control_request(ControlRequest::Start {
            unit: "sshd".to_string(),
        });

        assert_eq!(
            response,
            ControlResponse::Error {
                message: "start is not implemented yet for sshd".to_string()
            }
        );
    }

    #[test]
    fn handles_one_wire_request_line() {
        let request = encode_request(&ControlRequest::Status {
            unit: Some("sshd".to_string()),
        })
        .unwrap();

        let response_line = handle_control_line(&request).unwrap();
        let response = decode_response(&response_line).unwrap();

        assert_eq!(response, ControlResponse::Status { units: Vec::new() });
    }

    #[test]
    fn handles_one_buffered_request() {
        let request = encode_request(&ControlRequest::Start {
            unit: "sshd".to_string(),
        })
        .unwrap();
        let mut reader = std::io::BufReader::new(request.as_bytes());
        let mut writer = Vec::new();

        handle_control_io(&mut reader, &mut writer).unwrap();

        let response = String::from_utf8(writer).unwrap();
        let decoded = decode_response(&response).unwrap();
        assert_eq!(
            decoded,
            ControlResponse::Error {
                message: "start is not implemented yet for sshd".to_string()
            }
        );
    }

    fn service_manager_with_sshd() -> ServiceManager {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "sshd.service"
description = "OpenSSH daemon"

[exec]
start = ["/usr/bin/sshd", "-D"]
"#,
        )
        .unwrap();
        let mut services = ServiceManager::new();
        services.add_unit(unit).unwrap();
        services
    }

    #[test]
    fn control_service_reports_registered_unit_status() {
        let mut service = ControlService::new(service_manager_with_sshd());

        let response = service.handle_request(ControlRequest::Status {
            unit: Some("sshd.service".to_string()),
        });

        assert_eq!(
            response,
            ControlResponse::Status {
                units: vec![minit_core::ipc::UnitStatus {
                    unit: "sshd.service".to_string(),
                    state: UnitState::Inactive,
                    main_pid: None,
                    description: Some("OpenSSH daemon".to_string()),
                }]
            }
        );
    }

    #[test]
    fn control_service_reports_unknown_unit_errors() {
        let mut service = ControlService::new(ServiceManager::new());

        let response = service.handle_request(ControlRequest::Status {
            unit: Some("missing.service".to_string()),
        });

        assert_eq!(
            response,
            ControlResponse::Error {
                message: "unit missing.service was not found".to_string()
            }
        );
    }
}
