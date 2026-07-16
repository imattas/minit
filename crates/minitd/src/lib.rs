pub mod cgroups;
pub mod control;
pub mod early_mounts;
pub mod reaper;
pub mod rescue;
pub mod runtime;
pub mod shutdown;

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
    let is_pid_one = current_process_is_pid_one();
    let kernel_cmdline = read_kernel_cmdline();

    if should_enter_rescue(_args, is_pid_one, &kernel_cmdline) {
        return run_rescue_entrypoint();
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
    is_rescue_requested(args)
        || kernel_cmdline
            .split_whitespace()
            .any(|arg| arg == "minit.rescue=1")
        || is_pid_one
}

fn current_process_is_pid_one() -> bool {
    std::process::id() == 1
}

fn read_kernel_cmdline() -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/cmdline").unwrap_or_default()
    }

    #[cfg(not(target_os = "linux"))]
    {
        String::new()
    }
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
}
