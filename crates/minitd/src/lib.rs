pub mod cgroups;
pub mod control;
pub mod early_mounts;
pub mod reaper;
pub mod rescue;
pub mod runtime;
pub mod shutdown;
pub mod units;

use std::path::PathBuf;

pub const DEFAULT_UNIT_DIR: &str = "/etc/minit/services";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalConfig {
    pub unit_dir: PathBuf,
    pub socket: control::ControlSocketConfig,
    pub smoke_status_unit: Option<String>,
    pub smoke_start_unit: Option<String>,
    pub smoke_stop_unit: Option<String>,
    pub smoke_restart_unit: Option<String>,
    pub smoke_cgroup_cleanup_unit: Option<String>,
    pub smoke_restart_policy_unit: Option<String>,
    pub smoke_shutdown_stop_unit: Option<String>,
    pub smoke_stuck_stop_unit: Option<String>,
    pub smoke_shutdown_stuck_unit: Option<String>,
    pub smoke_boot_target: Option<String>,
}

impl Default for NormalConfig {
    fn default() -> Self {
        Self {
            unit_dir: PathBuf::from(DEFAULT_UNIT_DIR),
            socket: control::ControlSocketConfig::default(),
            smoke_status_unit: None,
            smoke_start_unit: None,
            smoke_stop_unit: None,
            smoke_restart_unit: None,
            smoke_cgroup_cleanup_unit: None,
            smoke_restart_policy_unit: None,
            smoke_shutdown_stop_unit: None,
            smoke_stuck_stop_unit: None,
            smoke_shutdown_stuck_unit: None,
            smoke_boot_target: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError(pub String);

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn run() {
    let exit_code = run_with_args(std::env::args());
    std::process::exit(exit_code);
}

pub fn run_with_args<I, S>(_args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args: Vec<String> = _args.into_iter().map(Into::into).collect();
    let is_pid_one = current_process_is_pid_one();
    let kernel_cmdline = read_kernel_cmdline();

    if should_enter_rescue(args.clone(), is_pid_one, &kernel_cmdline) {
        return run_rescue_entrypoint();
    }

    if should_enter_normal(args.clone(), is_pid_one, &kernel_cmdline) {
        let config = match normal_config_from_kernel_cmdline(&kernel_cmdline)
            .and_then(|kernel_config| merge_normal_config(kernel_config, args))
        {
            Ok(config) => config,
            Err(error) => {
                eprintln!("minitd: {error}", error = error.0);
                return 2;
            }
        };
        return run_normal_entrypoint(config);
    }

    0
}

pub fn is_rescue_requested<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    args.into_iter()
        .map(Into::into)
        .any(|arg| arg == "--rescue" || arg == "minit.rescue=1")
}

pub fn should_enter_rescue<I, S>(args: I, is_pid_one: bool, kernel_cmdline: &str) -> bool
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    let explicit_rescue = is_rescue_requested(args.clone())
        || kernel_cmdline
            .split_whitespace()
            .any(|arg| arg == "minit.rescue=1");

    explicit_rescue
        || (is_pid_one && !is_normal_requested(args) && !kernel_requests_normal(kernel_cmdline))
}

pub fn should_enter_normal<I, S>(args: I, is_pid_one: bool, kernel_cmdline: &str) -> bool
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    !is_rescue_requested(args.clone())
        && !kernel_cmdline
            .split_whitespace()
            .any(|arg| arg == "minit.rescue=1")
        && (is_pid_one || is_normal_requested(args) || kernel_requests_normal(kernel_cmdline))
}

fn is_normal_requested<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    args.into_iter()
        .map(Into::into)
        .any(|arg| arg == "--normal")
}

fn kernel_requests_normal(kernel_cmdline: &str) -> bool {
    kernel_cmdline
        .split_whitespace()
        .any(|arg| arg == "minit.normal=1")
}

pub fn normal_config_from_args<I, S>(args: I) -> Result<NormalConfig, ConfigError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut config = NormalConfig::default();
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--unit-dir" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err(ConfigError("--unit-dir requires a path".to_string()));
                };
                config.unit_dir = PathBuf::from(value);
            }
            "--control-socket" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err(ConfigError("--control-socket requires a path".to_string()));
                };
                config.socket.socket_path = PathBuf::from(value);
            }
            "--max-requests" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err(ConfigError("--max-requests requires a number".to_string()));
                };
                config.socket.max_requests = Some(
                    value
                        .parse()
                        .map_err(|_| ConfigError("--max-requests must be a number".to_string()))?,
                );
            }
            _ => {}
        }
        index += 1;
    }
    Ok(config)
}

pub fn normal_config_from_kernel_cmdline(
    kernel_cmdline: &str,
) -> Result<NormalConfig, ConfigError> {
    let mut config = NormalConfig::default();
    for arg in kernel_cmdline.split_whitespace() {
        if let Some(value) = arg.strip_prefix("minit.unit_dir=") {
            config.unit_dir = PathBuf::from(value);
        } else if let Some(value) = arg.strip_prefix("minit.control_socket=") {
            config.socket.socket_path = PathBuf::from(value);
        } else if let Some(value) = arg.strip_prefix("minit.max_requests=") {
            config.socket.max_requests = Some(
                value
                    .parse()
                    .map_err(|_| ConfigError("minit.max_requests must be a number".to_string()))?,
            );
        } else if let Some(value) = arg.strip_prefix("minit.smoke_status=") {
            config.smoke_status_unit = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("minit.smoke_start=") {
            config.smoke_start_unit = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("minit.smoke_stop=") {
            config.smoke_stop_unit = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("minit.smoke_restart=") {
            config.smoke_restart_unit = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("minit.smoke_cgroup_cleanup=") {
            config.smoke_cgroup_cleanup_unit = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("minit.smoke_restart_policy=") {
            config.smoke_restart_policy_unit = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("minit.smoke_shutdown_stop=") {
            config.smoke_shutdown_stop_unit = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("minit.smoke_stuck_stop=") {
            config.smoke_stuck_stop_unit = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("minit.smoke_shutdown_stuck=") {
            config.smoke_shutdown_stuck_unit = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("minit.smoke_boot_target=") {
            config.smoke_boot_target = Some(value.to_string());
        }
    }
    Ok(config)
}

fn merge_normal_config<I, S>(mut config: NormalConfig, args: I) -> Result<NormalConfig, ConfigError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let arg_config = normal_config_from_args(args)?;
    if arg_config.unit_dir != PathBuf::from(DEFAULT_UNIT_DIR) {
        config.unit_dir = arg_config.unit_dir;
    }
    if arg_config.socket.socket_path != PathBuf::from(minit_core::ipc::DEFAULT_CONTROL_SOCKET) {
        config.socket.socket_path = arg_config.socket.socket_path;
    }
    if arg_config.socket.max_requests.is_some() {
        config.socket.max_requests = arg_config.socket.max_requests;
    }
    if arg_config.smoke_status_unit.is_some() {
        config.smoke_status_unit = arg_config.smoke_status_unit;
    }
    if arg_config.smoke_start_unit.is_some() {
        config.smoke_start_unit = arg_config.smoke_start_unit;
    }
    if arg_config.smoke_stop_unit.is_some() {
        config.smoke_stop_unit = arg_config.smoke_stop_unit;
    }
    if arg_config.smoke_restart_unit.is_some() {
        config.smoke_restart_unit = arg_config.smoke_restart_unit;
    }
    if arg_config.smoke_cgroup_cleanup_unit.is_some() {
        config.smoke_cgroup_cleanup_unit = arg_config.smoke_cgroup_cleanup_unit;
    }
    if arg_config.smoke_restart_policy_unit.is_some() {
        config.smoke_restart_policy_unit = arg_config.smoke_restart_policy_unit;
    }
    if arg_config.smoke_shutdown_stop_unit.is_some() {
        config.smoke_shutdown_stop_unit = arg_config.smoke_shutdown_stop_unit;
    }
    if arg_config.smoke_stuck_stop_unit.is_some() {
        config.smoke_stuck_stop_unit = arg_config.smoke_stuck_stop_unit;
    }
    if arg_config.smoke_shutdown_stuck_unit.is_some() {
        config.smoke_shutdown_stuck_unit = arg_config.smoke_shutdown_stuck_unit;
    }
    if arg_config.smoke_boot_target.is_some() {
        config.smoke_boot_target = arg_config.smoke_boot_target;
    }
    Ok(config)
}

fn current_process_is_pid_one() -> bool {
    std::process::id() == 1
}

fn read_kernel_cmdline() -> String {
    #[cfg(target_os = "linux")]
    {
        let first_attempt = std::fs::read_to_string("/proc/cmdline").unwrap_or_default();
        if !first_attempt.trim().is_empty() {
            return first_attempt;
        }

        if std::process::id() == 1 {
            let _ = mount_proc_for_cmdline();
        }
        std::fs::read_to_string("/proc/cmdline").unwrap_or_default()
    }

    #[cfg(not(target_os = "linux"))]
    {
        String::new()
    }
}

#[cfg(target_os = "linux")]
fn mount_proc_for_cmdline() -> Result<(), String> {
    std::fs::create_dir_all("/proc").map_err(|err| err.to_string())?;
    nix::mount::mount(
        Some("proc"),
        "/proc",
        Some("proc"),
        nix::mount::MsFlags::empty(),
        None::<&str>,
    )
    .map_err(|err| err.to_string())
}

fn run_rescue_entrypoint() -> i32 {
    #[cfg(target_os = "linux")]
    {
        rescue::run_linux_rescue()
    }

    #[cfg(not(target_os = "linux"))]
    {
        0
    }
}

pub fn run_normal_entrypoint(config: NormalConfig) -> i32 {
    #[cfg(target_os = "linux")]
    {
        use crate::shutdown::ShutdownExecutor;

        if let Err(error) = prepare_normal_filesystems() {
            eprintln!("minitd: failed to prepare normal-mode filesystems: {error}");
            return 1;
        }

        let services = if config.unit_dir.exists() {
            match units::load_units_from_dir(&config.unit_dir) {
                Ok(services) => services,
                Err(error) => {
                    eprintln!("minitd: failed to load units: {error}");
                    return 1;
                }
            }
        } else {
            minit_core::manager::ServiceManager::new()
        };
        let mut socket = config.socket.clone();
        if let Some(unit) = &config.smoke_start_unit {
            socket.max_requests = Some(2);
            let socket_path = socket.socket_path.display();
            socket.startup_command = Some(vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                format!(
                    "/bin/minitctl --socket {socket_path} start {unit}; /bin/minitctl --socket {socket_path} status {unit}"
                ),
            ]);
        } else if let Some(unit) = &config.smoke_stop_unit {
            socket.max_requests = Some(3);
            let socket_path = socket.socket_path.display();
            socket.startup_command = Some(vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                format!(
                    "/bin/minitctl --socket {socket_path} start {unit}; /bin/minitctl --socket {socket_path} stop {unit}; /bin/minitctl --socket {socket_path} status {unit}"
                ),
            ]);
        } else if let Some(unit) = &config.smoke_restart_unit {
            socket.max_requests = Some(3);
            let socket_path = socket.socket_path.display();
            socket.startup_command = Some(vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                format!(
                    "/bin/minitctl --socket {socket_path} start {unit}; /bin/minitctl --socket {socket_path} restart {unit}; /bin/minitctl --socket {socket_path} status {unit}"
                ),
            ]);
        } else if let Some(unit) = &config.smoke_cgroup_cleanup_unit {
            socket.max_requests = Some(2);
            let socket_path = socket.socket_path.display();
            socket.startup_command = Some(vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                format!(
                    "/bin/minitctl --socket {socket_path} start {unit}; /bin/minitctl --socket {socket_path} stop {unit}; if [ ! -d /sys/fs/cgroup/minit/{unit} ]; then echo cgroup-cleaned:{unit}; else echo cgroup-still-present:{unit}; fi"
                ),
            ]);
        } else if let Some(unit) = &config.smoke_restart_policy_unit {
            socket.max_requests = Some(2);
            let socket_path = socket.socket_path.display();
            socket.startup_command = Some(vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                format!(
                    "/bin/minitctl --socket {socket_path} start {unit}; /bin/sleep 1; /bin/minitctl --socket {socket_path} status {unit}"
                ),
            ]);
        } else if let Some(unit) = &config.smoke_shutdown_stop_unit {
            socket.max_requests = Some(2);
            let socket_path = socket.socket_path.display();
            socket.startup_command = Some(vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                format!(
                    "/bin/minitctl --socket {socket_path} start {unit}; /bin/minitctl --socket {socket_path} status {unit}"
                ),
            ]);
        } else if let Some(unit) = &config.smoke_stuck_stop_unit {
            socket.max_requests = Some(2);
            let socket_path = socket.socket_path.display();
            socket.startup_command = Some(vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                format!(
                    "/bin/minitctl --socket {socket_path} start {unit}; /bin/minitctl --socket {socket_path} stop {unit}"
                ),
            ]);
        } else if let Some(unit) = &config.smoke_shutdown_stuck_unit {
            socket.max_requests = Some(2);
            let socket_path = socket.socket_path.display();
            socket.startup_command = Some(vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                format!(
                    "/bin/minitctl --socket {socket_path} start {unit}; /bin/minitctl --socket {socket_path} status {unit}"
                ),
            ]);
        } else if let Some(target) = &config.smoke_boot_target {
            socket.max_requests = Some(4);
            let socket_path = socket.socket_path.display();
            socket.startup_command = Some(vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                format!(
                    "/bin/minitctl --socket {socket_path} start {target}; /bin/minitctl --socket {socket_path} status {target}; /bin/minitctl --socket {socket_path} status network.service; /bin/minitctl --socket {socket_path} status demo-sleep"
                ),
            ]);
        } else if let Some(unit) = &config.smoke_status_unit {
            socket.max_requests = Some(1);
            socket.startup_command = Some(vec![
                "/bin/minitctl".to_string(),
                "--socket".to_string(),
                socket.socket_path.display().to_string(),
                "status".to_string(),
                unit.clone(),
            ]);
        }
        let runtime = runtime::ServiceRuntime::with_reaper(
            cgroups::CgroupManager::new(cgroups::DEFAULT_CGROUP_ROOT),
            cgroups::LinuxCgroupFs,
            runtime::CommandSpawner,
            reaper::LinuxReaper,
        );
        let mut service = control::ControlService::with_runtime(services, runtime);
        match control::run_control_socket(&socket, &mut service) {
            Ok(()) => {
                if (config.smoke_status_unit.is_some()
                    || config.smoke_start_unit.is_some()
                    || config.smoke_stop_unit.is_some()
                    || config.smoke_restart_unit.is_some()
                    || config.smoke_cgroup_cleanup_unit.is_some()
                    || config.smoke_restart_policy_unit.is_some()
                    || config.smoke_shutdown_stop_unit.is_some()
                    || config.smoke_stuck_stop_unit.is_some()
                    || config.smoke_shutdown_stuck_unit.is_some()
                    || config.smoke_boot_target.is_some())
                    && std::process::id() == 1
                {
                    if let Err(error) = service.shutdown() {
                        eprintln!("minitd: failed to stop services during shutdown: {error}");
                    }
                    let mut shutdown = shutdown::LinuxShutdownExecutor;
                    let _ = shutdown.sync_filesystems();
                    let _ = shutdown.reboot(shutdown::ShutdownAction::Poweroff);
                }
                0
            }
            Err(error) => {
                eprintln!("minitd: control socket failed: {error}");
                1
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = config;
        0
    }
}

#[cfg(target_os = "linux")]
fn prepare_normal_filesystems() -> Result<(), String> {
    mount_fs("proc", "/proc", "proc")?;
    mount_fs("sysfs", "/sys", "sysfs")?;
    mount_fs("devtmpfs", "/dev", "devtmpfs")?;
    mount_fs("tmpfs", "/run", "tmpfs")?;
    mount_fs("cgroup2", "/sys/fs/cgroup", "cgroup2")?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn mount_fs(source: &str, target: &str, fstype: &str) -> Result<(), String> {
    std::fs::create_dir_all(target).map_err(|err| err.to_string())?;
    match nix::mount::mount(
        Some(source),
        target,
        Some(fstype),
        nix::mount::MsFlags::empty(),
        None::<&str>,
    ) {
        Ok(()) => Ok(()),
        Err(nix::errno::Errno::EBUSY) => Ok(()),
        Err(error) => Err(format!("failed to mount {target}: {error}")),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_package_version() {
        assert_eq!(crate::version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn run_with_args_returns_success_for_host_smoke() {
        assert_eq!(crate::run_with_args(["minitd"]), 0);
    }

    #[test]
    fn detects_rescue_flags_from_cli_and_kernel_args() {
        assert!(crate::is_rescue_requested(["minitd", "--rescue"]));
        assert!(crate::is_rescue_requested(["minitd", "minit.rescue=1"]));
        assert!(!crate::is_rescue_requested(["minitd"]));
    }

    #[test]
    fn host_run_accepts_rescue_flag() {
        assert_eq!(crate::run_with_args(["minitd", "--rescue"]), 0);
    }

    #[test]
    fn host_run_accepts_kernel_rescue_arg() {
        assert_eq!(crate::run_with_args(["minitd", "minit.rescue=1"]), 0);
    }

    #[test]
    fn pid_one_enters_rescue_from_kernel_cmdline() {
        assert!(crate::should_enter_rescue(
            ["/init"],
            true,
            "console=ttyS0 minit.rescue=1"
        ));
        assert!(crate::should_enter_rescue(["/init"], true, "console=ttyS0"));
        assert!(!crate::should_enter_rescue(["minitd"], false, ""));
    }

    #[test]
    fn normal_flag_selects_normal_mode_unless_rescue_is_explicit() {
        assert!(crate::should_enter_normal(
            ["/init", "--normal"],
            true,
            "console=ttyS0"
        ));
        assert!(!crate::should_enter_rescue(
            ["/init", "--normal"],
            true,
            "console=ttyS0"
        ));
        assert!(!crate::should_enter_normal(
            ["/init", "--normal", "--rescue"],
            true,
            "console=ttyS0"
        ));
    }

    #[test]
    fn kernel_cmdline_can_select_normal_mode() {
        assert!(crate::should_enter_normal(
            ["/init"],
            true,
            "console=ttyS0 minit.normal=1"
        ));
        assert!(!crate::should_enter_rescue(
            ["/init"],
            true,
            "console=ttyS0 minit.normal=1"
        ));
    }

    #[test]
    fn normal_config_parses_unit_dir_and_socket_path() {
        let config = crate::normal_config_from_args([
            "minitd",
            "--normal",
            "--unit-dir",
            "/tmp/minit-units",
            "--control-socket",
            "/tmp/minit.sock",
            "--max-requests",
            "2",
        ])
        .unwrap();

        assert_eq!(
            config.unit_dir,
            std::path::PathBuf::from("/tmp/minit-units")
        );
        assert_eq!(
            config.socket.socket_path,
            std::path::PathBuf::from("/tmp/minit.sock")
        );
        assert_eq!(config.socket.max_requests, Some(2));
    }

    #[test]
    fn normal_config_parses_kernel_cmdline_options() {
        let config = crate::normal_config_from_kernel_cmdline(
            "console=ttyS0 minit.normal=1 minit.unit_dir=/etc/minit/services minit.control_socket=/run/minit/minitd.sock minit.max_requests=1",
        )
        .unwrap();

        assert_eq!(
            config.unit_dir,
            std::path::PathBuf::from("/etc/minit/services")
        );
        assert_eq!(
            config.socket.socket_path,
            std::path::PathBuf::from("/run/minit/minitd.sock")
        );
        assert_eq!(config.socket.max_requests, Some(1));
        assert_eq!(config.smoke_status_unit.as_deref(), None);
        assert_eq!(config.smoke_start_unit.as_deref(), None);
        assert_eq!(config.smoke_stop_unit.as_deref(), None);
        assert_eq!(config.smoke_restart_unit.as_deref(), None);
        assert_eq!(config.smoke_cgroup_cleanup_unit.as_deref(), None);
        assert_eq!(config.smoke_restart_policy_unit.as_deref(), None);
        assert_eq!(config.smoke_shutdown_stop_unit.as_deref(), None);
        assert_eq!(config.smoke_stuck_stop_unit.as_deref(), None);
        assert_eq!(config.smoke_shutdown_stuck_unit.as_deref(), None);
        assert_eq!(config.smoke_boot_target.as_deref(), None);
    }

    #[test]
    fn normal_config_parses_smoke_status_unit() {
        let config = crate::normal_config_from_kernel_cmdline(
            "console=ttyS0 minit.normal=1 minit.smoke_status=sshd",
        )
        .unwrap();

        assert_eq!(config.smoke_status_unit.as_deref(), Some("sshd"));
    }

    #[test]
    fn normal_config_parses_smoke_start_unit() {
        let config = crate::normal_config_from_kernel_cmdline(
            "console=ttyS0 minit.normal=1 minit.smoke_start=demo-sleep",
        )
        .unwrap();

        assert_eq!(config.smoke_start_unit.as_deref(), Some("demo-sleep"));
    }

    #[test]
    fn normal_config_parses_smoke_stop_and_restart_units() {
        let stop = crate::normal_config_from_kernel_cmdline(
            "console=ttyS0 minit.normal=1 minit.smoke_stop=demo-sleep",
        )
        .unwrap();
        let restart = crate::normal_config_from_kernel_cmdline(
            "console=ttyS0 minit.normal=1 minit.smoke_restart=demo-sleep",
        )
        .unwrap();

        assert_eq!(stop.smoke_stop_unit.as_deref(), Some("demo-sleep"));
        assert_eq!(restart.smoke_restart_unit.as_deref(), Some("demo-sleep"));
    }

    #[test]
    fn normal_config_parses_smoke_cgroup_cleanup_unit() {
        let config = crate::normal_config_from_kernel_cmdline(
            "console=ttyS0 minit.normal=1 minit.smoke_cgroup_cleanup=demo-sleep",
        )
        .unwrap();

        assert_eq!(
            config.smoke_cgroup_cleanup_unit.as_deref(),
            Some("demo-sleep")
        );
    }

    #[test]
    fn normal_config_parses_smoke_restart_policy_unit() {
        let config = crate::normal_config_from_kernel_cmdline(
            "console=ttyS0 minit.normal=1 minit.smoke_restart_policy=crashy",
        )
        .unwrap();

        assert_eq!(config.smoke_restart_policy_unit.as_deref(), Some("crashy"));
    }

    #[test]
    fn normal_config_parses_smoke_shutdown_stop_unit() {
        let config = crate::normal_config_from_kernel_cmdline(
            "console=ttyS0 minit.normal=1 minit.smoke_shutdown_stop=demo-sleep",
        )
        .unwrap();

        assert_eq!(
            config.smoke_shutdown_stop_unit.as_deref(),
            Some("demo-sleep")
        );
    }

    #[test]
    fn normal_config_parses_stuck_service_smokes() {
        let stop = crate::normal_config_from_kernel_cmdline(
            "console=ttyS0 minit.normal=1 minit.smoke_stuck_stop=stubborn",
        )
        .unwrap();
        let shutdown = crate::normal_config_from_kernel_cmdline(
            "console=ttyS0 minit.normal=1 minit.smoke_shutdown_stuck=stubborn",
        )
        .unwrap();

        assert_eq!(stop.smoke_stuck_stop_unit.as_deref(), Some("stubborn"));
        assert_eq!(
            shutdown.smoke_shutdown_stuck_unit.as_deref(),
            Some("stubborn")
        );
    }

    #[test]
    fn normal_config_parses_boot_target_smoke() {
        let config = crate::normal_config_from_kernel_cmdline(
            "console=ttyS0 minit.normal=1 minit.smoke_boot_target=multi-user.target",
        )
        .unwrap();

        assert_eq!(
            config.smoke_boot_target.as_deref(),
            Some("multi-user.target")
        );
    }
}
