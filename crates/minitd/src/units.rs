use minit_core::manager::{ServiceManager, ServiceManagerError};
use minit_core::unit::{parse_unit_toml, UnitParseError};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UnitLoadError {
    #[error("failed to read unit directory {path}: {message}")]
    ReadDir { path: PathBuf, message: String },
    #[error("failed to read unit file {path}: {message}")]
    ReadFile { path: PathBuf, message: String },
    #[error("failed to parse unit file {path}: {source}")]
    Parse {
        path: PathBuf,
        source: UnitParseError,
    },
    #[error("failed to register unit file {path}: {source}")]
    Register {
        path: PathBuf,
        source: ServiceManagerError,
    },
}

pub fn load_units_from_dir(path: impl AsRef<Path>) -> Result<ServiceManager, UnitLoadError> {
    let path = path.as_ref();
    let mut manager = ServiceManager::new();
    let entries = std::fs::read_dir(path).map_err(|err| UnitLoadError::ReadDir {
        path: path.to_path_buf(),
        message: err.to_string(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|err| UnitLoadError::ReadDir {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
        let file_path = entry.path();
        if file_path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }

        let input = std::fs::read_to_string(&file_path).map_err(|err| UnitLoadError::ReadFile {
            path: file_path.clone(),
            message: err.to_string(),
        })?;
        let unit = parse_unit_toml(&input).map_err(|source| UnitLoadError::Parse {
            path: file_path.clone(),
            source,
        })?;
        manager
            .add_unit(unit)
            .map_err(|source| UnitLoadError::Register {
                path: file_path,
                source,
            })?;
    }

    Ok(manager)
}

#[cfg(test)]
mod tests {
    use super::*;
    use minit_core::ipc::UnitState;
    use std::fs;

    #[test]
    fn load_units_from_dir_registers_service_toml_files() {
        let dir = std::env::temp_dir().join(format!("minit-units-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("demo.service.toml"),
            r#"
[unit]
name = "demo.service"
description = "Demo service"

[exec]
start = ["/bin/sh", "-c", "sleep 60"]
"#,
        )
        .unwrap();
        fs::write(dir.join("ignored.txt"), "not a unit").unwrap();

        let manager = load_units_from_dir(&dir).unwrap();
        let status = manager.status(Some("demo.service")).unwrap();

        assert_eq!(status[0].unit, "demo.service");
        assert_eq!(status[0].state, UnitState::Inactive);
        assert_eq!(status[0].description.as_deref(), Some("Demo service"));

        fs::remove_dir_all(&dir).unwrap();
    }
}
