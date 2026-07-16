use minit_core::ipc::{
    decode_request, encode_response, ControlRequest, ControlResponse, WireError,
    DEFAULT_CONTROL_SOCKET,
};
use minit_core::manager::ServiceManager;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ControlError {
    #[error("invalid control request: {0}")]
    Request(#[from] WireError),
    #[error("failed to encode control response: {0}")]
    Response(WireError),
    #[error("control I/O failed: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlSocketConfig {
    pub socket_path: PathBuf,
    pub max_requests: Option<usize>,
    pub startup_command: Option<Vec<String>>,
}

impl Default for ControlSocketConfig {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from(DEFAULT_CONTROL_SOCKET),
            max_requests: None,
            startup_command: None,
        }
    }
}

pub trait ControlRuntime {
    fn start(&mut self, services: &mut ServiceManager, unit: &str) -> Result<String, String>;
    fn stop(&mut self, services: &mut ServiceManager, unit: &str) -> Result<String, String>;

    fn reap(&mut self, _services: &mut ServiceManager) -> Result<(), String> {
        Ok(())
    }

    fn restart(&mut self, services: &mut ServiceManager, unit: &str) -> Result<String, String> {
        self.stop(services, unit)?;
        self.start(services, unit)
    }
}

pub struct DisabledRuntime;

impl ControlRuntime for DisabledRuntime {
    fn start(&mut self, _services: &mut ServiceManager, unit: &str) -> Result<String, String> {
        Err(format!("start is not implemented yet for {unit}"))
    }

    fn stop(&mut self, _services: &mut ServiceManager, unit: &str) -> Result<String, String> {
        Err(format!("stop is not implemented yet for {unit}"))
    }
}

pub struct ControlService<R = DisabledRuntime> {
    services: ServiceManager,
    runtime: R,
}

impl ControlService<DisabledRuntime> {
    pub fn new(services: ServiceManager) -> Self {
        Self {
            services,
            runtime: DisabledRuntime,
        }
    }
}

impl<R: ControlRuntime> ControlService<R> {
    pub fn with_runtime(services: ServiceManager, runtime: R) -> Self {
        Self { services, runtime }
    }

    pub fn handle_request(&mut self, request: ControlRequest) -> ControlResponse {
        if let Err(message) = self.runtime.reap(&mut self.services) {
            return ControlResponse::Error { message };
        }

        match request {
            ControlRequest::Status { unit } => match self.services.status(unit.as_deref()) {
                Ok(units) => ControlResponse::Status { units },
                Err(error) => ControlResponse::Error {
                    message: error.to_string(),
                },
            },
            ControlRequest::Start { unit } => match self.runtime.start(&mut self.services, &unit) {
                Ok(message) => ControlResponse::Accepted { message },
                Err(message) => ControlResponse::Error { message },
            },
            ControlRequest::Stop { unit } => match self.runtime.stop(&mut self.services, &unit) {
                Ok(message) => ControlResponse::Accepted { message },
                Err(message) => ControlResponse::Error { message },
            },
            ControlRequest::Restart { unit } => {
                match self.runtime.restart(&mut self.services, &unit) {
                    Ok(message) => ControlResponse::Accepted { message },
                    Err(message) => ControlResponse::Error { message },
                }
            }
        }
    }
}

pub fn handle_control_request(request: ControlRequest) -> ControlResponse {
    match request {
        ControlRequest::Status { .. } => ControlResponse::Status { units: Vec::new() },
        ControlRequest::Start { unit } => not_implemented("start", &unit),
        ControlRequest::Stop { unit } => not_implemented("stop", &unit),
        ControlRequest::Restart { unit } => not_implemented("restart", &unit),
    }
}

pub fn handle_control_line(line: &str) -> Result<String, ControlError> {
    let request = decode_request(line)?;
    let response = handle_control_request(request);
    encode_response(&response).map_err(ControlError::Response)
}

pub fn handle_control_line_with_service(
    service: &mut ControlService<impl ControlRuntime>,
    line: &str,
) -> Result<String, ControlError> {
    let request = decode_request(line)?;
    let response = service.handle_request(request);
    encode_response(&response).map_err(ControlError::Response)
}

pub fn handle_control_io<R, W>(reader: &mut R, writer: &mut W) -> Result<(), ControlError>
where
    R: BufRead,
    W: Write,
{
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let response_line = handle_control_line(&line)?;
    writer.write_all(response_line.as_bytes())?;
    writer.flush()?;
    Ok(())
}

pub fn handle_control_io_with_service<R, W>(
    service: &mut ControlService<impl ControlRuntime>,
    reader: &mut R,
    writer: &mut W,
) -> Result<(), ControlError>
where
    R: BufRead,
    W: Write,
{
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let response_line = handle_control_line_with_service(service, &line)?;
    writer.write_all(response_line.as_bytes())?;
    writer.flush()?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn run_control_socket_once(
    config: &ControlSocketConfig,
    service: &mut ControlService<impl ControlRuntime>,
) -> Result<(), ControlError> {
    let mut one_request = config.clone();
    one_request.max_requests = Some(1);
    run_control_socket(&one_request, service)
}

#[cfg(target_os = "linux")]
pub fn run_control_socket(
    config: &ControlSocketConfig,
    service: &mut ControlService<impl ControlRuntime>,
) -> Result<(), ControlError> {
    use std::io::BufReader;
    use std::os::unix::net::UnixListener;

    if let Some(parent) = config.socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if config.socket_path.exists() {
        std::fs::remove_file(&config.socket_path)?;
    }

    let listener = UnixListener::bind(&config.socket_path)?;
    eprintln!(
        "minitd: normal mode ready; control socket {}",
        config.socket_path.display()
    );
    let mut startup_child = match &config.startup_command {
        Some(argv) if !argv.is_empty() => {
            let child = std::process::Command::new(&argv[0])
                .args(&argv[1..])
                .spawn()?;
            Some(child)
        }
        _ => None,
    };
    for stream in listener
        .incoming()
        .take(config.max_requests.unwrap_or(usize::MAX))
    {
        let stream = stream?;
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut writer = stream;
        handle_control_io_with_service(service, &mut reader, &mut writer)?;
    }
    if let Some(child) = startup_child.as_mut() {
        let _ = child.wait();
    }
    Ok(())
}

fn not_implemented(command: &str, unit: &str) -> ControlResponse {
    ControlResponse::Error {
        message: format!("{command} is not implemented yet for {unit}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minit_core::ipc::{decode_response, encode_request, UnitState};
    use minit_core::manager::ServiceManager;
    use minit_core::unit::parse_unit_toml;

    #[test]
    fn status_request_returns_empty_status_until_manager_exists() {
        let response = handle_control_request(ControlRequest::Status { unit: None });

        assert_eq!(response, ControlResponse::Status { units: Vec::new() });
    }

    #[test]
    fn lifecycle_requests_return_explicit_unimplemented_errors() {
        let response = handle_control_request(ControlRequest::Start {
            unit: "sshd".to_string(),
        });

        assert_eq!(
            response,
            ControlResponse::Error {
                message: "start is not implemented yet for sshd".to_string()
            }
        );
    }

    #[test]
    fn handles_one_wire_request_line() {
        let request = encode_request(&ControlRequest::Status {
            unit: Some("sshd".to_string()),
        })
        .unwrap();

        let response_line = handle_control_line(&request).unwrap();
        let response = decode_response(&response_line).unwrap();

        assert_eq!(response, ControlResponse::Status { units: Vec::new() });
    }

    #[test]
    fn handles_one_buffered_request() {
        let request = encode_request(&ControlRequest::Start {
            unit: "sshd".to_string(),
        })
        .unwrap();
        let mut reader = std::io::BufReader::new(request.as_bytes());
        let mut writer = Vec::new();

        handle_control_io(&mut reader, &mut writer).unwrap();

        let response = String::from_utf8(writer).unwrap();
        let decoded = decode_response(&response).unwrap();
        assert_eq!(
            decoded,
            ControlResponse::Error {
                message: "start is not implemented yet for sshd".to_string()
            }
        );
    }

    fn service_manager_with_sshd() -> ServiceManager {
        let unit = parse_unit_toml(
            r#"
[unit]
name = "sshd.service"
description = "OpenSSH daemon"

[exec]
start = ["/usr/bin/sshd", "-D"]
"#,
        )
        .unwrap();
        let mut services = ServiceManager::new();
        services.add_unit(unit).unwrap();
        services
    }

    #[test]
    fn control_service_reports_registered_unit_status() {
        let mut service = ControlService::new(service_manager_with_sshd());

        let response = service.handle_request(ControlRequest::Status {
            unit: Some("sshd.service".to_string()),
        });

        assert_eq!(
            response,
            ControlResponse::Status {
                units: vec![minit_core::ipc::UnitStatus {
                    unit: "sshd.service".to_string(),
                    state: UnitState::Inactive,
                    main_pid: None,
                    description: Some("OpenSSH daemon".to_string()),
                }]
            }
        );
    }

    #[test]
    fn control_service_reports_unknown_unit_errors() {
        let mut service = ControlService::new(ServiceManager::new());

        let response = service.handle_request(ControlRequest::Status {
            unit: Some("missing.service".to_string()),
        });

        assert_eq!(
            response,
            ControlResponse::Error {
                message: "unit missing.service was not found".to_string()
            }
        );
    }

    #[derive(Default)]
    struct FakeRuntime {
        calls: Vec<String>,
    }

    impl ControlRuntime for FakeRuntime {
        fn start(&mut self, services: &mut ServiceManager, unit: &str) -> Result<String, String> {
            self.calls.push(format!("start:{unit}"));
            services
                .mark_active(unit, 123)
                .map_err(|err| err.to_string())?;
            Ok(format!("started {unit} as pid 123"))
        }

        fn stop(&mut self, services: &mut ServiceManager, unit: &str) -> Result<String, String> {
            self.calls.push(format!("stop:{unit}"));
            services
                .mark_inactive(unit)
                .map_err(|err| err.to_string())?;
            Ok(format!("stopped {unit}"))
        }
    }

    #[test]
    fn control_service_start_uses_runtime_and_updates_status() {
        let mut service =
            ControlService::with_runtime(service_manager_with_sshd(), FakeRuntime::default());

        let response = service.handle_request(ControlRequest::Start {
            unit: "sshd.service".to_string(),
        });
        let status = service.handle_request(ControlRequest::Status {
            unit: Some("sshd.service".to_string()),
        });

        assert_eq!(
            response,
            ControlResponse::Accepted {
                message: "started sshd.service as pid 123".to_string(),
            }
        );
        assert_eq!(
            status,
            ControlResponse::Status {
                units: vec![minit_core::ipc::UnitStatus {
                    unit: "sshd.service".to_string(),
                    state: UnitState::Active,
                    main_pid: Some(123),
                    description: Some("OpenSSH daemon".to_string()),
                }]
            }
        );
    }

    #[test]
    fn control_service_restart_stops_then_starts_unit() {
        let mut service =
            ControlService::with_runtime(service_manager_with_sshd(), FakeRuntime::default());

        let response = service.handle_request(ControlRequest::Restart {
            unit: "sshd.service".to_string(),
        });

        assert_eq!(
            response,
            ControlResponse::Accepted {
                message: "started sshd.service as pid 123".to_string(),
            }
        );
    }

    #[test]
    fn control_socket_config_uses_default_socket() {
        let config = ControlSocketConfig::default();

        assert_eq!(
            config.socket_path,
            std::path::PathBuf::from(minit_core::ipc::DEFAULT_CONTROL_SOCKET)
        );
        assert_eq!(config.max_requests, None);
        assert_eq!(config.startup_command, None);
    }
}
