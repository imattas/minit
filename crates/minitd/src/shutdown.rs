use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownAction {
    Halt,
    Poweroff,
    Reboot,
}

#[derive(Debug, Error)]
pub enum ShutdownError {
    #[error("sync failed: {0}")]
    Sync(String),
    #[error("reboot syscall failed: {0}")]
    Reboot(String),
}

pub trait ShutdownExecutor {
    fn sync_filesystems(&mut self) -> Result<(), ShutdownError>;
    fn reboot(&mut self, action: ShutdownAction) -> Result<(), ShutdownError>;
}

pub fn perform_shutdown<E: ShutdownExecutor>(
    executor: &mut E,
    action: ShutdownAction,
) -> Result<(), ShutdownError> {
    executor.sync_filesystems()?;
    executor.reboot(action)
}

#[cfg(target_os = "linux")]
pub struct LinuxShutdownExecutor;

#[cfg(target_os = "linux")]
impl ShutdownExecutor for LinuxShutdownExecutor {
    fn sync_filesystems(&mut self) -> Result<(), ShutdownError> {
        unsafe {
            libc::sync();
        }
        Ok(())
    }

    fn reboot(&mut self, action: ShutdownAction) -> Result<(), ShutdownError> {
        let command = match action {
            ShutdownAction::Halt => libc::RB_HALT_SYSTEM,
            ShutdownAction::Poweroff => libc::RB_POWER_OFF,
            ShutdownAction::Reboot => libc::RB_AUTOBOOT,
        };

        let result = unsafe { libc::reboot(command) };
        if result == 0 {
            Ok(())
        } else {
            Err(ShutdownError::Reboot(
                std::io::Error::last_os_error().to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeShutdownExecutor {
        calls: Vec<String>,
    }

    impl ShutdownExecutor for FakeShutdownExecutor {
        fn sync_filesystems(&mut self) -> Result<(), ShutdownError> {
            self.calls.push("sync".to_string());
            Ok(())
        }

        fn reboot(&mut self, action: ShutdownAction) -> Result<(), ShutdownError> {
            self.calls.push(format!("reboot:{action:?}"));
            Ok(())
        }
    }

    #[test]
    fn shutdown_syncs_before_poweroff() {
        let mut executor = FakeShutdownExecutor::default();

        perform_shutdown(&mut executor, ShutdownAction::Poweroff).expect("shutdown should succeed");

        assert_eq!(executor.calls, vec!["sync", "reboot:Poweroff"]);
    }
}
