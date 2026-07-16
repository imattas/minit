# VM Tests

The first VM milestone is:

1. Boot a Linux kernel with an initramfs containing `minitd` as `/init`.
2. Run `minitd` as PID 1 with `minit.rescue=1`.
3. Mount `/proc`, `/sys`, `/dev`, and `/run`.
4. Start `/bin/sh` or `/sbin/getty console`.
5. Reap children.
6. Shut down or exit cleanly.

The PowerShell scripts in `tools/vm/` are verification helpers. They require a Linux kernel image, a static BusyBox binary, `bash`, `cpio`, and `qemu-system-x86_64`.
