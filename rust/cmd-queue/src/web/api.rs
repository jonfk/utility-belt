use std::sync::Arc;

use actix_web::{post, web, App, HttpServer, Responder};

use crate::{
    CommandFailed, CommandQApp, CommandRequest, CommandResponse, CommandSuccess, ListRequest,
};

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

#[post("/api/commands/list")]
async fn list_tasks(
    app: web::Data<Arc<CommandQApp>>,
    request: web::Json<ListRequest>,
) -> impl Responder {
    let tasks = app.queue.list(
        request
            .state_filters
            .as_ref()
            .unwrap_or(&Vec::new())
            .to_vec(),
    );

    web::Json(tasks)
}
