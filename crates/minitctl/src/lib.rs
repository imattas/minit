use clap::{Parser, Subcommand};
use minit_core::ipc::{ControlRequest, ControlResponse, UnitState};

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

pub fn run_with_args<I, S>(args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    if let Command::Status { unit } = cli.command {
        print!("{}", render_status_unavailable(unit.as_deref()));
    }
    0
}

pub fn render_status_unavailable(unit: Option<&str>) -> String {
    let socket = "/run/minit/minitd.sock";
    match unit {
        Some(unit) => format!(
            "unit: {unit}\nstate: unknown\nminitd unavailable: cannot connect to {socket}\n"
        ),
        None => format!("state: unknown\nminitd unavailable: cannot connect to {socket}\n"),
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
}
