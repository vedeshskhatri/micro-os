# Architecture

## Control flow

```text
QEMU BIOS bootloader
      |
      v
`kernel/src/main.rs` Rust `no_std` entry
      |
      +--> UART 16550 serial console
      |
      +--> IDT + PIC + PIT timer interrupt
      |       |
      |       +--> Round Robin ready queue
      |               |
      |               +--> Task: runs one controlled step
      |
      +--> Spinlock: protects shared counter and queue
      |
      +--> Bump allocator: returns aligned memory regions
      |
      v
QEMU serial trace and completion summary
```

## Modules

| Module | Responsibility |
| --- | --- |
| Root `src/task.rs` | Host-test task identity, state, and work units |
| Root `src/scheduler.rs` | Host-test ready queue and Round Robin selection |
| Root `src/timer.rs` | Host-test tick count and quantum expiry |
| Root `src/sync.rs` | Host-test atomic spinlock and scoped guard |
| Root `src/memory.rs` | Host-test aligned bump allocation and failure handling |
| `kernel/src/main.rs` | Real no_std kernel, timer interrupt, queue, lock, allocator, and serial output |
| `build.rs` | Builds the kernel artifact and creates the BIOS disk image |

## Scheduling invariant

A task enters the ready queue only in `Ready` state. Each scheduling decision removes exactly one task. A task with remaining work returns to the back of the queue; a task with no remaining work becomes `Terminated` and is not requeued.

## Synchronization invariant

Only the holder of a `SpinLockGuard` can access the protected value mutably. Dropping the guard releases the atomic lock with `Release` ordering.

## Memory invariant

`next` always points to the first unused byte. Allocations are monotonic, aligned to a power-of-two boundary, and never exceed the heap capacity. This is intentionally a bump allocator: individual blocks are not freed.

## Deliberate educational boundary

The scheduler runs controlled task work units rather than switching independent CPU stacks. This makes Round Robin visible and repeatable without claiming full process isolation. A future extension can add saved register contexts and separate task stacks.
