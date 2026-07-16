pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn run() {
    println!("minitd {}", version());
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_package_version() {
        assert_eq!(crate::version(), env!("CARGO_PKG_VERSION"));
    }
}
