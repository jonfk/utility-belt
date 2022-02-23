use std::{sync::Mutex, time::SystemTime};

use crossbeam::queue::SegQueue;
use dashmap::DashMap;

use crate::{CommandRequest, Task, TaskState};

pub trait Queue {
    fn push_cmd(&self, command: &CommandRequest) {
        self.push(Task {
            command: command.clone(),
            ..Default::default()
        });
    }
    fn push(&self, task: Task);
    fn pop_next(&self) -> Option<(usize, Task)>;
    fn update(&self, id: usize, state: TaskState);
}

pub struct InMemoryQueue {
    queue: SegQueue<Task>,
    running: DashMap<usize, Task>,
    index: Mutex<usize>,
}

impl InMemoryQueue {
    pub fn new() -> Self {
        InMemoryQueue {
            queue: SegQueue::new(),
            running: DashMap::new(),
            index: Mutex::new(0),
        }
    }
}

impl Queue for InMemoryQueue {
    fn push(&self, task: Task) {
        self.queue.push(task);
    }

    fn pop_next(&self) -> Option<(usize, Task)> {
        if let Some(mut task) = self.queue.pop() {
            task.tries += 1;
            task.last_attempt = Some(SystemTime::now());

            let mut index = self.index.lock().unwrap();
            *index += 1;
            let id = *index;
            self.running.insert(id, task.clone());
            Some((id, task))
        } else {
            None
        }
    }

    fn update(&self, id: usize, state: TaskState) {
        match state {
            TaskState::Completed => {
                self.running.remove(&id);
            }
            TaskState::Failed => {
                let (_id, task) = self.running.remove(&id).expect("task does not exist");
                self.queue.push(task);
            }
        }
    }
}
