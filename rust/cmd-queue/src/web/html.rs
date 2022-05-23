use std::sync::Arc;

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use askama::Template;

use crate::{CommandFailed, CommandQApp, CommandRequest, CommandResponse, CommandSuccess};

#[derive(Template)]
#[template(path = "index.html")]
struct Index;

// TODO implement a more generic template to html trait https://github.com/djc/askama/blob/main/askama_actix/src/lib.rs#L33
#[get("/")]
async fn index(app: web::Data<Arc<CommandQApp>>) -> impl Responder {
    let html_body = Index.render().unwrap();
    HttpResponse::Ok().content_type("text/html").body(html_body)
}
