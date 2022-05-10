use cli_table::{print_stdout, Table};

use crate::Task;

#[derive(Table)]
struct TaskCliTable<'t> {
    id: &'t str,
    destination: &'t str,
    command: String,
    tries: usize,
    last_attempt: String,
}

impl<'t> TaskCliTable<'t> {
    fn from(task: &'t Task) -> Self {
        let last_attempt_since = task
            .last_attempt
            .and_then(|last_attempt| last_attempt.elapsed().ok())
            .map(|last_attempt_elapsed| {
                format!("{} ago", humantime::format_duration(last_attempt_elapsed))
            })
            .unwrap_or("None".to_string());
        TaskCliTable {
            id: &task.id,
            destination: &task.command.path,
            command: format!("{} {}", task.command.program, task.command.args.join(" ")),
            tries: task.tries,
            last_attempt: last_attempt_since,
        }
    }
}

pub fn print_tasks_as_table(tasks: Vec<Task>) -> Result<(), std::io::Error> {
    let table: Vec<_> = tasks.iter().map(|t| TaskCliTable::from(t)).collect();
    print_stdout(table)?;
    Ok(())
}
