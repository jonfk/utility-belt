use clap::{IntoApp, Parser, Subcommand};
use clap_complete;
use cmd_queue::{constants, error::CmdqClientError, CommandRequest, CommandResponse};
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
    /// Shutdown server daemon
    Shutdown {
        #[clap(long, short, help = "Force shutdown of cmdq server")]
        force: bool,
    },
    GenerateCompletion {
        #[clap(arg_enum)]
        shell: clap_complete::Shell,
    },
}

fn main() -> Result<(), CmdqClientError> {
    let cli = Cli::parse();
    println!("{:?}", cli);
    let cwd = std::env::current_dir().expect("current dir");

    if !cli.input.is_empty() && cli.subcommands.is_some() {
        println!("Sorry, but I don't know what to do both INPUT and subcommand were encountered. Going to sleep instead.");
        Ok(())
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
                command_request(&cwd.to_string_lossy(), "yt-dlp", args)
            }
            Subcommands::Shutdown { force } => shutdown_server(),
            Subcommands::GenerateCompletion { shell } => {
                print_completions(shell, &mut Cli::command_for_update());
                Ok(())
            }
        }
    } else if !cli.input.is_empty() {
        command_request(
            &cwd.to_string_lossy(),
            &cli.input[0],
            cli.input.clone().into_iter().skip(1).collect(),
        )
    } else {
        println!("no command queued");
        Ok(())
    }
}

fn print_completions<G: clap_complete::Generator>(gen: G, cmd: &mut clap::Command) {
    clap_complete::generate(gen, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
}

fn command_request(cwd: &str, program: &str, args: Vec<String>) -> Result<(), CmdqClientError> {
    start_server_if_needed().expect("failed to start server");

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(server_host("commands/"))
        .json(&CommandRequest {
            path: cwd.to_string(),
            program: program.to_string(),
            args: args,
        })
        .send()
        .map_err(|e| CmdqClientError::HttpClientError(e))?;
    println!("{:?}", response);
    let cmd_response = response
        .json::<CommandResponse>()
        .map_err(|e| CmdqClientError::ResponseDeserializationError(e))?;
    Ok(())
}

fn start_server_if_needed() -> std::io::Result<()> {
    let resp = reqwest::blocking::get(server_host("health"));
    if resp.is_err() {
        std::process::Command::new("cmdq_server").spawn()?;
        // TODO better handling of waiting for server to startup
        // Continue to poll health endpoint with max attempts and backoff
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    Ok(())
}

fn shutdown_server() -> Result<(), CmdqClientError> {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;
    use std::fs;
    let pid_str = fs::read_to_string(constants::SERVER_DAEMON_PIDFILE).map_err(|e| {
        CmdqClientError::ReadServerPidFile(constants::SERVER_DAEMON_PIDFILE.to_string(), e)
    })?;
    let pid = pid_str
        .parse::<i32>()
        .map_err(|e| CmdqClientError::ParseServerPid(e))?;
    signal::kill(Pid::from_raw(pid), Signal::SIGINT)
        .map_err(|e| CmdqClientError::KillServer(pid, e))?;
    Ok(())
}

fn server_host(path: &str) -> String {
    format!("http://localhost:{}/{}", constants::DEFAULT_PORT, path)
}
