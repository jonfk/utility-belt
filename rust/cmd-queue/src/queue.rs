use std::time::SystemTime;

use crossbeam::queue::SegQueue;
use dashmap::DashMap;
use nanoid::nanoid;

use crate::{CommandRequest, Task, TaskRunResult, TaskState};

const NANOID_ALPHABET: [char; 16] = [
    '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f',
];

pub fn generate_task_id() -> String {
    nanoid!(10, &NANOID_ALPHABET)
}

pub trait Queue {
    fn push_cmd(&self, command: &CommandRequest) -> Task {
        let task = Task {
            id: generate_task_id(),
            command: command.clone(),
            ..Default::default()
        };
        self.push(task.clone());
        task
    }
    fn push(&self, task: Task);
    fn pop_next(&self) -> Option<Task>;
    fn update(&self, id: &str, state: TaskRunResult);
    fn list(&self, state_filters: Vec<TaskState>) -> Vec<Task>;
    fn query(&self, id: &str) -> Option<(Task, TaskState)>;
}

pub struct InMemoryQueue {
    queue: SegQueue<Task>,
    running: DashMap<String, Task>,
}

impl InMemoryQueue {
    pub fn new() -> Self {
        InMemoryQueue {
            queue: SegQueue::new(),
            running: DashMap::new(),
        }
    }
}

impl Queue for InMemoryQueue {
    fn push(&self, task: Task) {
        self.queue.push(task);
    }

    fn pop_next(&self) -> Option<Task> {
        if let Some(mut task) = self.queue.pop() {
            task.tries += 1;

            self.running.insert(task.id.clone(), task.clone());
            Some(task)
        } else {
            None
        }
    }

    fn update(&self, id: &str, state: TaskRunResult) {
        match state {
            TaskRunResult::Completed => {
                self.running.remove(id);
            }
            TaskRunResult::Failed => {
                let (_id, mut task) = self.running.remove(id).expect("task does not exist");
                task.last_attempt = Some(SystemTime::now());
                self.queue.push(task);
            }
            TaskRunResult::Skipped => {
                let (_id, task) = self.running.remove(id).expect("task does not exist");
                self.queue.push(task);
            }
        }
    }

    fn list(&self, state_filters: Vec<TaskState>) -> Vec<Task> {
        let mut tasks: Vec<Task> = Vec::new();
        if state_filters.contains(&TaskState::Running) || state_filters.is_empty() {
            let mut running_tasks = self
                .running
                .iter()
                .map(|mapref| mapref.value().clone())
                .collect::<Vec<_>>();
            tasks.append(&mut running_tasks);
        }
        tasks
    }

    fn query(&self, id: &str) -> Option<(Task, TaskState)> {
        self.running
            .get(id)
            .map(|task| (task.clone(), TaskState::Running))
    }
}
