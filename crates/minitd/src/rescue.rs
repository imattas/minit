use minit_core::boot::RescueConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RescueCommand {
    pub argv: Vec<String>,
    pub fallback_used: bool,
}

pub fn select_rescue_command(config: &RescueConfig, candidates: &[&str]) -> RescueCommand {
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
    use minit_core::boot::RescueConfig;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    let mut mount_executor = LinuxMountExecutor;
    if let Err(error) = ensure_early_mounts(&mut mount_executor) {
        eprintln!("minitd: early mount failed: {error}");
    }

    let config = RescueConfig::default();
    let candidates = existing_rescue_candidates();
    let command = select_rescue_command(&config, &candidates);

    let child_result = Command::new(&command.argv[0]).args(&command.argv[1..]).spawn();
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
        if let Err(error) = drain_reap_events(&mut reaper) {
            eprintln!("minitd: reap failed: {error}");
        }

        match child.try_wait() {
            Ok(Some(status)) => return status.code().unwrap_or(0),
            Ok(None) => thread::sleep(Duration::from_millis(100)),
            Err(error) => {
                eprintln!("minitd: failed to observe rescue command: {error}");
                return 1;
            }
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
}
