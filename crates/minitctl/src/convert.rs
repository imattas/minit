use clap::ValueEnum;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SourceFormat {
    Systemd,
    #[value(name = "openrc")]
    OpenRc,
    Runit,
    S6,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversion {
    pub toml: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConvertError {
    #[error("systemd service is missing Service.ExecStart")]
    MissingExecStart,
}

pub fn convert_unit(
    from: SourceFormat,
    name: &str,
    input: &str,
) -> Result<Conversion, ConvertError> {
    match from {
        SourceFormat::Systemd => convert_systemd(name, input),
        SourceFormat::OpenRc => Ok(unsupported_skeleton(
            "OpenRC",
            name,
            "command skeleton",
            input.contains("command=") || input.contains("command_args="),
        )),
        SourceFormat::Runit => Ok(unsupported_skeleton(
            "runit",
            name,
            "run script skeleton",
            input
                .lines()
                .any(|line| line.trim_start().starts_with("exec ")),
        )),
        SourceFormat::S6 => Ok(unsupported_skeleton(
            "s6",
            name,
            "execline run script skeleton",
            input.contains("execlineb"),
        )),
    }
}

fn unsupported_skeleton(
    format: &str,
    name: &str,
    detected: &str,
    detected_input: bool,
) -> Conversion {
    let warning = if detected_input {
        format!("unsupported {format} conversion for {name}; detected {detected}")
    } else {
        format!("unsupported {format} conversion for {name}; no recognized skeleton detected")
    };
    Conversion {
        toml: String::new(),
        warnings: vec![warning],
    }
}

#[derive(Default)]
struct SystemdModel {
    description: Option<String>,
    after: Vec<String>,
    before: Vec<String>,
    requires: Vec<String>,
    wants: Vec<String>,
    conflicts: Vec<String>,
    exec_start: Vec<String>,
    exec_reload: Vec<String>,
    exec_stop: Vec<String>,
    working_directory: Option<String>,
    restart: Option<String>,
    user: Option<String>,
    group: Option<String>,
    no_new_privileges: bool,
    environment: Vec<String>,
    warnings: Vec<String>,
}

fn convert_systemd(name: &str, input: &str) -> Result<Conversion, ConvertError> {
    let mut model = SystemdModel::default();
    let mut section = String::new();

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            model
                .warnings
                .push(format!("unsupported systemd line {line}"));
            continue;
        };
        apply_systemd_field(&mut model, &section, key.trim(), value.trim());
    }

    if model.exec_start.is_empty() {
        return Err(ConvertError::MissingExecStart);
    }

    Ok(Conversion {
        toml: render_minit_toml(name, &model),
        warnings: model.warnings,
    })
}

fn apply_systemd_field(model: &mut SystemdModel, section: &str, key: &str, value: &str) {
    match (section, key) {
        ("Unit", "Description") => model.description = Some(value.to_string()),
        ("Unit", "After") => extend_words(&mut model.after, value),
        ("Unit", "Before") => extend_words(&mut model.before, value),
        ("Unit", "Requires") => extend_words(&mut model.requires, value),
        ("Unit", "Wants") => extend_words(&mut model.wants, value),
        ("Unit", "Conflicts") => extend_words(&mut model.conflicts, value),
        ("Service", "ExecStart") => model.exec_start = split_command(value),
        ("Service", "ExecReload") => model.exec_reload = split_command(value),
        ("Service", "ExecStop") => model.exec_stop = split_command(value),
        ("Service", "WorkingDirectory") => model.working_directory = Some(value.to_string()),
        ("Service", "Restart") => match value {
            "always" | "on-failure" => model.restart = Some(value.to_string()),
            "no" => {}
            _ => model
                .warnings
                .push(format!("unsupported systemd field Service.Restart={value}")),
        },
        ("Service", "User") => model.user = Some(value.to_string()),
        ("Service", "Group") => model.group = Some(value.to_string()),
        ("Service", "NoNewPrivileges") => model.no_new_privileges = is_systemd_truthy(value),
        ("Service", "Environment") => model.environment.extend(split_environment(value)),
        _ => model
            .warnings
            .push(format!("unsupported systemd field {section}.{key}={value}")),
    }
}

fn extend_words(values: &mut Vec<String>, input: &str) {
    values.extend(input.split_whitespace().map(str::to_string));
}

fn is_systemd_truthy(value: &str) -> bool {
    matches!(value, "1" | "yes" | "true" | "on")
}

fn split_environment(value: &str) -> Vec<String> {
    split_command(value)
        .into_iter()
        .filter(|entry| entry.contains('='))
        .collect()
}

fn split_command(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                args.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        args.push(current);
    }
    args
}

fn render_minit_toml(name: &str, model: &SystemdModel) -> String {
    let mut output = String::new();
    output.push_str("[unit]\n");
    output.push_str(&format!("name = \"{}\"\n", toml_string(name)));
    if let Some(description) = &model.description {
        output.push_str(&format!("description = \"{}\"\n", toml_string(description)));
    }
    output.push_str("kind = \"service\"\n\n");

    output.push_str("[exec]\n");
    output.push_str(&format!("start = {}\n", toml_array(&model.exec_start)));
    if !model.exec_reload.is_empty() {
        output.push_str(&format!("reload = {}\n", toml_array(&model.exec_reload)));
    }
    if !model.exec_stop.is_empty() {
        output.push_str(&format!("stop = {}\n", toml_array(&model.exec_stop)));
    }
    if let Some(working_directory) = &model.working_directory {
        output.push_str(&format!(
            "working_directory = \"{}\"\n",
            toml_string(working_directory)
        ));
    }

    if has_dependencies(model) {
        output.push_str("\n[dependencies]\n");
        render_vec_field(&mut output, "after", &model.after);
        render_vec_field(&mut output, "before", &model.before);
        render_vec_field(&mut output, "requires", &model.requires);
        render_vec_field(&mut output, "wants", &model.wants);
        render_vec_field(&mut output, "conflicts", &model.conflicts);
    }

    if let Some(restart) = &model.restart {
        output.push_str("\n[restart]\n");
        output.push_str(&format!("policy = \"{}\"\n", toml_string(restart)));
    }

    if has_security(model) {
        output.push_str("\n[security]\n");
        if let Some(user) = &model.user {
            output.push_str(&format!("user = \"{}\"\n", toml_string(user)));
        }
        if let Some(group) = &model.group {
            output.push_str(&format!("group = \"{}\"\n", toml_string(group)));
        }
        if model.no_new_privileges {
            output.push_str("no_new_privileges = true\n");
        }
        render_vec_field(&mut output, "environment", &model.environment);
    }

    output
}

fn has_dependencies(model: &SystemdModel) -> bool {
    !model.after.is_empty()
        || !model.before.is_empty()
        || !model.requires.is_empty()
        || !model.wants.is_empty()
        || !model.conflicts.is_empty()
}

fn has_security(model: &SystemdModel) -> bool {
    model.user.is_some()
        || model.group.is_some()
        || model.no_new_privileges
        || !model.environment.is_empty()
}

fn render_vec_field(output: &mut String, name: &str, values: &[String]) {
    if !values.is_empty() {
        output.push_str(&format!("{name} = {}\n", toml_array(values)));
    }
}

fn toml_array(values: &[String]) -> String {
    let entries = values
        .iter()
        .map(|value| format!("\"{}\"", toml_string(value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{entries}]")
}

fn toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_simple_systemd_service_to_minit_toml() {
        let input = r#"
[Unit]
Description=OpenSSH daemon
After=network-online.target
Requires=network.target

[Service]
ExecStart=/usr/bin/sshd -D
Restart=on-failure
User=root
Group=root
NoNewPrivileges=yes
"#;

        let conversion = convert_unit(SourceFormat::Systemd, "sshd.service", input)
            .expect("simple systemd service should convert");

        assert!(conversion.warnings.is_empty());
        assert!(conversion.toml.contains("name = \"sshd.service\""));
        assert!(conversion.toml.contains("description = \"OpenSSH daemon\""));
        assert!(conversion
            .toml
            .contains("start = [\"/usr/bin/sshd\", \"-D\"]"));
        assert!(conversion
            .toml
            .contains("after = [\"network-online.target\"]"));
        assert!(conversion.toml.contains("requires = [\"network.target\"]"));
        assert!(conversion.toml.contains("policy = \"on-failure\""));
        assert!(conversion.toml.contains("user = \"root\""));
        assert!(conversion.toml.contains("group = \"root\""));
        assert!(conversion.toml.contains("no_new_privileges = true"));

        let unit = minit_core::unit::parse_unit_toml(&conversion.toml).unwrap();
        unit.validate().unwrap();
    }

    #[test]
    fn warns_for_unsupported_systemd_fields_without_silent_conversion() {
        let input = r#"
[Unit]
Description=Timer-ish service

[Service]
ExecStart=/usr/bin/example
Type=forking
PrivateTmp=yes
AmbientCapabilities=CAP_NET_BIND_SERVICE
"#;

        let conversion = convert_unit(SourceFormat::Systemd, "example.service", input)
            .expect("partially supported systemd service should still convert");

        assert_eq!(
            conversion.warnings,
            vec![
                "unsupported systemd field Service.Type=forking".to_string(),
                "unsupported systemd field Service.PrivateTmp=yes".to_string(),
                "unsupported systemd field Service.AmbientCapabilities=CAP_NET_BIND_SERVICE"
                    .to_string(),
            ]
        );
        assert!(!conversion.toml.contains("PrivateTmp"));
        assert!(conversion.toml.contains("start = [\"/usr/bin/example\"]"));
    }

    #[test]
    fn detects_openrc_runit_and_s6_skeletons_as_explicitly_unsupported() {
        let openrc = convert_unit(
            SourceFormat::OpenRc,
            "sshd",
            "command=/usr/sbin/sshd\ncommand_args=\"-D\"\n",
        )
        .expect("OpenRC skeleton should produce review output");
        assert!(openrc.toml.is_empty());
        assert_eq!(
            openrc.warnings,
            vec!["unsupported OpenRC conversion for sshd; detected command skeleton".to_string()]
        );

        let runit = convert_unit(
            SourceFormat::Runit,
            "run",
            "#!/bin/sh\nexec /usr/bin/sshd -D\n",
        )
        .expect("runit skeleton should produce review output");
        assert!(runit.toml.is_empty());
        assert_eq!(
            runit.warnings,
            vec!["unsupported runit conversion for run; detected run script skeleton".to_string()]
        );

        let s6 = convert_unit(
            SourceFormat::S6,
            "run",
            "#!/bin/execlineb -P\n/usr/bin/sshd -D\n",
        )
        .expect("s6 skeleton should produce review output");
        assert!(s6.toml.is_empty());
        assert_eq!(
            s6.warnings,
            vec![
                "unsupported s6 conversion for run; detected execline run script skeleton"
                    .to_string()
            ]
        );
    }
}
