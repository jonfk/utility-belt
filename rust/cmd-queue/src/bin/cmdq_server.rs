use std::sync::Arc;

use actix_web::{web, App, HttpServer, Responder};
use cmd_queue::{CommandQApp, CommandRequest, CommandResponse, CommandSuccess};

async fn queue_command(
    app: web::Data<Arc<CommandQApp>>,
    command: web::Json<CommandRequest>,
) -> impl Responder {
    println!("queue command {:?}", command);
    app.queue.push_cmd(&command);

    web::Json(CommandResponse::Success(CommandSuccess {}))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // TODO write to a pseudo pid file in /var/run when starting
    // TODO take port number and bind address as cli args
    let cmdq_app = Arc::new(CommandQApp::new());

    HttpServer::new(move || {
        App::new()
            .data(cmdq_app.clone())
            .service(web::scope("/commands").route("/", web::post().to(queue_command)))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
