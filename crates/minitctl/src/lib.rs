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
    let _cli = Cli::parse_from(args);
    0
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
}
