use clap::{Parser, Subcommand};
#[cfg(target_os = "linux")]
use minit_core::ipc::{decode_response, encode_request};
use minit_core::ipc::{ControlRequest, ControlResponse, UnitState, DEFAULT_CONTROL_SOCKET};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Parser)]
#[command(name = "minitctl")]
pub struct Cli {
    #[arg(long, global = true)]
    pub socket: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    Status {
        unit: Option<String>,
    },
    List,
    Explain {
        unit: String,
    },
    Graph {
        #[arg(long)]
        json: bool,
        unit: String,
    },
    Events,
    Start {
        unit: String,
    },
    Stop {
        unit: String,
    },
    Restart {
        unit: String,
    },
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
    let socket = control_socket_for_cli(&cli).to_string();

    #[cfg(target_os = "linux")]
    {
        let mut transport = UnixSocketTransport::new(socket);
        run_with_transport(cli, &mut transport)
    }

    #[cfg(not(target_os = "linux"))]
    {
        let mut transport = UnavailableTransport::new(socket);
        run_with_transport(cli, &mut transport)
    }
}

pub fn control_socket_for_cli(cli: &Cli) -> &str {
    cli.socket.as_deref().unwrap_or(DEFAULT_CONTROL_SOCKET)
}

pub fn run_with_transport<T: ControlTransport>(cli: Cli, transport: &mut T) -> i32 {
    let command = cli.command;
    let request = command_to_request(command.clone());
    match transport.round_trip(&request) {
        Ok(response) => {
            let output = match command {
                Command::List => render_list_response(&response),
                Command::Graph { json: true, .. } => render_graph_json_response(&response),
                _ => render_response(&response),
            };
            print!("{output}");
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
        Command::List => ControlRequest::List,
        Command::Explain { unit } => ControlRequest::Explain { unit },
        Command::Graph { unit, .. } => ControlRequest::Graph { unit },
        Command::Events => ControlRequest::Events,
        Command::Start { unit } => ControlRequest::Start { unit },
        Command::Stop { unit } => ControlRequest::Stop { unit },
        Command::Restart { unit } => ControlRequest::Restart { unit },
    }
}

pub fn render_list_response(response: &ControlResponse) -> String {
    match response {
        ControlResponse::Status { units } => {
            if units.is_empty() {
                return "no units\n".to_string();
            }

            let mut output = String::new();
            for unit in units {
                let pid = unit
                    .main_pid
                    .map(|pid| pid.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let description = unit.description.as_deref().unwrap_or("-");
                output.push_str(&format!(
                    "{}\t{}\t{}\t{}\n",
                    unit.unit,
                    render_unit_state(&unit.state),
                    pid,
                    description
                ));
            }
            output
        }
        other => render_response(other),
    }
}

pub fn render_graph_json_response(response: &ControlResponse) -> String {
    match response {
        ControlResponse::Graph { unit, batches } => {
            let mut output = serde_json::json!({
                "unit": unit,
                "batches": batches,
            })
            .to_string();
            output.push('\n');
            output
        }
        other => render_response(other),
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
                output.push_str(&format!("restart_attempts: {}\n", unit.restart_attempts));
                if let Some(last_exit_status) = &unit.last_exit_status {
                    output.push_str(&format!("last_exit_status: {last_exit_status}\n"));
                }
                if let Some(cgroup_path) = &unit.cgroup_path {
                    output.push_str(&format!("cgroup_path: {cgroup_path}\n"));
                }
            }
            output
        }
        ControlResponse::Explanation { unit, lines } => {
            let mut output = format!("unit: {unit}\n");
            for line in lines {
                output.push_str(&format!("explain: {line}\n"));
            }
            output
        }
        ControlResponse::Graph { unit, batches } => {
            let mut output = format!("unit: {unit}\n");
            for (index, batch) in batches.iter().enumerate() {
                output.push_str(&format!("batch {}: {}\n", index + 1, batch.join(", ")));
            }
            output
        }
        ControlResponse::Events { events } => {
            if events.is_empty() {
                return "no events\n".to_string();
            }
            let mut output = String::new();
            for (index, event) in events.iter().enumerate() {
                if index > 0 {
                    output.push('\n');
                }
                output.push_str(&format!("event: {}\n", event.sequence));
                output.push_str(&format!("scope: {}\n", event.scope));
                output.push_str(&format!("message: {}\n", event.message));
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
    fn parses_socket_override() {
        let cli = Cli::parse_from(["minitctl", "--socket", "/tmp/minit.sock", "status"]);

        assert_eq!(cli.socket.as_deref(), Some("/tmp/minit.sock"));
        assert_eq!(cli.command, Command::Status { unit: None });
    }

    #[test]
    fn socket_override_replaces_default_control_socket() {
        let cli = Cli::parse_from(["minitctl", "--socket", "/tmp/minit.sock", "status"]);

        assert_eq!(control_socket_for_cli(&cli), "/tmp/minit.sock");
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
    fn parses_list_command() {
        let cli = Cli::parse_from(["minitctl", "list"]);

        assert_eq!(cli.command, Command::List);
    }

    #[test]
    fn parses_explain_command() {
        let cli = Cli::parse_from(["minitctl", "explain", "multi-user.target"]);

        assert_eq!(
            cli.command,
            Command::Explain {
                unit: "multi-user.target".to_string()
            }
        );
    }

    #[test]
    fn parses_graph_command() {
        let cli = Cli::parse_from(["minitctl", "graph", "multi-user.target"]);

        assert_eq!(
            cli.command,
            Command::Graph {
                json: false,
                unit: "multi-user.target".to_string()
            }
        );
    }

    #[test]
    fn parses_graph_json_command() {
        let cli = Cli::parse_from(["minitctl", "graph", "--json", "multi-user.target"]);

        assert_eq!(
            cli.command,
            Command::Graph {
                json: true,
                unit: "multi-user.target".to_string()
            }
        );
    }

    #[test]
    fn parses_events_command() {
        let cli = Cli::parse_from(["minitctl", "events"]);

        assert_eq!(cli.command, Command::Events);
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
        assert_eq!(command_to_request(Command::List), ControlRequest::List);
        assert_eq!(
            command_to_request(Command::Explain {
                unit: "sshd".to_string()
            }),
            ControlRequest::Explain {
                unit: "sshd".to_string()
            }
        );
        assert_eq!(
            command_to_request(Command::Graph {
                json: true,
                unit: "multi-user.target".to_string()
            }),
            ControlRequest::Graph {
                unit: "multi-user.target".to_string()
            }
        );
        assert_eq!(command_to_request(Command::Events), ControlRequest::Events);
    }

    #[test]
    fn renders_status_response() {
        let response = ControlResponse::Status {
            units: vec![UnitStatus {
                unit: "sshd".to_string(),
                state: UnitState::Active,
                main_pid: Some(123),
                description: Some("OpenSSH daemon".to_string()),
                restart_attempts: 2,
                last_exit_status: Some("exit 1".to_string()),
                cgroup_path: Some("/sys/fs/cgroup/minit/sshd".to_string()),
            }],
        };

        let output = render_response(&response);

        assert!(output.contains("unit: sshd"));
        assert!(output.contains("state: active"));
        assert!(output.contains("main_pid: 123"));
        assert!(output.contains("description: OpenSSH daemon"));
        assert!(output.contains("restart_attempts: 2"));
        assert!(output.contains("last_exit_status: exit 1"));
        assert!(output.contains("cgroup_path: /sys/fs/cgroup/minit/sshd"));
    }

    #[test]
    fn renders_list_response_compactly() {
        let response = ControlResponse::Status {
            units: vec![
                UnitStatus {
                    unit: "getty.service".to_string(),
                    state: UnitState::Inactive,
                    main_pid: None,
                    description: Some("Console login".to_string()),
                    restart_attempts: 0,
                    last_exit_status: None,
                    cgroup_path: None,
                },
                UnitStatus {
                    unit: "sshd.service".to_string(),
                    state: UnitState::Active,
                    main_pid: Some(42),
                    description: Some("OpenSSH daemon".to_string()),
                    restart_attempts: 0,
                    last_exit_status: None,
                    cgroup_path: None,
                },
            ],
        };

        let output = render_list_response(&response);

        assert_eq!(
            output,
            "getty.service\tinactive\t-\tConsole login\nsshd.service\tactive\t42\tOpenSSH daemon\n"
        );
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
    fn renders_explanation_response() {
        let response = ControlResponse::Explanation {
            unit: "multi-user.target".to_string(),
            lines: vec![
                "kind: target".to_string(),
                "wants: getty.service".to_string(),
            ],
        };

        let output = render_response(&response);

        assert!(output.contains("unit: multi-user.target"));
        assert!(output.contains("explain: kind: target"));
        assert!(output.contains("explain: wants: getty.service"));
    }

    #[test]
    fn renders_graph_response_by_start_batch() {
        let response = ControlResponse::Graph {
            unit: "multi-user.target".to_string(),
            batches: vec![
                vec!["network.service".to_string(), "demo-sleep".to_string()],
                vec!["multi-user.target".to_string()],
            ],
        };

        assert_eq!(
            render_response(&response),
            "unit: multi-user.target\nbatch 1: network.service, demo-sleep\nbatch 2: multi-user.target\n"
        );
    }

    #[test]
    fn renders_graph_response_as_json() {
        let response = ControlResponse::Graph {
            unit: "multi-user.target".to_string(),
            batches: vec![
                vec!["network.service".to_string(), "demo-sleep".to_string()],
                vec!["multi-user.target".to_string()],
            ],
        };

        let output = render_graph_json_response(&response);
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(json["unit"], "multi-user.target");
        assert_eq!(json["batches"][0][0], "network.service");
        assert_eq!(json["batches"][0][1], "demo-sleep");
        assert_eq!(json["batches"][1][0], "multi-user.target");
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn renders_events_response() {
        let response = ControlResponse::Events {
            events: vec![minit_core::diagnostics::DiagnosticEvent::sequenced(
                7,
                "runtime",
                "mounted var-log.mount",
            )],
        };

        let output = render_response(&response);

        assert!(output.contains("event: 7"));
        assert!(output.contains("scope: runtime"));
        assert!(output.contains("message: mounted var-log.mount"));
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
