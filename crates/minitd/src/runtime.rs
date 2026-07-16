use crate::cgroups::{CgroupError, CgroupFs, CgroupManager};
use crate::control::ControlRuntime;
use crate::reaper::{drain_reap_events, ReapStatus, Reaper};
use minit_core::manager::{ServiceManager, ServiceManagerError};
use std::time::Duration;
use thiserror::Error;

pub trait ProcessSpawner {
    fn spawn(&mut self, argv: &[String]) -> Result<u32, SpawnError>;
}

#[derive(Default)]
pub struct NoopReaper;

impl Reaper for NoopReaper {
    fn reap_once(&mut self) -> Result<Option<crate::reaper::ReapEvent>, crate::reaper::ReapError> {
        Ok(None)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("failed to spawn service process: {0}")]
pub struct SpawnError(pub String);

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("service manager error: {0}")]
    Manager(#[from] ServiceManagerError),
    #[error("cgroup error: {0}")]
    Cgroup(#[from] CgroupError),
    #[error("spawn error: {0}")]
    Spawn(#[from] SpawnError),
    #[error("cgroup for {0} did not become empty after kill")]
    CgroupStillPopulated(String),
}

pub fn start_service<F, P>(
    services: &mut ServiceManager,
    cgroups: &CgroupManager,
    cgroup_fs: &mut F,
    spawner: &mut P,
    unit: &str,
) -> Result<u32, RuntimeError>
where
    F: CgroupFs,
    P: ProcessSpawner,
{
    start_service_inner(services, cgroups, cgroup_fs, spawner, unit, false)
}

fn restart_service<F, P>(
    services: &mut ServiceManager,
    cgroups: &CgroupManager,
    cgroup_fs: &mut F,
    spawner: &mut P,
    unit: &str,
) -> Result<u32, RuntimeError>
where
    F: CgroupFs,
    P: ProcessSpawner,
{
    start_service_inner(services, cgroups, cgroup_fs, spawner, unit, true)
}

fn start_service_inner<F, P>(
    services: &mut ServiceManager,
    cgroups: &CgroupManager,
    cgroup_fs: &mut F,
    spawner: &mut P,
    unit: &str,
    is_restart: bool,
) -> Result<u32, RuntimeError>
where
    F: CgroupFs,
    P: ProcessSpawner,
{
    let plan = if is_restart {
        services.plan_restart(unit)?
    } else {
        services.plan_start(unit)?
    };

    if let Err(error) = cgroups.create_unit(cgroup_fs, unit) {
        let _ = services.mark_failed(unit);
        return Err(error.into());
    }

    let pid = match spawner.spawn(&plan.argv) {
        Ok(pid) => pid,
        Err(error) => {
            let _ = services.mark_failed(unit);
            return Err(error.into());
        }
    };

    if let Err(error) = cgroups.attach_pid(cgroup_fs, unit, pid) {
        let _ = services.mark_failed(unit);
        return Err(error.into());
    }

    services.mark_active(unit, pid)?;
    Ok(pid)
}

pub fn stop_service<F>(
    services: &mut ServiceManager,
    cgroups: &CgroupManager,
    cgroup_fs: &mut F,
    unit: &str,
) -> Result<(), RuntimeError>
where
    F: CgroupFs,
{
    cgroups.kill_unit(cgroup_fs, unit)?;
    wait_until_cgroup_empty(cgroups, cgroup_fs, unit)?;
    cgroups.remove_unit(cgroup_fs, unit)?;
    services.mark_inactive(unit)?;
    Ok(())
}

fn wait_until_cgroup_empty<F>(
    cgroups: &CgroupManager,
    cgroup_fs: &mut F,
    unit: &str,
) -> Result<(), RuntimeError>
where
    F: CgroupFs,
{
    for attempt in 0..20 {
        if cgroups.unit_is_empty(cgroup_fs, unit)? {
            return Ok(());
        }
        if attempt < 19 {
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    Err(RuntimeError::CgroupStillPopulated(unit.to_string()))
}

pub struct ServiceRuntime<F, P, R = NoopReaper> {
    cgroups: CgroupManager,
    cgroup_fs: F,
    spawner: P,
    reaper: R,
}

impl<F, P> ServiceRuntime<F, P, NoopReaper> {
    pub fn new(cgroups: CgroupManager, cgroup_fs: F, spawner: P) -> Self {
        Self {
            cgroups,
            cgroup_fs,
            spawner,
            reaper: NoopReaper,
        }
    }
}

impl<F, P, R> ServiceRuntime<F, P, R> {
    pub fn with_reaper(cgroups: CgroupManager, cgroup_fs: F, spawner: P, reaper: R) -> Self {
        Self {
            cgroups,
            cgroup_fs,
            spawner,
            reaper,
        }
    }
}

impl<F, P, R> ControlRuntime for ServiceRuntime<F, P, R>
where
    F: CgroupFs,
    P: ProcessSpawner,
    R: Reaper,
{
    fn start(&mut self, services: &mut ServiceManager, unit: &str) -> Result<String, String> {
        let pid = start_service(
            services,
            &self.cgroups,
            &mut self.cgroup_fs,
            &mut self.spawner,
            unit,
        )
        .map_err(|err| err.to_string())?;
        Ok(format!("started {unit} as pid {pid}"))
    }

    fn stop(&mut self, services: &mut ServiceManager, unit: &str) -> Result<String, String> {
        stop_service(services, &self.cgroups, &mut self.cgroup_fs, unit)
            .map_err(|err| err.to_string())?;
        Ok(format!("stopped {unit}"))
    }

    fn reap(&mut self, services: &mut ServiceManager) -> Result<(), String> {
        let events = drain_reap_events(&mut self.reaper).map_err(|err| err.to_string())?;
        for event in events {
            let successful = matches!(event.status, ReapStatus::Exited(0));
            let Some(decision) = services
                .record_exit(event.pid as u32, successful)
                .map_err(|err| err.to_string())?
            else {
                continue;
            };

            let _ = wait_until_cgroup_empty(&self.cgroups, &mut self.cgroup_fs, &decision.unit);
            let _ = self
                .cgroups
                .remove_unit(&mut self.cgroup_fs, &decision.unit);

            if decision.restart {
                let new_pid = restart_service(
                    services,
                    &self.cgroups,
                    &mut self.cgroup_fs,
                    &mut self.spawner,
                    &decision.unit,
                )
                .map_err(|err| err.to_string())?;
                eprintln!(
                    "minitd: restarted {} after pid {} exit as pid {}",
                    decision.unit, event.pid, new_pid
                );
            }
        }
        Ok(())
    }

    fn shutdown(&mut self, services: &mut ServiceManager) -> Result<(), String> {
        let units = services.active_unit_names();
        for unit in units {
            stop_service(services, &self.cgroups, &mut self.cgroup_fs, &unit)
                .map_err(|err| err.to_string())?;
            eprintln!("minitd: stopped {unit} for shutdown");
        }
        Ok(())
    }
}

pub struct CommandSpawner;

impl ProcessSpawner for CommandSpawner {
    fn spawn(&mut self, argv: &[String]) -> Result<u32, SpawnError> {
        let Some(program) = argv.first() else {
            return Err(SpawnError("empty argv".to_string()));
        };
        let child = std::process::Command::new(program)
            .args(&argv[1..])
            .spawn()
            .map_err(|err| SpawnError(err.to_string()))?;
        Ok(child.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cgroups::CgroupFs;
    use minit_core::ipc::UnitState;
    use minit_core::unit::parse_unit_toml;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    #[derive(Default)]
    struct FakeCgroupFs {
        dirs: BTreeSet<PathBuf>,
        reads: BTreeMap<PathBuf, String>,
        removed: BTreeSet<PathBuf>,
        writes: BTreeMap<PathBuf, String>,
    }

    impl CgroupFs for FakeCgroupFs {
        fn create_dir_all(&mut self, path: &Path) -> Result<(), CgroupError> {
            self.dirs.insert(path.to_path_buf());
            Ok(())
        }

        fn read_to_string(&mut self, path: &Path) -> Result<String, CgroupError> {
            self.reads
                .get(path)
                .cloned()
                .ok_or_else(|| CgroupError::Fs(format!("missing fake read {}", path.display())))
        }

        fn remove_dir(&mut self, path: &Path) -> Result<(), CgroupError> {
            self.removed.insert(path.to_path_buf());
            Ok(())
        }

        fn write(&mut self, path: &Path, value: &str) -> Result<(), CgroupError> {
            self.writes.insert(path.to_path_buf(), value.to_string());
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeSpawner {
        argv: Vec<Vec<String>>,
        next_pid: u32,
    }

    impl ProcessSpawner for FakeSpawner {
        fn spawn(&mut self, argv: &[String]) -> Result<u32, SpawnError> {
            self.argv.push(argv.to_vec());
            Ok(self.next_pid)
        }
    }

    fn manager_with_sshd() -> ServiceManager {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "sshd.service"

[exec]
start = ["/usr/bin/sshd", "-D"]
"#,
        )
        .unwrap();
        let mut manager = ServiceManager::new();
        manager.add_unit(unit).unwrap();
        manager
    }

    #[test]
    fn start_service_creates_cgroup_spawns_attaches_and_marks_active() {
        let mut services = manager_with_sshd();
        let cgroups = CgroupManager::new("/sys/fs/cgroup/minit");
        let mut cgroup_fs = FakeCgroupFs::default();
        let mut spawner = FakeSpawner {
            next_pid: 321,
            ..FakeSpawner::default()
        };

        let pid = start_service(
            &mut services,
            &cgroups,
            &mut cgroup_fs,
            &mut spawner,
            "sshd.service",
        )
        .unwrap();

        assert_eq!(pid, 321);
        assert_eq!(spawner.argv, vec![vec!["/usr/bin/sshd", "-D"]]);
        assert!(cgroup_fs
            .dirs
            .contains(Path::new("/sys/fs/cgroup/minit/sshd.service")));
        assert_eq!(
            cgroup_fs
                .writes
                .get(Path::new("/sys/fs/cgroup/minit/sshd.service/cgroup.procs")),
            Some(&"321\n".to_string())
        );

        let status = services.status(Some("sshd.service")).unwrap();
        assert_eq!(status[0].state, UnitState::Active);
        assert_eq!(status[0].main_pid, Some(321));
    }

    #[test]
    fn stop_service_kills_cgroup_and_marks_inactive() {
        let mut services = manager_with_sshd();
        services.plan_start("sshd.service").unwrap();
        services.mark_active("sshd.service", 321).unwrap();
        let cgroups = CgroupManager::new("/sys/fs/cgroup/minit");
        let mut cgroup_fs = FakeCgroupFs::default();
        cgroup_fs.reads.insert(
            PathBuf::from("/sys/fs/cgroup/minit/sshd.service/cgroup.events"),
            "populated 0\nfrozen 0\n".to_string(),
        );

        stop_service(&mut services, &cgroups, &mut cgroup_fs, "sshd.service").unwrap();

        assert_eq!(
            cgroup_fs
                .writes
                .get(Path::new("/sys/fs/cgroup/minit/sshd.service/cgroup.kill")),
            Some(&"1\n".to_string())
        );
        assert!(cgroup_fs
            .removed
            .contains(Path::new("/sys/fs/cgroup/minit/sshd.service")));
        let status = services.status(Some("sshd.service")).unwrap();
        assert_eq!(status[0].state, UnitState::Inactive);
        assert_eq!(status[0].main_pid, None);
    }

    #[test]
    fn stop_service_fails_if_cgroup_stays_populated() {
        let mut services = manager_with_sshd();
        services.plan_start("sshd.service").unwrap();
        services.mark_active("sshd.service", 321).unwrap();
        let cgroups = CgroupManager::new("/sys/fs/cgroup/minit");
        let mut cgroup_fs = FakeCgroupFs::default();
        cgroup_fs.reads.insert(
            PathBuf::from("/sys/fs/cgroup/minit/sshd.service/cgroup.events"),
            "populated 1\nfrozen 0\n".to_string(),
        );

        let error =
            stop_service(&mut services, &cgroups, &mut cgroup_fs, "sshd.service").unwrap_err();

        assert_eq!(
            error,
            RuntimeError::CgroupStillPopulated("sshd.service".to_string())
        );
        assert!(!cgroup_fs
            .removed
            .contains(Path::new("/sys/fs/cgroup/minit/sshd.service")));
    }
}
