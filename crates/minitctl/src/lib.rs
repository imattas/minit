use clap::{Parser, Subcommand};
#[cfg(target_os = "linux")]
use minit_core::ipc::{decode_response, encode_request};
use minit_core::ipc::{ControlRequest, ControlResponse, UnitState, DEFAULT_CONTROL_SOCKET};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Parser)]
#[command(name = "minitctl")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    Status { unit: Option<String> },
    Start { unit: String },
    Stop { unit: String },
    Restart { unit: String },
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("cannot connect to {socket}: {source}")]
    Connect {
        socket: String,
        source: std::io::Error,
    },
    #[error("control protocol error: {0}")]
    Protocol(String),
}

pub trait ControlTransport {
    fn round_trip(&mut self, request: &ControlRequest) -> Result<ControlResponse, ClientError>;
}

pub fn run_with_args<I, S>(args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    run_cli(cli)
}

pub fn run_cli(cli: Cli) -> i32 {
    #[cfg(target_os = "linux")]
    {
        let mut transport = UnixSocketTransport::new(DEFAULT_CONTROL_SOCKET);
        run_with_transport(cli, &mut transport)
    }

    #[cfg(not(target_os = "linux"))]
    {
        let mut transport = UnavailableTransport::new(DEFAULT_CONTROL_SOCKET);
        run_with_transport(cli, &mut transport)
    }
}

pub fn run_with_transport<T: ControlTransport>(cli: Cli, transport: &mut T) -> i32 {
    let request = command_to_request(cli.command);
    match transport.round_trip(&request) {
        Ok(response) => {
            print!("{}", render_response(&response));
            0
        }
        Err(err) => {
            eprintln!("minitd unavailable: {err}");
            1
        }
    }
}

pub fn render_status_unavailable(unit: Option<&str>) -> String {
    let socket = DEFAULT_CONTROL_SOCKET;
    match unit {
        Some(unit) => format!(
            "unit: {unit}\nstate: unknown\nminitd unavailable: cannot connect to {socket}\n"
        ),
        None => format!("state: unknown\nminitd unavailable: cannot connect to {socket}\n"),
    }
}

#[cfg(target_os = "linux")]
pub struct UnixSocketTransport {
    socket: String,
}

#[cfg(target_os = "linux")]
impl UnixSocketTransport {
    pub fn new(socket: impl Into<String>) -> Self {
        Self {
            socket: socket.into(),
        }
    }
}

#[cfg(target_os = "linux")]
impl ControlTransport for UnixSocketTransport {
    fn round_trip(&mut self, request: &ControlRequest) -> Result<ControlResponse, ClientError> {
        use std::io::{BufRead, BufReader, Write};
        use std::os::unix::net::UnixStream;

        let mut stream =
            UnixStream::connect(&self.socket).map_err(|source| ClientError::Connect {
                socket: self.socket.clone(),
                source,
            })?;
        let request_line =
            encode_request(request).map_err(|err| ClientError::Protocol(err.to_string()))?;
        stream
            .write_all(request_line.as_bytes())
            .map_err(|err| ClientError::Protocol(err.to_string()))?;
        stream
            .flush()
            .map_err(|err| ClientError::Protocol(err.to_string()))?;

        let mut response_line = String::new();
        let mut reader = BufReader::new(stream);
        reader
            .read_line(&mut response_line)
            .map_err(|err| ClientError::Protocol(err.to_string()))?;

        decode_response(&response_line).map_err(|err| ClientError::Protocol(err.to_string()))
    }
}

#[cfg(not(target_os = "linux"))]
pub struct UnavailableTransport {
    socket: String,
}

#[cfg(not(target_os = "linux"))]
impl UnavailableTransport {
    pub fn new(socket: impl Into<String>) -> Self {
        Self {
            socket: socket.into(),
        }
    }
}

#[cfg(not(target_os = "linux"))]
impl ControlTransport for UnavailableTransport {
    fn round_trip(&mut self, _request: &ControlRequest) -> Result<ControlResponse, ClientError> {
        Err(ClientError::Connect {
            socket: self.socket.clone(),
            source: std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "minitctl control sockets are Linux-only",
            ),
        })
    }
}

pub fn command_to_request(command: Command) -> ControlRequest {
    match command {
        Command::Status { unit } => ControlRequest::Status { unit },
        Command::Start { unit } => ControlRequest::Start { unit },
        Command::Stop { unit } => ControlRequest::Stop { unit },
        Command::Restart { unit } => ControlRequest::Restart { unit },
    }
}

pub fn render_response(response: &ControlResponse) -> String {
    match response {
        ControlResponse::Status { units } => {
            if units.is_empty() {
                return "no units\n".to_string();
            }

            let mut output = String::new();
            for (index, unit) in units.iter().enumerate() {
                if index > 0 {
                    output.push('\n');
                }
                output.push_str(&format!("unit: {}\n", unit.unit));
                output.push_str(&format!("state: {}\n", render_unit_state(&unit.state)));
                if let Some(main_pid) = unit.main_pid {
                    output.push_str(&format!("main_pid: {main_pid}\n"));
                }
                if let Some(description) = &unit.description {
                    output.push_str(&format!("description: {description}\n"));
                }
            }
            output
        }
        ControlResponse::Accepted { message } => format!("accepted: {message}\n"),
        ControlResponse::Error { message } => format!("error: {message}\n"),
    }
}

fn render_unit_state(state: &UnitState) -> &'static str {
    match state {
        UnitState::Unknown => "unknown",
        UnitState::Inactive => "inactive",
        UnitState::Starting => "starting",
        UnitState::Active => "active",
        UnitState::Stopping => "stopping",
        UnitState::Failed => "failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use minit_core::ipc::UnitStatus;

    #[derive(Default)]
    struct MockTransport {
        requests: Vec<ControlRequest>,
        response: Option<ControlResponse>,
    }

    impl ControlTransport for MockTransport {
        fn round_trip(&mut self, request: &ControlRequest) -> Result<ControlResponse, ClientError> {
            self.requests.push(request.clone());
            Ok(self.response.clone().unwrap_or(ControlResponse::Accepted {
                message: "ok".to_string(),
            }))
        }
    }

    #[test]
    fn parses_status_without_unit() {
        let cli = Cli::parse_from(["minitctl", "status"]);

        assert_eq!(cli.command, Command::Status { unit: None });
    }

    #[test]
    fn parses_status_with_unit() {
        let cli = Cli::parse_from(["minitctl", "status", "sshd"]);

        assert_eq!(
            cli.command,
            Command::Status {
                unit: Some("sshd".to_string())
            }
        );
    }

    #[test]
    fn parses_lifecycle_commands() {
        assert_eq!(
            Cli::parse_from(["minitctl", "start", "sshd"]).command,
            Command::Start {
                unit: "sshd".to_string()
            }
        );
        assert_eq!(
            Cli::parse_from(["minitctl", "stop", "sshd"]).command,
            Command::Stop {
                unit: "sshd".to_string()
            }
        );
        assert_eq!(
            Cli::parse_from(["minitctl", "restart", "sshd"]).command,
            Command::Restart {
                unit: "sshd".to_string()
            }
        );
    }

    #[test]
    fn renders_global_status_when_minitd_is_unavailable() {
        let output = render_status_unavailable(None);

        assert!(output.contains("minitd unavailable"));
        assert!(output.contains("/run/minit/minitd.sock"));
    }

    #[test]
    fn renders_unit_status_when_minitd_is_unavailable() {
        let output = render_status_unavailable(Some("sshd"));

        assert!(output.contains("sshd"));
        assert!(output.contains("minitd unavailable"));
    }

    #[test]
    fn maps_cli_commands_to_control_requests() {
        assert_eq!(
            command_to_request(Command::Status {
                unit: Some("sshd".to_string())
            }),
            ControlRequest::Status {
                unit: Some("sshd".to_string())
            }
        );
        assert_eq!(
            command_to_request(Command::Restart {
                unit: "sshd".to_string()
            }),
            ControlRequest::Restart {
                unit: "sshd".to_string()
            }
        );
    }

    #[test]
    fn renders_status_response() {
        let response = ControlResponse::Status {
            units: vec![UnitStatus {
                unit: "sshd".to_string(),
                state: UnitState::Active,
                main_pid: Some(123),
                description: Some("OpenSSH daemon".to_string()),
            }],
        };

        let output = render_response(&response);

        assert!(output.contains("unit: sshd"));
        assert!(output.contains("state: active"));
        assert!(output.contains("main_pid: 123"));
        assert!(output.contains("description: OpenSSH daemon"));
    }

    #[test]
    fn renders_accepted_response() {
        let response = ControlResponse::Accepted {
            message: "queued restart for sshd".to_string(),
        };

        assert_eq!(
            render_response(&response),
            "accepted: queued restart for sshd\n"
        );
    }

    #[test]
    fn run_with_transport_sends_request_and_returns_success() {
        let cli = Cli::parse_from(["minitctl", "start", "sshd"]);
        let mut transport = MockTransport {
            response: Some(ControlResponse::Accepted {
                message: "queued start for sshd".to_string(),
            }),
            ..MockTransport::default()
        };

        let code = run_with_transport(cli, &mut transport);

        assert_eq!(code, 0);
        assert_eq!(
            transport.requests,
            vec![ControlRequest::Start {
                unit: "sshd".to_string()
            }]
        );
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn run_cli_uses_unavailable_transport_off_linux() {
        let cli = Cli::parse_from(["minitctl", "status"]);

        let code = run_cli(cli);

        assert_eq!(code, 1);
    }
}
