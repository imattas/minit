# VM Tests

The first VM milestone is:

1. Boot a Linux kernel with an initramfs containing `minitd` as `/init`.
2. Run `minitd` as PID 1 with `minit.rescue=1`.
3. Mount `/proc`, `/sys`, `/dev`, and `/run`.
4. Start `/bin/sh` or `/sbin/getty console`.
5. Reap children.
6. Shut down or exit cleanly.

The PowerShell scripts in `tools/vm/` are verification helpers. They require a Linux kernel image, a static BusyBox binary, `bash`, `cpio`, and `qemu-system-x86_64`.

Use the normal release gate for day-to-day verification. Use `-ExtendedVmStress -StressBootCount 25` before release-candidate builds to run a longer repeated boot/shutdown loop.

Run the Alpine minirootfs profile gate to validate `minit` against a real distro root filesystem:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\vm\verify-alpine-minirootfs.ps1 -Kernel C:\minit-vm\bzImage
```

That gate downloads the Alpine minirootfs tarball from the official Alpine CDN, verifies its SHA256 file, injects `minitd`, `minitctl`, and the `config/profiles/alpine-minirootfs` units, then boots the generated initramfs in QEMU.
