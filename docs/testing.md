# Testing and Evidence

## Automated tests

Run native mechanism tests:

```powershell
cargo test
```

The integration suite validates:

- two-task Round Robin fairness;
- task termination state;
- timer quantum boundary;
- aligned allocation;
- allocation failure;
- spinlock-protected mutation.

Build and boot the actual kernel:

```powershell
cargo build
.\scripts\run_qemu.ps1
```

The QEMU serial trace validates:

- BIOS bootloader handoff to the Rust kernel;
- serial console initialization;
- IDT and PIC initialization;
- PIT timer configuration;
- timer-triggered scheduler decisions;
- task completion and queue exhaustion;
- kernel allocation;
- protected shared counter updates.

## Demonstration evidence

Run the native demonstration:

```powershell
cargo run --bin micro-os-demo | Tee-Object traces/demo.log
```

The trace provides evidence for:

- kernel entry;
- subsystem initialization;
- task creation;
- timer ticks;
- scheduler order;
- task termination;
- synchronized shared counter updates;
- successful memory allocation;
- final switch and completion counts.

Capture the real boot trace with:

```powershell
.\scripts\run_qemu.ps1 | Tee-Object traces\qemu-boot.log
python scripts\analyze_trace.py traces\qemu-boot.log
```

## Required experiment matrix

| Case | Expected observation |
| --- | --- |
| Two tasks | Alternating task selection until both terminate |
| Three tasks | Each ready task receives repeated turns |
| Short task | It terminates and leaves the queue |
| Large task | It remains ready across multiple turns |
| Invalid allocation | Request is rejected without changing allocator state |
| Repeated run | Same task order and completion counts |

## Report metrics

Record task switches, completion order, timer ticks, allocation usage, and final shared-counter value. These measurements can be parsed from `traces/demo.log` using Python in a later analysis milestone.
