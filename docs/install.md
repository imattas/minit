# minit Install and Rollback Notes

`minit` is still experimental. Test it in a disposable VM first, then only try it on a host with a known-good rescue boot path.

## Package Layout

The release package contains:

- `bin/minitd`
- `bin/minitctl`
- `etc/minit/services/*.toml`
- `install/install.md`
- `docs/release-template.md`
- `SHA256SUMS`

## Install Sketch

1. Verify `SHA256SUMS` before copying files.
2. Copy `bin/minitd` and `bin/minitctl` into the initramfs build root.
3. Copy desired unit files into `/etc/minit/services`.
4. Set the kernel command line to use `init=/init minit.normal=1 minit.unit_dir=/etc/minit/services`.
5. Keep a known-good rescue initramfs and boot entry available.

Do not replace the only boot entry. Add a new experimental boot entry that points at the `minit` initramfs, and keep the previous entry as the default until the VM and host smoke checks pass.

## Initramfs Integration

For local VM evaluation, use:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\build-initramfs.ps1 `
  -MinitdPath target\x86_64-unknown-linux-musl\release\minitd `
  -MinitctlPath target\x86_64-unknown-linux-musl\release\minitctl `
  -BusyBoxPath C:\minit-vm\busybox `
  -UnitDir config\examples `
  -Output tools\vm\artifacts\minit-normal-initramfs.cpio
```

## Rollback

Keep the previous initramfs and bootloader entry.

Rollback path:

1. Boot the previous entry or a rescue shell that does not require `minit.normal=1`.
2. Restore the prior initramfs if it was replaced.
3. Remove `minit.normal=1`, `minit.unit_dir=...`, and any experimental `minit.smoke_*` arguments from the failing boot entry.
4. Reboot into the known-good entry.
5. Inspect the failed `minit` unit files offline before trying a new package.

If the control socket is available before rollback, collect quick state first:

```sh
/bin/minitctl status
/bin/minitctl boot-timeline
/bin/minitctl events
```
