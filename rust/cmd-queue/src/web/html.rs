use std::sync::Arc;

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use askama::Template;

use crate::{CommandFailed, CommandQApp, CommandRequest, CommandResponse, CommandSuccess, Task};

#[derive(Template)]
#[template(path = "index.html")]
struct Index {
    queued_tasks: Vec<TaskTemplateObject>,
    running_tasks: Vec<TaskTemplateObject>,
}

struct TaskTemplateObject {
    id: String,
    path: String,
    process: String,
    tries: usize,
    last_attempt: String,
}

impl From<Task> for TaskTemplateObject {
    fn from(task: Task) -> Self {
        TaskTemplateObject {
            id: task.id,
            path: task.command.path,
            process: format!("{} {:?}", task.command.program, task.command.args),
            tries: task.tries,
            last_attempt: task
                .last_attempt
                .and_then(|last_attempt| last_attempt.elapsed().ok())
                .map(|last_attempt_elapsed| {
                    format!("{} ago", humantime::format_duration(last_attempt_elapsed))
                })
                .unwrap_or("None".to_string()),
        }
    }
}

// TODO implement a more generic template to html trait https://github.com/djc/askama/blob/main/askama_actix/src/lib.rs#L33
#[get("/")]
async fn index(app: web::Data<Arc<CommandQApp>>) -> impl Responder {
    let queued_tasks = app.queue.queued().into_iter().map(|t| t.into()).collect();
    let running_tasks = app.queue.running().into_iter().map(|t| t.into()).collect();
    let html_body = Index {
        queued_tasks,
        running_tasks,
    }
    .render()
    .unwrap();
    HttpResponse::Ok().content_type("text/html").body(html_body)
}
