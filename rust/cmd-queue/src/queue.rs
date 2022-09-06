use std::{path::Path, sync::RwLock, time::SystemTime};

use crossbeam::queue::SegQueue;
use dashmap::DashMap;
use nanoid::nanoid;
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};

use crate::{constants, error::CmdqError, CommandRequest, Task, TaskRunResult};

const NANOID_ALPHABET: [char; 16] = [
    '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f',
];

pub fn generate_task_id() -> String {
    nanoid!(10, &NANOID_ALPHABET)
}

pub struct InMemoryQueue {
    queue: SegQueue<Task>,
    running: DashMap<String, Task>,
    pickledb: RwLock<PickleDb>,
}

impl InMemoryQueue {
    pub fn new() -> Result<Self, CmdqError> {
        let queue = SegQueue::new();

        let db_file_path = constants::DBFILE;
        let pickledb = if Path::new(db_file_path).exists() {
            let db = PickleDb::load(
                db_file_path,
                PickleDbDumpPolicy::AutoDump,
                SerializationMethod::Bin,
            )
            .map_err(|e| CmdqError::PickleLoadDbError(db_file_path.to_string(), e))?;
            db.iter()
                .filter_map(|item| item.get_value::<Task>())
                .for_each(|task| queue.push(task));
            db
        } else {
            PickleDb::new(
                db_file_path,
                PickleDbDumpPolicy::AutoDump,
                SerializationMethod::Bin,
            )
        };
        Ok(InMemoryQueue {
            queue: queue,
            running: DashMap::new(),
            pickledb: RwLock::new(pickledb),
        })
    }

    pub fn push_cmd(&self, command: &CommandRequest) -> Result<Task, CmdqError> {
        let task = Task {
            id: generate_task_id(),
            command: command.clone(),
            ..Default::default()
        };
        self.push(task.clone())?;
        Ok(task)
    }

    fn push(&self, task: Task) -> Result<(), CmdqError> {
        self.queue.push(task.clone());
        let mut pickledb = self.pickledb.write().unwrap();
        pickledb
            .set(&task.id, &task)
            .map_err(|e| CmdqError::PickleDbWriteError(e))?;
        Ok(())
    }

    pub fn pop_next(&self) -> Option<Task> {
        if let Some(task) = self.queue.pop() {
            self.running.insert(task.id.clone(), task.clone());
            Some(task)
        } else {
            None
        }
    }

    pub fn update(&self, id: &str, state: TaskRunResult) -> Result<(), CmdqError> {
        match state {
            TaskRunResult::Completed => {
                self.running.remove(id);
                {
                    let mut pickledb = self.pickledb.write().unwrap();
                    pickledb
                        .rem(id)
                        .map_err(|e| CmdqError::PickleDbWriteError(e))?;
                }
            }
            TaskRunResult::Failed => {
                let (_id, mut task) = self.running.remove(id).expect("task does not exist");
                task.tries += 1;
                task.last_attempt = Some(SystemTime::now());

                {
                    let mut pickledb = self.pickledb.write().unwrap();
                    pickledb
                        .set(&task.id, &task)
                        .map_err(|e| CmdqError::PickleDbWriteError(e))?;
                }
                self.queue.push(task);
            }
            TaskRunResult::Skipped => {
                let (_id, task) = self.running.remove(id).expect("task does not exist");
                self.queue.push(task);
            }
        }
        Ok(())
    }

    pub fn queued(&self) -> Vec<Task> {
        let pickledb = self.pickledb.read().unwrap();
        pickledb
            .iter()
            .filter_map(|item| item.get_value::<Task>())
            .collect::<Vec<_>>()
    }

    pub fn running(&self) -> Vec<Task> {
        self.running
            .iter()
            .map(|mapref| mapref.value().clone())
            .collect::<Vec<_>>()
    }
}
