use micro_os::memory::BumpAllocator;
use micro_os::scheduler::RoundRobinScheduler;
use micro_os::sync::SpinLock;
use micro_os::task::{Task, TaskState};
use micro_os::timer::Timer;

#[test]
fn round_robin_is_fair_and_completes_tasks() {
    let mut scheduler = RoundRobinScheduler::new();
    scheduler.add_task(Task::new(1, "a", 2));
    scheduler.add_task(Task::new(2, "b", 2));

    let order: Vec<_> = (0..4)
        .map(|_| scheduler.schedule_once().unwrap().task_id)
        .collect();

    assert_eq!(order, vec![1, 2, 1, 2]);
    assert!(scheduler.is_empty());
    assert_eq!(scheduler.completed, 2);
}

#[test]
fn task_transitions_to_terminated() {
    let mut task = Task::new(7, "test", 1);
    assert_eq!(task.state, TaskState::Ready);
    assert!(task.run_one_step());
    assert_eq!(task.state, TaskState::Terminated);
}

#[test]
fn timer_expires_at_quantum_boundary() {
    let mut timer = Timer::new(3);
    assert!(!timer.tick());
    assert!(!timer.tick());
    assert!(timer.tick());
    assert_eq!(timer.ticks, 3);
}

#[test]
fn allocator_respects_alignment_and_failure() {
    let mut allocator = BumpAllocator::new(32);
    assert_eq!(allocator.allocate(3, 1).unwrap().offset, 0);
    assert_eq!(allocator.allocate(4, 8).unwrap().offset, 8);
    assert!(allocator.allocate(100, 8).is_none());
    assert!(allocator.allocate(2, 3).is_none());
}

#[test]
fn spinlock_protects_shared_value() {
    let lock = SpinLock::new(0usize);
    {
        let mut value = lock.lock();
        *value += 1;
    }
    assert_eq!(*lock.lock(), 1);
}
