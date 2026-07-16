use crate::cgroups::{CgroupError, CgroupFs, CgroupManager};
use crate::control::ControlRuntime;
use crate::reaper::{drain_reap_events, ReapStatus, Reaper};
use crate::storage::{StorageError, StorageExecutor, SystemStorageExecutor};
use minit_core::manager::{ServiceManager, ServiceManagerError, StartPlan};
use std::time::Duration;
use thiserror::Error;

const STOP_POLL_INTERVAL: Duration = Duration::from_millis(25);

pub trait ProcessSpawner {
    fn spawn(&mut self, plan: &StartPlan) -> Result<u32, SpawnError>;
}

pub trait ProcessSignaler {
    fn terminate(&mut self, pid: u32) -> Result<(), SignalError>;
}

pub trait RestartSleeper {
    fn sleep(&mut self, delay: Duration);
}

#[derive(Default)]
pub struct ThreadRestartSleeper;

impl RestartSleeper for ThreadRestartSleeper {
    fn sleep(&mut self, delay: Duration) {
        std::thread::sleep(delay);
    }
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
#[error("failed to signal process {pid}: {message}")]
pub struct SignalError {
    pub pid: u32,
    pub message: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("service manager error: {0}")]
    Manager(#[from] ServiceManagerError),
    #[error("cgroup error: {0}")]
    Cgroup(#[from] CgroupError),
    #[error("spawn error: {0}")]
    Spawn(#[from] SpawnError),
    #[error("signal error: {0}")]
    Signal(#[from] SignalError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
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
    if let Err(error) = cgroups.apply_resources(cgroup_fs, unit, &plan.resources) {
        let _ = services.mark_failed(unit);
        return Err(error.into());
    }

    let pid = match spawner.spawn(&plan) {
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

    services.set_cgroup_path(unit, cgroups.cgroup_path(unit)?.display().to_string())?;
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
    let mut signaler = SystemProcessSignaler;
    stop_service_with_signaler(services, cgroups, cgroup_fs, &mut signaler, unit)
}

pub fn stop_service_with_signaler<F, S>(
    services: &mut ServiceManager,
    cgroups: &CgroupManager,
    cgroup_fs: &mut F,
    signaler: &mut S,
    unit: &str,
) -> Result<(), RuntimeError>
where
    F: CgroupFs,
    S: ProcessSignaler,
{
    let stop_timeout = services.stop_timeout(unit)?;
    for pid in cgroups.unit_pids(cgroup_fs, unit)? {
        signaler.terminate(pid)?;
    }

    if !wait_until_cgroup_empty_for_duration(cgroups, cgroup_fs, unit, stop_timeout)? {
        cgroups.kill_unit(cgroup_fs, unit)?;
        eprintln!("minitd: escalated {unit} to cgroup.kill");
        wait_until_cgroup_empty_for_duration(cgroups, cgroup_fs, unit, stop_timeout)?
            .then_some(())
            .ok_or_else(|| RuntimeError::CgroupStillPopulated(unit.to_string()))?;
    }

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
    if wait_until_cgroup_empty_for_duration(cgroups, cgroup_fs, unit, Duration::from_millis(500))? {
        return Ok(());
    }
    Err(RuntimeError::CgroupStillPopulated(unit.to_string()))
}

fn wait_until_cgroup_empty_for_duration<F>(
    cgroups: &CgroupManager,
    cgroup_fs: &mut F,
    unit: &str,
    timeout: Duration,
) -> Result<bool, RuntimeError>
where
    F: CgroupFs,
{
    let attempts = stop_poll_attempts(timeout);
    for attempt in 0..attempts {
        if cgroups.unit_is_empty(cgroup_fs, unit)? {
            return Ok(true);
        }
        if attempt + 1 < attempts {
            std::thread::sleep(STOP_POLL_INTERVAL);
        }
    }
    Ok(false)
}

fn stop_poll_attempts(timeout: Duration) -> usize {
    let interval_ms = STOP_POLL_INTERVAL.as_millis();
    let timeout_ms = timeout.as_millis();
    timeout_ms.div_ceil(interval_ms).max(1) as usize
}

pub struct ServiceRuntime<F, P, R = NoopReaper, S = ThreadRestartSleeper, T = SystemStorageExecutor>
{
    cgroups: CgroupManager,
    cgroup_fs: F,
    spawner: P,
    reaper: R,
    sleeper: S,
    storage: T,
}

impl<F, P> ServiceRuntime<F, P, NoopReaper, ThreadRestartSleeper, SystemStorageExecutor> {
    pub fn new(cgroups: CgroupManager, cgroup_fs: F, spawner: P) -> Self {
        Self {
            cgroups,
            cgroup_fs,
            spawner,
            reaper: NoopReaper,
            sleeper: ThreadRestartSleeper,
            storage: SystemStorageExecutor,
        }
    }
}

impl<F, P, T> ServiceRuntime<F, P, NoopReaper, ThreadRestartSleeper, T> {
    pub fn with_storage(cgroups: CgroupManager, cgroup_fs: F, spawner: P, storage: T) -> Self {
        Self {
            cgroups,
            cgroup_fs,
            spawner,
            reaper: NoopReaper,
            sleeper: ThreadRestartSleeper,
            storage,
        }
    }
}

impl<F, P, R> ServiceRuntime<F, P, R, ThreadRestartSleeper, SystemStorageExecutor> {
    pub fn with_reaper(cgroups: CgroupManager, cgroup_fs: F, spawner: P, reaper: R) -> Self {
        Self {
            cgroups,
            cgroup_fs,
            spawner,
            reaper,
            sleeper: ThreadRestartSleeper,
            storage: SystemStorageExecutor,
        }
    }
}

impl<F, P, R, S> ServiceRuntime<F, P, R, S, SystemStorageExecutor> {
    pub fn with_reaper_and_sleeper(
        cgroups: CgroupManager,
        cgroup_fs: F,
        spawner: P,
        reaper: R,
        sleeper: S,
    ) -> Self {
        Self {
            cgroups,
            cgroup_fs,
            spawner,
            reaper,
            sleeper,
            storage: SystemStorageExecutor,
        }
    }
}

impl<F, P, R, S, T> ControlRuntime for ServiceRuntime<F, P, R, S, T>
where
    F: CgroupFs,
    P: ProcessSpawner,
    R: Reaper,
    S: RestartSleeper,
    T: StorageExecutor,
{
    fn start(&mut self, services: &mut ServiceManager, unit: &str) -> Result<String, String> {
        if services.is_mount(unit).map_err(|err| err.to_string())? {
            let plan = services.plan_mount(unit).map_err(|err| err.to_string())?;
            if let Err(error) = self.storage.mount(&plan) {
                if plan.required {
                    let _ = services.mark_failed(unit);
                    return Err(error.to_string());
                }
                services
                    .mark_inactive(unit)
                    .map_err(|err| err.to_string())?;
                return Ok(format!("skipped optional mount {unit}"));
            }
            services
                .mark_active_without_pid(unit)
                .map_err(|err| err.to_string())?;
            return Ok(format!("mounted {unit}"));
        }
        if services.is_swap(unit).map_err(|err| err.to_string())? {
            let plan = services.plan_swap(unit).map_err(|err| err.to_string())?;
            if let Err(error) = self.storage.swap_on(&plan) {
                if plan.required {
                    let _ = services.mark_failed(unit);
                    return Err(error.to_string());
                }
                services
                    .mark_inactive(unit)
                    .map_err(|err| err.to_string())?;
                return Ok(format!("skipped optional swap {unit}"));
            }
            services
                .mark_active_without_pid(unit)
                .map_err(|err| err.to_string())?;
            return Ok(format!("activated swap {unit}"));
        }
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
        if services.is_mount(unit).map_err(|err| err.to_string())? {
            let plan = services.plan_mount(unit).map_err(|err| err.to_string())?;
            self.storage.unmount(&plan).map_err(|err| err.to_string())?;
            services
                .mark_inactive(unit)
                .map_err(|err| err.to_string())?;
            return Ok(format!("unmounted {unit}"));
        }
        if services.is_swap(unit).map_err(|err| err.to_string())? {
            let plan = services.plan_swap(unit).map_err(|err| err.to_string())?;
            self.storage
                .swap_off(&plan)
                .map_err(|err| err.to_string())?;
            services
                .mark_inactive(unit)
                .map_err(|err| err.to_string())?;
            return Ok(format!("deactivated swap {unit}"));
        }
        stop_service(services, &self.cgroups, &mut self.cgroup_fs, unit)
            .map_err(|err| err.to_string())?;
        Ok(format!("stopped {unit}"))
    }

    fn reap(&mut self, services: &mut ServiceManager) -> Result<(), String> {
        let events = drain_reap_events(&mut self.reaper).map_err(|err| err.to_string())?;
        for event in events {
            let successful = matches!(event.status, ReapStatus::Exited(0));
            let Some(decision) = services
                .record_exit_with_status(
                    event.pid as u32,
                    successful,
                    render_reap_status(&event.status),
                )
                .map_err(|err| err.to_string())?
            else {
                continue;
            };

            let _ = wait_until_cgroup_empty(&self.cgroups, &mut self.cgroup_fs, &decision.unit);
            let _ = self
                .cgroups
                .remove_unit(&mut self.cgroup_fs, &decision.unit);

            if decision.restart {
                if !decision.delay.is_zero() {
                    self.sleeper.sleep(decision.delay);
                }
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
        let storage_units = services.active_storage_unit_names();
        for unit in storage_units {
            self.stop(services, &unit)?;
            eprintln!("minitd: deactivated {unit} for shutdown");
        }
        Ok(())
    }
}

fn render_reap_status(status: &ReapStatus) -> String {
    match status {
        ReapStatus::Exited(code) => format!("exit {code}"),
        ReapStatus::Signaled(signal) => format!("signal {signal}"),
        ReapStatus::StillAlive => "still alive".to_string(),
    }
}

pub struct CommandSpawner;

impl ProcessSpawner for CommandSpawner {
    fn spawn(&mut self, plan: &StartPlan) -> Result<u32, SpawnError> {
        let Some(program) = plan.argv.first() else {
            return Err(SpawnError("empty argv".to_string()));
        };
        let mut command = std::process::Command::new(program);
        command.args(&plan.argv[1..]);
        if let Some(working_directory) = &plan.working_directory {
            command.current_dir(working_directory);
        }
        command.env_clear();
        for entry in &plan.environment {
            if let Some((key, value)) = entry.split_once('=') {
                command.env(key, value);
            }
        }
        configure_child_security(
            &mut command,
            plan.no_new_privileges,
            plan.user.as_deref(),
            plan.group.as_deref(),
        );
        let child = command.spawn().map_err(|err| SpawnError(err.to_string()))?;
        Ok(child.id())
    }
}

pub struct SystemProcessSignaler;

impl ProcessSignaler for SystemProcessSignaler {
    fn terminate(&mut self, pid: u32) -> Result<(), SignalError> {
        terminate_process(pid)
    }
}

#[cfg(target_os = "linux")]
fn terminate_process(pid: u32) -> Result<(), SignalError> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    kill(Pid::from_raw(pid as i32), Signal::SIGTERM).map_err(|err| SignalError {
        pid,
        message: err.to_string(),
    })
}

#[cfg(not(target_os = "linux"))]
fn terminate_process(_pid: u32) -> Result<(), SignalError> {
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn parse_security_id(value: &str) -> Option<u32> {
    if value == "root" {
        Some(0)
    } else {
        value.parse::<u32>().ok()
    }
}

#[cfg(target_os = "linux")]
fn configure_child_security(
    command: &mut std::process::Command,
    no_new_privileges: bool,
    user: Option<&str>,
    group: Option<&str>,
) {
    use std::os::unix::process::CommandExt;

    if no_new_privileges || user.is_some() || group.is_some() {
        let uid = user.and_then(parse_security_id);
        let gid = group.and_then(parse_security_id);
        unsafe {
            command.pre_exec(move || {
                if let Some(gid) = gid {
                    if libc::setgid(gid) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                if let Some(uid) = uid {
                    if libc::setuid(uid) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                if no_new_privileges {
                    let result = libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0);
                    if result != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn configure_child_security(
    _command: &mut std::process::Command,
    _no_new_privileges: bool,
    _user: Option<&str>,
    _group: Option<&str>,
) {
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cgroups::CgroupFs;
    use crate::reaper::{ReapError, ReapEvent};
    use crate::storage::{StorageError, StorageExecutor};
    use minit_core::ipc::UnitState;
    use minit_core::manager::{MountPlan, SwapPlan};
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
        fn spawn(&mut self, plan: &StartPlan) -> Result<u32, SpawnError> {
            self.argv.push(plan.argv.clone());
            Ok(self.next_pid)
        }
    }

    struct FakeReaper {
        events: Vec<ReapEvent>,
    }

    impl Reaper for FakeReaper {
        fn reap_once(&mut self) -> Result<Option<ReapEvent>, ReapError> {
            Ok(self.events.pop())
        }
    }

    #[derive(Default)]
    struct FakeSignaler {
        terminated: Vec<u32>,
    }

    impl ProcessSignaler for FakeSignaler {
        fn terminate(&mut self, pid: u32) -> Result<(), SignalError> {
            self.terminated.push(pid);
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeSleeper {
        sleeps: Vec<Duration>,
    }

    impl RestartSleeper for FakeSleeper {
        fn sleep(&mut self, delay: Duration) {
            self.sleeps.push(delay);
        }
    }

    #[derive(Default)]
    struct FakeStorage {
        calls: Vec<String>,
        fail_mount: bool,
    }

    impl StorageExecutor for FakeStorage {
        fn mount(&mut self, plan: &MountPlan) -> Result<(), StorageError> {
            if self.fail_mount {
                return Err(StorageError::Mount {
                    target: plan.where_path.clone(),
                    message: "fake mount failure".to_string(),
                });
            }
            self.calls.push(format!(
                "mount:{}:{}:{}:{}",
                plan.what,
                plan.where_path,
                plan.fstype,
                plan.options.join(",")
            ));
            Ok(())
        }

        fn unmount(&mut self, plan: &MountPlan) -> Result<(), StorageError> {
            self.calls.push(format!("unmount:{}", plan.where_path));
            Ok(())
        }

        fn swap_on(&mut self, plan: &SwapPlan) -> Result<(), StorageError> {
            self.calls
                .push(format!("swapon:{}:{:?}", plan.path, plan.priority));
            Ok(())
        }

        fn swap_off(&mut self, plan: &SwapPlan) -> Result<(), StorageError> {
            self.calls.push(format!("swapoff:{}", plan.path));
            Ok(())
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
    fn stop_service_terminates_cgroup_and_marks_inactive() {
        let mut services = manager_with_sshd();
        services.plan_start("sshd.service").unwrap();
        services.mark_active("sshd.service", 321).unwrap();
        let cgroups = CgroupManager::new("/sys/fs/cgroup/minit");
        let mut cgroup_fs = FakeCgroupFs::default();
        cgroup_fs.reads.insert(
            PathBuf::from("/sys/fs/cgroup/minit/sshd.service/cgroup.procs"),
            "321\n".to_string(),
        );
        cgroup_fs.reads.insert(
            PathBuf::from("/sys/fs/cgroup/minit/sshd.service/cgroup.events"),
            "populated 0\nfrozen 0\n".to_string(),
        );
        let mut signaler = FakeSignaler::default();

        stop_service_with_signaler(
            &mut services,
            &cgroups,
            &mut cgroup_fs,
            &mut signaler,
            "sshd.service",
        )
        .unwrap();

        assert!(!cgroup_fs
            .writes
            .contains_key(Path::new("/sys/fs/cgroup/minit/sshd.service/cgroup.kill")));
        assert_eq!(signaler.terminated, vec![321]);
        assert!(cgroup_fs
            .removed
            .contains(Path::new("/sys/fs/cgroup/minit/sshd.service")));
        let status = services.status(Some("sshd.service")).unwrap();
        assert_eq!(status[0].state, UnitState::Inactive);
        assert_eq!(status[0].main_pid, None);
    }

    #[test]
    fn stop_service_sends_sigterm_to_cgroup_members_before_kill() {
        let mut services = manager_with_sshd();
        services.plan_start("sshd.service").unwrap();
        services.mark_active("sshd.service", 321).unwrap();
        let cgroups = CgroupManager::new("/sys/fs/cgroup/minit");
        let mut cgroup_fs = FakeCgroupFs::default();
        cgroup_fs.reads.insert(
            PathBuf::from("/sys/fs/cgroup/minit/sshd.service/cgroup.procs"),
            "321\n654\n".to_string(),
        );
        cgroup_fs.reads.insert(
            PathBuf::from("/sys/fs/cgroup/minit/sshd.service/cgroup.events"),
            "populated 1\nfrozen 0\n".to_string(),
        );
        let mut signaler = FakeSignaler::default();

        let _ = stop_service_with_signaler(
            &mut services,
            &cgroups,
            &mut cgroup_fs,
            &mut signaler,
            "sshd.service",
        );

        assert_eq!(signaler.terminated, vec![321, 654]);
        assert_eq!(
            cgroup_fs
                .writes
                .get(Path::new("/sys/fs/cgroup/minit/sshd.service/cgroup.kill")),
            Some(&"1\n".to_string())
        );
    }

    #[test]
    fn stop_service_fails_if_cgroup_stays_populated() {
        let mut services = manager_with_sshd();
        services.plan_start("sshd.service").unwrap();
        services.mark_active("sshd.service", 321).unwrap();
        let cgroups = CgroupManager::new("/sys/fs/cgroup/minit");
        let mut cgroup_fs = FakeCgroupFs::default();
        cgroup_fs.reads.insert(
            PathBuf::from("/sys/fs/cgroup/minit/sshd.service/cgroup.procs"),
            "321\n".to_string(),
        );
        cgroup_fs.reads.insert(
            PathBuf::from("/sys/fs/cgroup/minit/sshd.service/cgroup.events"),
            "populated 1\nfrozen 0\n".to_string(),
        );
        let mut signaler = FakeSignaler::default();

        let error = stop_service_with_signaler(
            &mut services,
            &cgroups,
            &mut cgroup_fs,
            &mut signaler,
            "sshd.service",
        )
        .unwrap_err();

        assert_eq!(
            error,
            RuntimeError::CgroupStillPopulated("sshd.service".to_string())
        );
        assert_eq!(signaler.terminated, vec![321]);
        assert!(!cgroup_fs
            .removed
            .contains(Path::new("/sys/fs/cgroup/minit/sshd.service")));
    }

    #[test]
    fn reap_waits_restart_backoff_before_restart() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "crashy.service"

[exec]
start = ["/bin/false"]

[restart]
policy = "on-failure"
limit = "3/min"
backoff = "fixed"
"#,
        )
        .unwrap();
        let mut services = ServiceManager::new();
        services.add_unit(unit).unwrap();
        services.plan_start("crashy.service").unwrap();
        services.mark_active("crashy.service", 321).unwrap();
        let mut cgroup_fs = FakeCgroupFs::default();
        cgroup_fs.reads.insert(
            PathBuf::from("/sys/fs/cgroup/minit/crashy.service/cgroup.events"),
            "populated 0\nfrozen 0\n".to_string(),
        );
        let cgroups = CgroupManager::new("/sys/fs/cgroup/minit");
        let spawner = FakeSpawner {
            next_pid: 654,
            ..FakeSpawner::default()
        };
        let reaper = FakeReaper {
            events: vec![ReapEvent {
                pid: 321,
                status: ReapStatus::Exited(1),
            }],
        };
        let sleeper = FakeSleeper::default();
        let mut runtime =
            ServiceRuntime::with_reaper_and_sleeper(cgroups, cgroup_fs, spawner, reaper, sleeper);

        runtime.reap(&mut services).unwrap();

        assert_eq!(runtime.sleeper.sleeps, vec![Duration::from_secs(1)]);
        let status = services.status(Some("crashy.service")).unwrap();
        assert_eq!(status[0].state, UnitState::Active);
        assert_eq!(status[0].main_pid, Some(654));
    }

    #[test]
    fn stop_poll_attempts_are_derived_from_timeout() {
        assert_eq!(stop_poll_attempts(Duration::from_millis(1)), 1);
        assert_eq!(stop_poll_attempts(Duration::from_millis(25)), 1);
        assert_eq!(stop_poll_attempts(Duration::from_millis(26)), 2);
        assert_eq!(stop_poll_attempts(Duration::from_millis(500)), 20);
    }

    #[test]
    fn start_mount_unit_uses_storage_executor_and_marks_active() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "var-log.mount"
kind = "mount"

[mount]
what = "tmpfs"
where = "/var/log"
fstype = "tmpfs"
options = ["nosuid", "nodev"]
"#,
        )
        .unwrap();
        let mut services = ServiceManager::new();
        services.add_unit(unit).unwrap();
        let mut runtime = ServiceRuntime::with_storage(
            CgroupManager::new("/sys/fs/cgroup/minit"),
            FakeCgroupFs::default(),
            FakeSpawner::default(),
            FakeStorage::default(),
        );

        let message = runtime.start(&mut services, "var-log.mount").unwrap();

        assert_eq!(message, "mounted var-log.mount");
        assert_eq!(
            runtime.storage.calls,
            vec!["mount:tmpfs:/var/log:tmpfs:nosuid,nodev"]
        );
        assert_eq!(
            services.status(Some("var-log.mount")).unwrap()[0].state,
            UnitState::Active
        );
    }

    #[test]
    fn start_swap_unit_uses_storage_executor_and_marks_active() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "scratch.swap"
kind = "swap"

[swap]
path = "/swapfile"
priority = 5
"#,
        )
        .unwrap();
        let mut services = ServiceManager::new();
        services.add_unit(unit).unwrap();
        let mut runtime = ServiceRuntime::with_storage(
            CgroupManager::new("/sys/fs/cgroup/minit"),
            FakeCgroupFs::default(),
            FakeSpawner::default(),
            FakeStorage::default(),
        );

        let message = runtime.start(&mut services, "scratch.swap").unwrap();

        assert_eq!(message, "activated swap scratch.swap");
        assert_eq!(runtime.storage.calls, vec!["swapon:/swapfile:Some(5)"]);
        assert_eq!(
            services.status(Some("scratch.swap")).unwrap()[0].state,
            UnitState::Active
        );
    }

    #[test]
    fn shutdown_deactivates_active_storage_units() {
        let mount = parse_unit_toml(
            r#"
[unit]
name = "var-log.mount"
kind = "mount"

[mount]
what = "tmpfs"
where = "/var/log"
fstype = "tmpfs"
"#,
        )
        .unwrap();
        let swap = parse_unit_toml(
            r#"
[unit]
name = "scratch.swap"
kind = "swap"

[swap]
path = "/swapfile"
"#,
        )
        .unwrap();
        let mut services = ServiceManager::new();
        services.add_unit(mount).unwrap();
        services.add_unit(swap).unwrap();
        services.mark_active_without_pid("var-log.mount").unwrap();
        services.mark_active_without_pid("scratch.swap").unwrap();
        let mut runtime = ServiceRuntime::with_storage(
            CgroupManager::new("/sys/fs/cgroup/minit"),
            FakeCgroupFs::default(),
            FakeSpawner::default(),
            FakeStorage::default(),
        );

        runtime.shutdown(&mut services).unwrap();

        assert_eq!(
            runtime.storage.calls,
            vec!["unmount:/var/log", "swapoff:/swapfile"]
        );
        assert_eq!(
            services.status(Some("var-log.mount")).unwrap()[0].state,
            UnitState::Inactive
        );
        assert_eq!(
            services.status(Some("scratch.swap")).unwrap()[0].state,
            UnitState::Inactive
        );
    }

    #[test]
    fn required_mount_failure_marks_unit_failed() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "broken.mount"
kind = "mount"

[mount]
what = "missing"
where = "/mnt/missing"
fstype = "missingfs"
"#,
        )
        .unwrap();
        let mut services = ServiceManager::new();
        services.add_unit(unit).unwrap();
        let mut runtime = ServiceRuntime::with_storage(
            CgroupManager::new("/sys/fs/cgroup/minit"),
            FakeCgroupFs::default(),
            FakeSpawner::default(),
            FakeStorage {
                fail_mount: true,
                ..FakeStorage::default()
            },
        );

        let error = runtime.start(&mut services, "broken.mount").unwrap_err();

        assert!(error.contains("fake mount failure"));
        assert_eq!(
            services.status(Some("broken.mount")).unwrap()[0].state,
            UnitState::Failed
        );
    }

    #[test]
    fn optional_mount_failure_marks_unit_inactive_and_does_not_error() {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "optional.mount"
kind = "mount"

[mount]
what = "missing"
where = "/mnt/optional"
fstype = "missingfs"
required = false
"#,
        )
        .unwrap();
        let mut services = ServiceManager::new();
        services.add_unit(unit).unwrap();
        let mut runtime = ServiceRuntime::with_storage(
            CgroupManager::new("/sys/fs/cgroup/minit"),
            FakeCgroupFs::default(),
            FakeSpawner::default(),
            FakeStorage {
                fail_mount: true,
                ..FakeStorage::default()
            },
        );

        let message = runtime.start(&mut services, "optional.mount").unwrap();

        assert_eq!(message, "skipped optional mount optional.mount");
        assert_eq!(
            services.status(Some("optional.mount")).unwrap()[0].state,
            UnitState::Inactive
        );
    }

    #[test]
    fn security_principals_parse_root_and_numeric_ids() {
        assert_eq!(parse_security_id("root"), Some(0));
        assert_eq!(parse_security_id("1000"), Some(1000));
        assert_eq!(parse_security_id("daemon"), None);
    }
}
