use std::{
    ops::Add,
    process::Command,
    sync::Arc,
    time::{Duration, SystemTime},
};

use crate::{error::CmdqError, queue::InMemoryQueue, QueueState, TaskRunResult};

const MAX_RETRIES: usize = 20;
const MAX_DELAY_SECONDS: u64 = 600;
const DELAY_SECONDS: u64 = 2;

pub struct TaskService {
    queue: Arc<InMemoryQueue>,
}

impl TaskService {
    pub fn new(queue: Arc<InMemoryQueue>) -> Self {
        TaskService { queue }
    }
    pub fn run_next_task(&self) -> Result<QueueState, CmdqError> {
        if let Some(task) = self.queue.pop_next() {
            println!("Running task {:?}", task);
            if task.tries > 1
                && task
                    .last_attempt
                    .map(|last_attempt| {
                        last_attempt.add(delay(task.tries as u32)) > SystemTime::now()
                    })
                    .unwrap_or(false)
            {
                self.queue.update(&task.id, TaskRunResult::Skipped)?;
                return Ok(QueueState::NotEmpty);
            }

            if task.tries > MAX_RETRIES {
                println!("Task was retried more than {}, skipping", MAX_RETRIES);
                return Ok(QueueState::NotEmpty);
            }

            let output_res = Command::new(&task.command.program)
                .args(&task.command.args)
                .current_dir(task.command.path)
                .output();

            match output_res {
                Ok(output) => {
                    if output.status.success() {
                        self.queue.update(&task.id, TaskRunResult::Completed)?;
                    } else {
                        self.queue.update(&task.id, TaskRunResult::Failed)?;
                    }
                    println!("{:?}", output);
                }
                Err(_err) => self.queue.update(&task.id, TaskRunResult::Failed)?,
            }
            Ok(QueueState::NotEmpty)
        } else {
            Ok(QueueState::Empty)
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
