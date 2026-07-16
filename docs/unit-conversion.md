# Unit Conversion Notes

This document maps simple systemd and OpenRC services into `minit` TOML units.

## systemd Service

Common source fields:

- `Description=` maps to `[unit].description`.
- `ExecStart=` maps to `[exec].start`.
- `WorkingDirectory=` maps to `[exec].working_directory`.
- `Restart=always` maps to `[restart].policy = "always"`.
- `Restart=on-failure` maps to `[restart].policy = "on-failure"`.
- `After=` maps to `[dependencies].after`.
- `Requires=` maps to `[dependencies].requires`.
- `Wants=` maps to `[dependencies].wants`.

Unsupported systemd features must be omitted or represented by explicit follow-up work. Do not silently approximate sandboxing, socket activation, timers, device units, user sessions, or journald behavior.

## OpenRC Service

Common source fields:

- `description` maps to `[unit].description`.
- `command` plus `command_args` maps to `[exec].start`.
- `directory` maps to `[exec].working_directory`.
- `depend() { need ... }` maps to `[dependencies].requires`.
- `depend() { want ... }` maps to `[dependencies].wants`.
- `depend() { after ... }` maps to `[dependencies].after`.

## Example

```toml
[unit]
name = "example.service"
description = "Example daemon"
kind = "service"

[exec]
start = ["/usr/bin/exampled", "--foreground"]
working_directory = "/"

[dependencies]
after = ["networking.service"]
requires = ["networking.service"]

[restart]
policy = "on-failure"
limit = "5/min"
backoff = "exponential"
max_delay = "30s"

[security]
no_new_privileges = true
```
