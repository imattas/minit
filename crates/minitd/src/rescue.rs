use minit_core::boot::RescueConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RescueCommand {
    pub argv: Vec<String>,
    pub fallback_used: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RescueExitAction {
    Return(i32),
    Poweroff,
}

pub fn rescue_child_exit_action(is_pid_one: bool, child_code: i32) -> RescueExitAction {
    if is_pid_one {
        RescueExitAction::Poweroff
    } else {
        RescueExitAction::Return(child_code)
    }
}

pub fn select_rescue_command(config: &RescueConfig, candidates: &[&str]) -> RescueCommand {
    select_rescue_command_for_cmdline(config, candidates, "")
}

pub fn select_rescue_command_for_cmdline(
    config: &RescueConfig,
    candidates: &[&str],
    kernel_cmdline: &str,
) -> RescueCommand {
    if kernel_cmdline
        .split_whitespace()
        .any(|arg| arg == "minit.rescue.autoshutdown=1")
        && candidates.contains(&"/bin/sh")
    {
        return RescueCommand {
            argv: vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "exit 0".to_string(),
            ],
            fallback_used: true,
        };
    }

    if let Some(program) = config.command.first() {
        if candidates.iter().any(|candidate| candidate == program) {
            return RescueCommand {
                argv: config.command.clone(),
                fallback_used: false,
            };
        }
    }

    if candidates.contains(&"/sbin/getty") {
        return RescueCommand {
            argv: vec!["/sbin/getty".to_string(), "console".to_string()],
            fallback_used: true,
        };
    }

    RescueCommand {
        argv: vec!["/bin/sh".to_string()],
        fallback_used: true,
    }
}

#[cfg(target_os = "linux")]
pub fn existing_rescue_candidates() -> Vec<&'static str> {
    ["/bin/sh", "/sbin/getty"]
        .into_iter()
        .filter(|path| std::path::Path::new(path).exists())
        .collect()
}

#[cfg(not(target_os = "linux"))]
pub fn existing_rescue_candidates() -> Vec<&'static str> {
    Vec::new()
}

#[cfg(target_os = "linux")]
pub fn run_linux_rescue() -> i32 {
    use crate::early_mounts::{ensure_early_mounts, LinuxMountExecutor};
    use crate::reaper::{drain_reap_events, LinuxReaper};
    use crate::shutdown::{perform_shutdown, LinuxShutdownExecutor, ShutdownAction};
    use minit_core::boot::RescueConfig;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    let mut mount_executor = LinuxMountExecutor;
    if let Err(error) = ensure_early_mounts(&mut mount_executor) {
        eprintln!("minitd: early mount failed: {error}");
    }

    let kernel_cmdline = std::fs::read_to_string("/proc/cmdline").unwrap_or_default();

    if kernel_cmdline
        .split_whitespace()
        .any(|arg| arg == "minit.rescue.autoshutdown=1")
    {
        let mut shutdown = LinuxShutdownExecutor;
        if let Err(error) = perform_shutdown(&mut shutdown, ShutdownAction::Poweroff) {
            eprintln!("minitd: failed to power off during autoshutdown smoke: {error}");
            return 1;
        }
    }

    let config = RescueConfig::default();
    let candidates = existing_rescue_candidates();
    let command = select_rescue_command_for_cmdline(&config, &candidates, &kernel_cmdline);

    let child_result = Command::new(&command.argv[0])
        .args(&command.argv[1..])
        .spawn();
    let mut child = match child_result {
        Ok(child) => child,
        Err(error) => {
            eprintln!(
                "minitd: failed to start rescue command {:?}: {error}",
                command.argv
            );
            return 1;
        }
    };

    let mut reaper = LinuxReaper;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let child_code = status.code().unwrap_or(0);
                match rescue_child_exit_action(std::process::id() == 1, child_code) {
                    RescueExitAction::Return(code) => return code,
                    RescueExitAction::Poweroff => {
                        let mut shutdown = LinuxShutdownExecutor;
                        if let Err(error) =
                            perform_shutdown(&mut shutdown, ShutdownAction::Poweroff)
                        {
                            eprintln!("minitd: failed to power off after rescue exit: {error}");
                            return 1;
                        }
                    }
                }
            }
            Ok(None) => thread::sleep(Duration::from_millis(100)),
            Err(error) => {
                eprintln!("minitd: failed to observe rescue command: {error}");
                return 1;
            }
        }

        if let Err(error) = drain_reap_events(&mut reaper) {
            eprintln!("minitd: reap failed: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minit_core::boot::RescueConfig;

    #[test]
    fn selects_configured_shell_when_available() {
        let config = RescueConfig::default();

        let command = select_rescue_command(&config, &["/bin/sh"]);

        assert_eq!(command.argv, vec!["/bin/sh"]);
        assert!(!command.fallback_used);
    }

    #[test]
    fn falls_back_to_getty_when_shell_missing() {
        let config = RescueConfig::default();

        let command = select_rescue_command(&config, &["/sbin/getty"]);

        assert_eq!(command.argv, vec!["/sbin/getty", "console"]);
        assert!(command.fallback_used);
    }

    #[test]
    fn pid_one_powers_off_when_rescue_child_exits() {
        assert_eq!(
            rescue_child_exit_action(true, 0),
            RescueExitAction::Poweroff
        );
        assert_eq!(
            rescue_child_exit_action(false, 7),
            RescueExitAction::Return(7)
        );
    }

    #[test]
    fn autoshutdown_cmdline_selects_exit_command_for_vm_smoke() {
        let config = RescueConfig::default();

        let command = select_rescue_command_for_cmdline(
            &config,
            &["/bin/sh"],
            "console=ttyS0 minit.rescue.autoshutdown=1",
        );

        assert_eq!(command.argv, vec!["/bin/sh", "-c", "exit 0"]);
        assert!(command.fallback_used);
    }
}
