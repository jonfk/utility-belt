use std::{process::Command, sync::Arc, thread, time::Duration};

use crate::{queue::Queue, TaskRanState, TaskState};

const MAX_RETRIES: usize = 10;
const MAX_DELAY_SECONDS: u64 = 300;
const DELAY_SECONDS: u64 = 2;

pub struct TaskService {
    pub queue: Arc<dyn Queue + Send + Sync>,
}

impl TaskService {
    pub fn new(queue: Arc<dyn Queue + Send + Sync>) -> Self {
        TaskService {
            queue
        }
    }
    pub fn run_next_task(&self) -> TaskRanState {
        if let Some((task_id, task)) = self.queue.pop_next() {
            println!("Running task {:?}", task);
            if task.tries > 1 {
                thread::sleep(delay(task.tries as u32));
            }

            if task.tries > MAX_RETRIES {
                println!("Task was retried more than {}, skipping", MAX_RETRIES);
                return TaskRanState::Skipped;
            }

            let output_res = Command::new(&task.command.program)
                .args(&task.command.args)
                .current_dir(task.command.path)
                .output();

            match output_res {
                Ok(output) => {
                    if output.status.success() {
                        self.queue.update(task_id, TaskState::Completed);
                    } else {
                        self.queue.update(task_id, TaskState::Failed);
                    }
                    println!("{:?}", output);
                }
                Err(_err) => self.queue.update(task_id, TaskState::Failed),
            }
            TaskRanState::Completed
        } else {
            TaskRanState::Empty
        }
    }
}

fn delay(tries: u32) -> Duration {
    let delay = DELAY_SECONDS.pow(tries);
    if delay > MAX_DELAY_SECONDS {
        Duration::from_secs(MAX_DELAY_SECONDS)
    } else {
        Duration::from_secs(delay)
    }
}
