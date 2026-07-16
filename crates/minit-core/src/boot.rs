#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootMode {
    Normal,
    Rescue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RescueConfig {
    pub mode: BootMode,
    pub command: Vec<String>,
}

impl Default for RescueConfig {
    fn default() -> Self {
        Self {
            mode: BootMode::Rescue,
            command: vec!["/bin/sh".to_string()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EarlyMount {
    pub source: &'static str,
    pub target: &'static str,
    pub fstype: &'static str,
    pub flags: u64,
}

pub fn default_early_mounts() -> Vec<EarlyMount> {
    vec![
        EarlyMount {
            source: "proc",
            target: "/proc",
            fstype: "proc",
            flags: 0,
        },
        EarlyMount {
            source: "sysfs",
            target: "/sys",
            fstype: "sysfs",
            flags: 0,
        },
        EarlyMount {
            source: "devtmpfs",
            target: "/dev",
            fstype: "devtmpfs",
            flags: 0,
        },
        EarlyMount {
            source: "tmpfs",
            target: "/run",
            fstype: "tmpfs",
            flags: 0,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_early_mounts_cover_required_rescue_filesystems() {
        let mounts = default_early_mounts();
        let targets: Vec<&str> = mounts.iter().map(|mount| mount.target).collect();

        assert_eq!(targets, vec!["/proc", "/sys", "/dev", "/run"]);
    }

    #[test]
    fn rescue_config_defaults_to_shell() {
        let config = RescueConfig::default();

        assert_eq!(config.command, vec!["/bin/sh"]);
        assert_eq!(config.mode, BootMode::Rescue);
    }
}
