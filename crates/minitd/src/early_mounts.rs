use minit_core::boot::{default_early_mounts, EarlyMount};
use minit_core::diagnostics::DiagnosticEvent;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MountError {
    #[error("failed to create mount point {path}: {message}")]
    CreateDir { path: String, message: String },
    #[error("failed to mount {target}: {message}")]
    Mount { target: String, message: String },
}

pub trait MountExecutor {
    fn ensure_dir(&mut self, path: &str) -> Result<(), MountError>;
    fn mount(&mut self, spec: &EarlyMount) -> Result<(), MountError>;
}

pub fn ensure_early_mounts<E: MountExecutor>(
    executor: &mut E,
) -> Result<Vec<DiagnosticEvent>, MountError> {
    let mut events = Vec::new();

    for spec in default_early_mounts() {
        executor.ensure_dir(spec.target)?;
        executor.mount(&spec)?;
        events.push(DiagnosticEvent::new(
            "boot",
            format!("mounted {}", spec.target),
        ));
    }

    Ok(events)
}

#[cfg(target_os = "linux")]
pub struct LinuxMountExecutor;

#[cfg(target_os = "linux")]
impl MountExecutor for LinuxMountExecutor {
    fn ensure_dir(&mut self, path: &str) -> Result<(), MountError> {
        std::fs::create_dir_all(path).map_err(|error| MountError::CreateDir {
            path: path.to_string(),
            message: error.to_string(),
        })
    }

    fn mount(&mut self, spec: &EarlyMount) -> Result<(), MountError> {
        use nix::mount::{mount, MsFlags};
        use std::ffi::OsStr;

        mount(
            Some(OsStr::new(spec.source)),
            spec.target,
            Some(OsStr::new(spec.fstype)),
            MsFlags::from_bits_truncate(spec.flags),
            None::<&OsStr>,
        )
        .or_else(|error| {
            if error == nix::errno::Errno::EBUSY {
                Ok(())
            } else {
                Err(MountError::Mount {
                    target: spec.target.to_string(),
                    message: error.to_string(),
                })
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minit_core::boot::default_early_mounts;

    #[derive(Default)]
    struct FakeMountExecutor {
        calls: Vec<String>,
    }

    impl MountExecutor for FakeMountExecutor {
        fn ensure_dir(&mut self, path: &str) -> Result<(), MountError> {
            self.calls.push(format!("dir:{path}"));
            Ok(())
        }

        fn mount(&mut self, spec: &EarlyMount) -> Result<(), MountError> {
            self.calls
                .push(format!("mount:{}:{}", spec.fstype, spec.target));
            Ok(())
        }
    }

    #[test]
    fn ensure_early_mounts_creates_directories_before_mounting() {
        let mut executor = FakeMountExecutor::default();

        let events = ensure_early_mounts(&mut executor).expect("mounts should succeed");

        assert_eq!(executor.calls[0], "dir:/proc");
        assert_eq!(executor.calls[1], "mount:proc:/proc");
        assert_eq!(executor.calls.len(), default_early_mounts().len() * 2);
        assert!(events.iter().any(|event| event.message == "mounted /proc"));
    }
}
