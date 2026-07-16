pub mod early_mounts;
pub mod reaper;
pub mod rescue;
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
    if is_rescue_requested(_args) {
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
}
