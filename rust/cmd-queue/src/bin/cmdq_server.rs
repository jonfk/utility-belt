use std::sync::Arc;

use actix_web::{web, App, HttpServer, Responder};
use clap::Parser;
use cmd_queue::{
    constants::DEFAULT_PORT,
    web::{
        api::{list_queued_tasks, list_running_tasks, queue_command},
        html::index,
    },
    CommandQApp,
};

#[derive(Parser, Debug)]
#[clap(name = "cmdq_server")]
#[clap(author = "Jonathan Fok kan <jonathan@fokkan.ca>")]
#[clap(version = "1.0")]
#[clap(about = "cmdq server", long_about = None)]
struct ServerCli {}

async fn health() -> impl Responder {
    "UP"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cli = ServerCli::parse();
    let cmdq_app = Arc::new(CommandQApp::new().expect("Failed to start server"));

    HttpServer::new(move || {
        App::new()
            .data(cmdq_app.clone())
            .service(queue_command)
            .service(list_queued_tasks)
            .service(list_running_tasks)
            .service(index)
            .service(web::resource("/health").to(health))
    })
    .bind(format!("0.0.0.0:{}", DEFAULT_PORT))?
    .run()
    .await
}
