use crate::memory::BumpAllocator;
use crate::scheduler::RoundRobinScheduler;
use crate::sync::SpinLock;
use crate::task::Task;
use crate::timer::Timer;

pub struct Kernel {
    pub scheduler: RoundRobinScheduler,
    pub timer: Timer,
    pub allocator: BumpAllocator,
    shared_counter: SpinLock<usize>,
}

impl Kernel {
    pub fn new() -> Self {
        Self {
            scheduler: RoundRobinScheduler::new(),
            timer: Timer::new(1),
            allocator: BumpAllocator::new(1024),
            shared_counter: SpinLock::new(0),
        }
    }

    pub fn create_task(&mut self, id: usize, name: &'static str, steps: usize) {
        self.scheduler.add_task(Task::new(id, name, steps));
    }

    pub fn increment_shared_counter(&self) -> usize {
        let mut counter = self.shared_counter.lock();
        *counter += 1;
        *counter
    }

    pub fn shared_counter(&self) -> usize {
        *self.shared_counter.lock()
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}
