use clap::{Parser, Subcommand};
use cmd_queue::{CommandRequest, CommandResponse};
use reqwest;

#[derive(Parser, Debug)]
#[clap(name = "Command Queue")]
#[clap(author = "Jonathan Fok kan <jonathan@fokkan.ca>")]
#[clap(version = "1.0")]
#[clap(about = "A program to queue commands", long_about = None)]
struct Cli {
    #[clap(help = "command to queue")]
    pub input: Vec<String>,

    #[clap(subcommand)]
    pub subcommands: Option<Subcommands>,
}

#[derive(Subcommand, Debug)]
enum Subcommands {
    /// Download with yt-dlp
    Ytdlp { url: String },
}

fn main() {
    let cli = Cli::parse();
    println!("{:?}", cli);
    let cwd = std::env::current_dir().expect("current dir");
    command_request(
        &cwd.to_string_lossy(),
        &cli.input[0],
        cli.input.clone().into_iter().skip(1).collect(),
    );
}

fn command_request(cwd: &str, program: &str, args: Vec<String>) {
    let client = reqwest::blocking::Client::new();
    let response = client
        .post("http://localhost:8080/commands/")
        .json(&CommandRequest {
            path: cwd.to_string(),
            program: program.to_string(),
            args: args,
        })
        .send()
        .expect("client response error");
    println!("{:?}", response);
    let json_response = response
        .json::<CommandResponse>()
        .expect("deserialize response");
}
