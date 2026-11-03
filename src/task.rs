#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Terminated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Task {
    pub id: usize,
    pub name: &'static str,
    pub state: TaskState,
    pub total_steps: usize,
    pub remaining_steps: usize,
}

impl Task {
    pub fn new(id: usize, name: &'static str, steps: usize) -> Self {
        assert!(steps > 0, "a task must have at least one step");
        Self {
            id,
            name,
            state: TaskState::Ready,
            total_steps: steps,
            remaining_steps: steps,
        }
    }

    pub fn run_one_step(&mut self) -> bool {
        self.state = TaskState::Running;
        self.remaining_steps -= 1;
        let finished = self.remaining_steps == 0;
        self.state = if finished {
            TaskState::Terminated
        } else {
            TaskState::Ready
        };
        finished
    }
}
