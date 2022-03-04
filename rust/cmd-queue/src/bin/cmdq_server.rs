use std::{fs::File, sync::Arc};

use actix_web::{post, web, App, HttpServer, Responder};
use clap::Parser;
use cmd_queue::{
    constants::{self, DEFAULT_PORT},
    CommandQApp, CommandRequest, CommandResponse, CommandSuccess, ListRequest,
};
use daemonize::Daemonize;

#[derive(Parser, Debug)]
#[clap(name = "cmdq_server")]
#[clap(author = "Jonathan Fok kan <jonathan@fokkan.ca>")]
#[clap(version = "1.0")]
#[clap(about = "cmdq server", long_about = None)]
struct ServerCli {
    #[clap(long, short, help = "Daemonize server process")]
    daemon: bool,
}

#[post("/commands")]
async fn queue_command(
    app: web::Data<Arc<CommandQApp>>,
    command: web::Json<CommandRequest>,
) -> impl Responder {
    println!("queue command {:?}", command);
    app.queue.push_cmd(&command);

    web::Json(CommandResponse::Success(CommandSuccess {}))
}

#[post("/commands/list")]
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

async fn health() -> impl Responder {
    "UP"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cli = ServerCli::parse();
    if cli.daemon {
        daemonize();
    }
    let cmdq_app = Arc::new(CommandQApp::new());

    HttpServer::new(move || {
        App::new()
            .data(cmdq_app.clone())
            .service(queue_command)
            .service(list_tasks)
            // .service(
            //     web::scope("/commands")
            //         .route("/", web::post().to(queue_command))
            //         .route("/", web::method().to(list_tasks)),
            // )
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
        .exit_action(|| println!("Server started as daemon"))
        .privileged_action(|| "Executed before drop privileges");

    match daemonize.start() {
        Ok(_) => println!("Success, daemonized"),
        Err(e) => eprintln!("Error starting server as daemon. error={}", e),
    }
}
