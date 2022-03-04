use std::sync::Arc;

use rayon::ThreadPool;

use crate::task::TaskService;

pub struct WorkerPool {
    task_svc: Arc<TaskService>,
    num_workers: usize,
    thread_pool: ThreadPool,
}

impl WorkerPool {
    pub fn new(task_svc: Arc<TaskService>, num_workers: usize, thread_pool: ThreadPool) -> Self {
        WorkerPool {
            task_svc,
            num_workers,
            thread_pool,
        }
    }
    pub fn spawn(&self) {
        for _i in 0..self.num_workers {
            let task_svc = self.task_svc.clone();
            self.thread_pool.spawn(move || loop {
                match task_svc.run_next_task() {
                    Ok(_) => {}
                    Err(e) => println!("Error running task {}", e),
                }
            });
        }
    }
}
