use minit_core::ipc::{
    decode_request, encode_response, ControlRequest, ControlResponse, WireError,
};
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
    use minit_core::ipc::{decode_response, encode_request};

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
}
