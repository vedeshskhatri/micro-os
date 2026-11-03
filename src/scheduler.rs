use std::collections::VecDeque;

use crate::task::{Task, TaskState};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleEvent {
    pub task_id: usize,
    pub task_name: &'static str,
    pub finished: bool,
    pub queue_after: Vec<usize>,
}

pub struct RoundRobinScheduler {
    ready: VecDeque<Task>,
    pub switches: usize,
    pub completed: usize,
}

impl RoundRobinScheduler {
    pub fn new() -> Self {
        Self {
            ready: VecDeque::new(),
            switches: 0,
            completed: 0,
        }
    }

    pub fn add_task(&mut self, task: Task) {
        assert_eq!(task.state, TaskState::Ready);
        self.ready.push_back(task);
    }

    pub fn is_empty(&self) -> bool {
        self.ready.is_empty()
    }

    pub fn ready_ids(&self) -> Vec<usize> {
        self.ready.iter().map(|task| task.id).collect()
    }

    pub fn schedule_once(&mut self) -> Option<ScheduleEvent> {
        let mut task = self.ready.pop_front()?;
        self.switches += 1;
        let task_id = task.id;
        let task_name = task.name;
        let finished = task.run_one_step();
        if finished {
            self.completed += 1;
        } else {
            self.ready.push_back(task);
        }
        Some(ScheduleEvent {
            task_id,
            task_name,
            finished,
            queue_after: self.ready_ids(),
        })
    }
}

impl Default for RoundRobinScheduler {
    fn default() -> Self {
        Self::new()
    }
}
