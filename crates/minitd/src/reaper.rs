use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReapEvent {
    pub pid: i32,
    pub status: ReapStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReapStatus {
    Exited(i32),
    Signaled(i32),
    StillAlive,
}

#[derive(Debug, Error)]
pub enum ReapError {
    #[error("wait failed: {0}")]
    Wait(String),
}

pub trait Reaper {
    fn reap_once(&mut self) -> Result<Option<ReapEvent>, ReapError>;
}

pub fn drain_reap_events<R: Reaper>(reaper: &mut R) -> Result<Vec<ReapEvent>, ReapError> {
    let mut events = Vec::new();

    while let Some(event) = reaper.reap_once()? {
        events.push(event);
    }

    Ok(events)
}

#[cfg(target_os = "linux")]
pub struct LinuxReaper;

#[cfg(target_os = "linux")]
impl Reaper for LinuxReaper {
    fn reap_once(&mut self) -> Result<Option<ReapEvent>, ReapError> {
        use nix::errno::Errno;
        use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
        use nix::unistd::Pid;

        match waitpid(Pid::from_raw(-1), Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => Ok(None),
            Ok(WaitStatus::Exited(pid, code)) => Ok(Some(ReapEvent {
                pid: pid.as_raw(),
                status: ReapStatus::Exited(code),
            })),
            Ok(WaitStatus::Signaled(pid, signal, _)) => Ok(Some(ReapEvent {
                pid: pid.as_raw(),
                status: ReapStatus::Signaled(signal as i32),
            })),
            Ok(_) => Ok(None),
            Err(Errno::ECHILD) => Ok(None),
            Err(error) => Err(ReapError::Wait(error.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeReaper {
        events: Vec<ReapEvent>,
    }

    impl Reaper for FakeReaper {
        fn reap_once(&mut self) -> Result<Option<ReapEvent>, ReapError> {
            Ok(self.events.pop())
        }
    }

    #[test]
    fn drain_reap_events_collects_until_empty() {
        let mut reaper = FakeReaper {
            events: vec![
                ReapEvent {
                    pid: 12,
                    status: ReapStatus::Exited(0),
                },
                ReapEvent {
                    pid: 11,
                    status: ReapStatus::Signaled(15),
                },
            ],
        };

        let events = drain_reap_events(&mut reaper).expect("reap drain should succeed");

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].pid, 11);
        assert_eq!(events[1].pid, 12);
    }
}
