# minit Install and Rollback Notes

`minit` is still experimental. Test it in a disposable VM before trying it on a recoverable host.

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

Keep the previous initramfs and bootloader entry. To roll back, boot the previous entry or rescue shell, restore the prior initramfs, and remove the `minit.normal=1` boot argument.
