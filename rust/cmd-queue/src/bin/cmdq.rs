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
    Ytdlp {
        url: String,
        #[clap(long, short, help = "Optional prefix to filename downloaded")]
        prefix: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    println!("{:?}", cli);
    let cwd = std::env::current_dir().expect("current dir");

    if !cli.input.is_empty() {
        command_request(
            &cwd.to_string_lossy(),
            &cli.input[0],
            cli.input.clone().into_iter().skip(1).collect(),
        );
    } else if let Some(subcommand) = cli.subcommands {
        match subcommand {
            Subcommands::Ytdlp { url, prefix } => {
                let args = if let Some(prefix) = prefix {
                    vec![
                        "-o".to_string(),
                        format!("{} %(title)s [%(id)s].%(ext)s", prefix),
                        url,
                    ]
                } else {
                    vec![url]
                };
                command_request(&cwd.to_string_lossy(), "yt-dlp", args);
            }
        }
    } else {
        println!("no command queued");
    }
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
