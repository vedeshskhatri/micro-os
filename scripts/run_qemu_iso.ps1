$ErrorActionPreference = 'Stop'

$qemu = Get-Command qemu-system-x86_64 -ErrorAction SilentlyContinue
if (-not $qemu) {
    $qemuPath = 'C:\Program Files\qemu\qemu-system-x86_64.exe'
    if (-not (Test-Path $qemuPath)) {
        throw 'QEMU was not found. Install QEMU or add it to PATH.'
    }
} else {
    $qemuPath = $qemu.Source
}

$firmware = 'C:\Program Files\qemu\share\edk2-x86_64-code.fd'
if (-not (Test-Path $firmware)) {
    throw 'QEMU UEFI firmware was not found.'
}
if (-not (Test-Path 'target\micro-os.iso')) {
    cargo build
}

& $qemuPath '-drive' "if=pflash,format=raw,readonly=on,file=$firmware" '-cdrom' 'target\micro-os.iso' '-serial' 'stdio' '-display' 'none' '-no-reboot' '-device' 'isa-debug-exit,iobase=0xf4,iosize=0x04'