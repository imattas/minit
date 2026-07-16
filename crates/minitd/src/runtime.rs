use crate::cgroups::{CgroupError, CgroupFs, CgroupManager};
use minit_core::manager::{ServiceManager, ServiceManagerError};
use thiserror::Error;

pub trait ProcessSpawner {
    fn spawn(&mut self, argv: &[String]) -> Result<u32, SpawnError>;
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
    let plan = services.plan_start(unit)?;

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
        writes: BTreeMap<PathBuf, String>,
    }

    impl CgroupFs for FakeCgroupFs {
        fn create_dir_all(&mut self, path: &Path) -> Result<(), CgroupError> {
            self.dirs.insert(path.to_path_buf());
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
}
