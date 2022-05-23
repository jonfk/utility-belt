use std::sync::Arc;

use actix_web::{get, post, web, Responder};

use crate::{CommandFailed, CommandQApp, CommandRequest, CommandResponse, CommandSuccess};

#[post("/api/commands")]
async fn queue_command(
    app: web::Data<Arc<CommandQApp>>,
    command: web::Json<CommandRequest>,
) -> impl Responder {
    println!("queue command {:?}", command);
    // TODO better error handling
    match app.queue.push_cmd(&command) {
        Ok(_) => web::Json(CommandResponse::Success(CommandSuccess {})),
        Err(_) => web::Json(CommandResponse::Failed(CommandFailed {})),
    }
}

#[get("/api/commands/list/queued")]
async fn list_queued_tasks(app: web::Data<Arc<CommandQApp>>) -> impl Responder {
    web::Json(app.queue.queued())
}

#[get("/api/commands/list/running")]
async fn list_running_tasks(app: web::Data<Arc<CommandQApp>>) -> impl Responder {
    web::Json(app.queue.running())
}
