use micro_os::kernel::Kernel;

fn main() {
    println!("Micro-OS educational kernel demo");
    println!("[BOOT] Rust kernel entry reached");
    println!("[INIT] console, timer, scheduler, memory and synchronization ready");

    let mut kernel = Kernel::new();
    kernel.create_task(1, "shell", 3);
    kernel.create_task(2, "logger", 2);
    kernel.create_task(3, "worker", 4);

    println!("[TASK] created task 1: shell (3 steps)");
    println!("[TASK] created task 2: logger (2 steps)");
    println!("[TASK] created task 3: worker (4 steps)");

    while !kernel.scheduler.is_empty() {
        let quantum_expired = kernel.timer.tick();
        let event = kernel
            .scheduler
            .schedule_once()
            .expect("queue was not empty");
        let counter = kernel.increment_shared_counter();
        println!(
            "[SCHED] tick={} task={} ({}) quantum_expired={} shared_counter={} queue={:?}{}",
            kernel.timer.ticks,
            event.task_id,
            event.task_name,
            quantum_expired,
            counter,
            event.queue_after,
            if event.finished { " TERMINATED" } else { "" }
        );
    }

    let allocation = kernel
        .allocator
        .allocate(128, 8)
        .expect("demo allocation should fit");
    println!(
        "[MEM] allocated {} bytes at offset {}; used={}/{}",
        allocation.size,
        allocation.offset,
        kernel.allocator.used(),
        kernel.allocator.capacity()
    );
    println!(
        "[DONE] switches={} completed_tasks={} final_counter={}",
        kernel.scheduler.switches,
        kernel.scheduler.completed,
        kernel.shared_counter()
    );
}
