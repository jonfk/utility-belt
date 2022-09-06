use std::{sync::Arc, time::SystemTime};

//use crate::task::TaskService;
use constants::DEFAULT_CONCURRENCY_LEVEL;
use error::CmdqError;
use execution::scheduler::TaskScheduler;
use queue::InMemoryQueue;
use rayon::ThreadPoolBuilder;
use serde::{Deserialize, Serialize};
//use workerpool::WorkerPool;

pub mod cli_util;
pub mod client;
pub mod constants;
pub mod error;
pub mod execution;
pub mod queue;
//pub mod task;
pub mod web;
//pub mod workerpool;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Clone)]
pub struct CommandQApp {
    pub queue: Arc<InMemoryQueue>,
    pub task_scheduler: Arc<TaskScheduler>,
    // pub task_svc: Arc<TaskService>,
    // pub worker_pool: Arc<WorkerPool>,
}

impl CommandQApp {
    pub fn new() -> Result<Self, CmdqError> {
        let queue = Arc::new(InMemoryQueue::new()?);
        //let task_svc = Arc::new(TaskService::new(queue.clone()));

        let num_workers = DEFAULT_CONCURRENCY_LEVEL;
        // let thread_pool = ThreadPoolBuilder::new()
        //     .num_threads(num_workers)
        //     .build()
        //     .expect("failed building threadpool");
        // let worker_pool = Arc::new(WorkerPool::new(task_svc.clone(), num_workers, thread_pool));
        // worker_pool.spawn();
        let task_scheduler = Arc::new(TaskScheduler::new(queue.clone(), num_workers));
        task_scheduler.clone().run();

        Ok(CommandQApp {
            queue: queue,
            task_scheduler: task_scheduler,
            // task_svc: task_svc,
            // worker_pool: worker_pool,

        })
    }
}
