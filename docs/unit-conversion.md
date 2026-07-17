# Unit Conversion

`minitctl convert` produces reviewable `minit` TOML from a small subset of foreign init unit formats.

The converter is intentionally conservative. Unsupported fields are reported as warnings instead of being silently approximated.

## systemd Service Files

Convert a simple systemd service:

```sh
minitctl convert --from systemd ./sshd.service > sshd.service.toml
```

Supported systemd fields:

- `[Unit] Description=` -> `[unit].description`
- `[Unit] After=` -> `[dependencies].after`
- `[Unit] Before=` -> `[dependencies].before`
- `[Unit] Requires=` -> `[dependencies].requires`
- `[Unit] Wants=` -> `[dependencies].wants`
- `[Unit] Conflicts=` -> `[dependencies].conflicts`
- `[Service] ExecStart=` -> `[exec].start`
- `[Service] ExecReload=` -> `[exec].reload`
- `[Service] ExecStop=` -> `[exec].stop`
- `[Service] WorkingDirectory=` -> `[exec].working_directory`
- `[Service] Restart=always` -> `[restart].policy = "always"`
- `[Service] Restart=on-failure` -> `[restart].policy = "on-failure"`
- `[Service] User=` -> `[security].user`
- `[Service] Group=` -> `[security].group`
- `[Service] NoNewPrivileges=yes` -> `[security].no_new_privileges = true`
- `[Service] Environment=` -> `[security].environment`

Example input:

```ini
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
```

Example output:

```toml
[unit]
name = "sshd.service"
description = "OpenSSH daemon"
kind = "service"

[exec]
start = ["/usr/bin/sshd", "-D"]

[dependencies]
after = ["network-online.target"]
requires = ["network.target"]

[restart]
policy = "on-failure"

[security]
user = "root"
group = "root"
no_new_privileges = true
```

## Unsupported Sources

These source formats are detected but not converted in this release:

```sh
minitctl convert --from openrc ./sshd
minitctl convert --from runit ./run
minitctl convert --from s6 ./run
```

For OpenRC, runit, and s6, the command emits explicit warnings and no TOML. This keeps the first converter release honest while reserving the CLI shape for later implementation.

## Limitations

- Only service units are converted.
- `ExecStart=` parsing handles normal quoted command lines, not full shell syntax.
- systemd socket activation, timers, device units, mount units, user sessions, journald behavior, notify readiness, complex sandboxing, and capability rules are not converted.
- Unsupported fields appear as warnings on stderr.
- Always review the generated TOML and run `minit_core::unit` validation or the release gate before booting converted units.
