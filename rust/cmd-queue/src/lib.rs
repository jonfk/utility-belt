use std::{sync::Arc, time::SystemTime};

use crate::{queue::Queue, task::TaskService};
use queue::InMemoryQueue;
use rayon::ThreadPoolBuilder;
use serde::{Deserialize, Serialize};
use workerpool::WorkerPool;

pub mod client;
pub mod constants;
pub mod error;
pub mod queue;
pub mod task;
pub mod workerpool;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommandRequest {
    pub path: String,
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandResponse {
    Success(CommandSuccess),
    Failed(CommandFailed),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSuccess {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandFailed {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Task {
    id: String,
    command: CommandRequest,
    tries: usize,
    last_attempt: Option<SystemTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskRunResult {
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum TaskState {
    Running,
    Queued,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueueState {
    NotEmpty,
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRequest {
    pub state_filters: Option<Vec<TaskState>>,
}

#[derive(Clone)]
pub struct CommandQApp {
    pub queue: Arc<dyn Queue + Send + Sync>,
    pub task_svc: Arc<TaskService>,
    pub worker_pool: Arc<WorkerPool>,
}

impl CommandQApp {
    pub fn new() -> Self {
        let queue = Arc::new(InMemoryQueue::new());
        let task_svc = Arc::new(TaskService::new(queue.clone()));

        let num_workers = 6;
        let thread_pool = ThreadPoolBuilder::new()
            .num_threads(num_workers)
            .build()
            .expect("failed building threadpool");
        let worker_pool = Arc::new(WorkerPool::new(task_svc.clone(), num_workers, thread_pool));
        worker_pool.spawn();

        CommandQApp {
            queue: queue,
            task_svc: task_svc,
            worker_pool: worker_pool,
        }
    }
}
