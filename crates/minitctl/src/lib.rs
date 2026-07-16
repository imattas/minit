use clap::{Parser, Subcommand};

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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

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
}
