use std::path::{Path, PathBuf};
use thiserror::Error;

pub const DEFAULT_CGROUP_ROOT: &str = "/sys/fs/cgroup/minit";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitCgroup {
    pub unit: String,
    pub path: PathBuf,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CgroupError {
    #[error("unit name {0:?} is not safe for a cgroup path")]
    UnsafeUnitName(String),
    #[error("cgroup filesystem operation failed: {0}")]
    Fs(String),
}

pub trait CgroupFs {
    fn create_dir_all(&mut self, path: &Path) -> Result<(), CgroupError>;
    fn read_to_string(&mut self, path: &Path) -> Result<String, CgroupError>;
    fn remove_dir(&mut self, path: &Path) -> Result<(), CgroupError>;
    fn write(&mut self, path: &Path, value: &str) -> Result<(), CgroupError>;
}

#[derive(Debug, Clone)]
pub struct CgroupManager {
    root: PathBuf,
}

impl CgroupManager {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn ensure_root<F: CgroupFs>(&self, fs: &mut F) -> Result<(), CgroupError> {
        fs.create_dir_all(&self.root)
    }

    pub fn create_unit<F: CgroupFs>(
        &self,
        fs: &mut F,
        unit: &str,
    ) -> Result<UnitCgroup, CgroupError> {
        let path = self.unit_path(unit)?;
        fs.create_dir_all(&path)?;
        Ok(UnitCgroup {
            unit: unit.to_string(),
            path,
        })
    }

    pub fn attach_pid<F: CgroupFs>(
        &self,
        fs: &mut F,
        unit: &str,
        pid: u32,
    ) -> Result<(), CgroupError> {
        let path = self.unit_path(unit)?.join("cgroup.procs");
        fs.write(&path, &format!("{pid}\n"))
    }

    pub fn kill_unit<F: CgroupFs>(&self, fs: &mut F, unit: &str) -> Result<(), CgroupError> {
        let path = self.unit_path(unit)?.join("cgroup.kill");
        fs.write(&path, "1\n")
    }

    pub fn unit_pids<F: CgroupFs>(&self, fs: &mut F, unit: &str) -> Result<Vec<u32>, CgroupError> {
        let path = self.unit_path(unit)?.join("cgroup.procs");
        let procs = fs.read_to_string(&path)?;
        procs
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                line.trim().parse::<u32>().map_err(|_| {
                    CgroupError::Fs(format!("invalid pid {:?} in {}", line, path.display()))
                })
            })
            .collect()
    }

    pub fn unit_is_empty<F: CgroupFs>(&self, fs: &mut F, unit: &str) -> Result<bool, CgroupError> {
        let path = self.unit_path(unit)?.join("cgroup.events");
        let events = fs.read_to_string(&path)?;
        Ok(events
            .lines()
            .any(|line| line.split_whitespace().eq(["populated", "0"])))
    }

    pub fn remove_unit<F: CgroupFs>(&self, fs: &mut F, unit: &str) -> Result<(), CgroupError> {
        let path = self.unit_path(unit)?;
        fs.remove_dir(&path)
    }

    fn unit_path(&self, unit: &str) -> Result<PathBuf, CgroupError> {
        if !minit_core::unit::is_safe_unit_name(unit) {
            return Err(CgroupError::UnsafeUnitName(unit.to_string()));
        }
        Ok(self.root.join(unit))
    }
}

pub struct LinuxCgroupFs;

impl CgroupFs for LinuxCgroupFs {
    fn create_dir_all(&mut self, path: &Path) -> Result<(), CgroupError> {
        std::fs::create_dir_all(path).map_err(|err| CgroupError::Fs(err.to_string()))
    }

    fn read_to_string(&mut self, path: &Path) -> Result<String, CgroupError> {
        std::fs::read_to_string(path).map_err(|err| CgroupError::Fs(err.to_string()))
    }

    fn remove_dir(&mut self, path: &Path) -> Result<(), CgroupError> {
        std::fs::remove_dir(path).map_err(|err| CgroupError::Fs(err.to_string()))
    }

    fn write(&mut self, path: &Path, value: &str) -> Result<(), CgroupError> {
        std::fs::write(path, value).map_err(|err| CgroupError::Fs(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

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

    #[test]
    fn ensure_root_creates_configured_root() {
        let manager = CgroupManager::new("/sys/fs/cgroup/minit");
        let mut fs = FakeCgroupFs::default();

        manager.ensure_root(&mut fs).unwrap();

        assert!(fs.dirs.contains(Path::new("/sys/fs/cgroup/minit")));
    }

    #[test]
    fn create_unit_creates_per_unit_cgroup() {
        let manager = CgroupManager::new("/sys/fs/cgroup/minit");
        let mut fs = FakeCgroupFs::default();

        let cgroup = manager.create_unit(&mut fs, "sshd.service").unwrap();

        assert_eq!(cgroup.unit, "sshd.service");
        assert_eq!(
            cgroup.path,
            PathBuf::from("/sys/fs/cgroup/minit/sshd.service")
        );
        assert!(fs
            .dirs
            .contains(Path::new("/sys/fs/cgroup/minit/sshd.service")));
    }

    #[test]
    fn attach_pid_writes_to_cgroup_procs() {
        let manager = CgroupManager::new("/sys/fs/cgroup/minit");
        let mut fs = FakeCgroupFs::default();

        manager.attach_pid(&mut fs, "sshd.service", 123).unwrap();

        assert_eq!(
            fs.writes
                .get(Path::new("/sys/fs/cgroup/minit/sshd.service/cgroup.procs")),
            Some(&"123\n".to_string())
        );
    }

    #[test]
    fn kill_unit_writes_to_cgroup_kill() {
        let manager = CgroupManager::new("/sys/fs/cgroup/minit");
        let mut fs = FakeCgroupFs::default();

        manager.kill_unit(&mut fs, "sshd.service").unwrap();

        assert_eq!(
            fs.writes
                .get(Path::new("/sys/fs/cgroup/minit/sshd.service/cgroup.kill")),
            Some(&"1\n".to_string())
        );
    }

    #[test]
    fn unit_pids_reads_cgroup_procs() {
        let manager = CgroupManager::new("/sys/fs/cgroup/minit");
        let mut fs = FakeCgroupFs::default();
        fs.reads.insert(
            PathBuf::from("/sys/fs/cgroup/minit/sshd.service/cgroup.procs"),
            "123\n456\n\n".to_string(),
        );

        let pids = manager.unit_pids(&mut fs, "sshd.service").unwrap();

        assert_eq!(pids, vec![123, 456]);
    }

    #[test]
    fn unit_is_empty_reads_cgroup_events() {
        let manager = CgroupManager::new("/sys/fs/cgroup/minit");
        let mut fs = FakeCgroupFs::default();
        fs.reads.insert(
            PathBuf::from("/sys/fs/cgroup/minit/sshd.service/cgroup.events"),
            "populated 0\nfrozen 0\n".to_string(),
        );

        assert!(manager.unit_is_empty(&mut fs, "sshd.service").unwrap());
    }

    #[test]
    fn remove_unit_removes_per_unit_cgroup() {
        let manager = CgroupManager::new("/sys/fs/cgroup/minit");
        let mut fs = FakeCgroupFs::default();

        manager.remove_unit(&mut fs, "sshd.service").unwrap();

        assert!(fs
            .removed
            .contains(Path::new("/sys/fs/cgroup/minit/sshd.service")));
    }

    #[test]
    fn unsafe_unit_names_are_rejected() {
        let manager = CgroupManager::new("/sys/fs/cgroup/minit");

        let error = manager.unit_path("../escape").unwrap_err();

        assert_eq!(error, CgroupError::UnsafeUnitName("../escape".to_string()));
    }
}
