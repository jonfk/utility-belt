use std::{fs::File, sync::Arc};

use actix_web::{web, App, HttpServer, Responder};
use cmd_queue::{
    constants::{self, DEFAULT_PORT},
    CommandQApp, CommandRequest, CommandResponse, CommandSuccess,
};
use daemonize::Daemonize;

async fn queue_command(
    app: web::Data<Arc<CommandQApp>>,
    command: web::Json<CommandRequest>,
) -> impl Responder {
    println!("queue command {:?}", command);
    app.queue.push_cmd(&command);

    web::Json(CommandResponse::Success(CommandSuccess {}))
}

async fn health() -> impl Responder {
    "UP"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    daemonize();
    let cmdq_app = Arc::new(CommandQApp::new());

    HttpServer::new(move || {
        App::new()
            .data(cmdq_app.clone())
            .service(web::scope("/commands").route("/", web::post().to(queue_command)))
            .service(web::resource("/health").to(health))
    })
    .bind(format!("127.0.0.1:{}", DEFAULT_PORT))?
    .run()
    .await
}

fn daemonize() {
    std::fs::create_dir_all(constants::SERVER_DAEMON_DIR).unwrap();
    let stdout = File::create(constants::SERVER_DAEMON_OUTFILE).unwrap();
    let stderr = File::create(constants::SERVER_DAEMON_ERRFILE).unwrap();

    let daemonize = Daemonize::new()
        .pid_file(constants::SERVER_DAEMON_PIDFILE) // Every method except `new` and `start`
        .working_directory(constants::SERVER_DAEMON_DIR) // for default behaviour.
        .stdout(stdout) // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr) // Redirect stderr to `/tmp/daemon.err`.
        .exit_action(|| println!("Executed before master process exits"))
        .privileged_action(|| "Executed before drop privileges");

    match daemonize.start() {
        Ok(_) => println!("Success, daemonized"),
        Err(e) => eprintln!("Error, {}", e),
    }
}
