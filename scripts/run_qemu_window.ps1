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

if (-not (Test-Path 'target\micro-os-bios.img')) {
    cargo build
}

& $qemuPath '-drive' 'format=raw,file=target\micro-os-bios.img' '-serial' 'stdio' '-display' 'gtk' '-no-reboot'
