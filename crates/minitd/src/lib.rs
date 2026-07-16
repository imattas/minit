pub mod early_mounts;
pub mod reaper;
pub mod rescue;

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
    0
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
}
