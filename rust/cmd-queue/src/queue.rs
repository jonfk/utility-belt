use std::time::SystemTime;

use crossbeam::queue::SegQueue;
use dashmap::DashMap;
use nanoid::nanoid;

use crate::{CommandRequest, Task, TaskRunState, TaskState};

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
    fn update(&self, id: &str, state: TaskRunState);
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
            task.last_attempt = Some(SystemTime::now());

            self.running.insert(task.id.clone(), task.clone());
            Some(task)
        } else {
            None
        }
    }

    fn update(&self, id: &str, state: TaskRunState) {
        match state {
            TaskRunState::Completed => {
                self.running.remove(id);
            }
            TaskRunState::Failed => {
                let (_id, task) = self.running.remove(id).expect("task does not exist");
                self.queue.push(task);
            }
        }
    }

    fn list(&self, state_filters: Vec<TaskState>) -> Vec<Task> {
        let mut tasks: Vec<Task> = Vec::new();
        if state_filters.contains(&TaskState::Running) || state_filters.is_empty() {
            self.running
                .iter()
                .map(|mapref| mapref.value().clone())
                .collect::<Vec<_>>()
                .append(&mut tasks);
        }
        tasks
    }

    fn query(&self, id: &str) -> Option<(Task, TaskState)> {
        self.running
            .get(id)
            .map(|task| (task.clone(), TaskState::Running))
    }
}
