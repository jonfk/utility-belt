use std::{
    collections::HashMap,
    ops::Add,
    process::{Child, Command},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use crate::{
    error::CmdqError,
    execution::{delay, MAX_RETRIES},
    queue::InMemoryQueue,
    Task, TaskRunResult,
};

pub struct TaskScheduler {
    queue: Arc<InMemoryQueue>,
    num_workers: usize,
    //running_tasks: HashMap<String, Child>,
    num_running_tasks: Arc<Mutex<usize>>,
}

impl TaskScheduler {
    pub fn new(queue: Arc<InMemoryQueue>, num_workers: usize) -> Self {
        TaskScheduler {
            queue: queue.clone(),
            num_workers: num_workers,
            //running_tasks:
            num_running_tasks: Arc::new(Mutex::new(0)),
        }
    }
    pub fn run(&self) {
        let task_scheduler = Arc::new(self);

        std::thread::scope(|s| {
            let scheduler = task_scheduler.clone();
            s.spawn(move || loop {
                scheduler.run_loop();
                std::thread::sleep(Duration::from_secs(10));
            });
        });
        // std::thread::spawn(move || loop {
        //     self.run_loop();
        //     std::thread::sleep(Duration::from_secs(10));
        // });
    }
    pub fn run_loop(&self) {
        while *self.num_running_tasks.lock().unwrap() < self.num_workers {
            let task_opt = self.queue.pop_next();
            let queue = self.queue.clone();
            let num_running_tasks = self.num_running_tasks.clone();

            if let Some(task) = task_opt {
                std::thread::spawn(move || {
                    {
                        let mut num_running_tasks = num_running_tasks.lock().unwrap();
                        *num_running_tasks += 1;
                    }
                    run_task(task, queue);

                    {
                        let mut num_running_tasks = num_running_tasks.lock().unwrap();
                        *num_running_tasks -= 1;
                    }
                });
            } else {
                break;
            }
        }
    }
}

fn run_task(task: Task, queue: Arc<InMemoryQueue>) {
    println!("Running task {:?}", task);
    if task.tries > 1
        && task
            .last_attempt
            .map(|last_attempt| last_attempt.add(delay(task.tries as u32)) > SystemTime::now())
            .unwrap_or(false)
    {
        println!("Task not ready yet");
        if let Err(err) = queue.update(&task.id, TaskRunResult::Skipped) {
            println!("Error writing skipped task result");
        }
        return;
    }

    if task.tries > MAX_RETRIES {
        println!("Task was retried more than {}, skipping", MAX_RETRIES);
        return;
    }

    // TODO change to child and save child to enable killing tasks
    // See https://doc.rust-lang.org/std/process/struct.Child.html#method.wait_with_output on how to capture the output piped while the process is running
    let output_res = Command::new(&task.command.program)
        .args(&task.command.args)
        .current_dir(task.command.path)
        .output();

    // TODO write error to task
    let write_res = match output_res {
        Ok(output) => {
            println!("{:?}", output);
            if output.status.success() {
                queue.update(&task.id, TaskRunResult::Completed)
            } else {
                queue.update(&task.id, TaskRunResult::Failed)
            }
        }
        Err(_err) => queue.update(&task.id, TaskRunResult::Failed),
    };
    if write_res.is_err() {
        println!("Error writing task result");
    }
}
