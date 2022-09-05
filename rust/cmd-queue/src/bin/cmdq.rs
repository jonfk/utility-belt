use clap::{IntoApp, Parser, Subcommand};
use clap_complete;
use cmd_queue::{
    cli_util, client::Client, constants, error::CmdqClientError, CommandRequest, TaskState,
};
use reqwest;

#[derive(Parser, Debug)]
#[clap(name = "cmdq")]
#[clap(author = "Jonathan Fok kan <jonathan@fokkan.ca>")]
#[clap(version = "1.0")]
#[clap(about = "A program to queue commands", long_about = None)]
struct Cli {
    #[clap(help = "server url", env = "CMDQ_SERVER_URL")]
    pub server_url: String,
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
    List {
        #[clap(long, short, help = "Filter by running tasks")]
        running: bool,
    },
    GenerateCompletion {
        #[clap(arg_enum)]
        shell: clap_complete::Shell,
    },
}

fn main() -> Result<(), CmdqClientError> {
    let cli = Cli::parse();
    // TODO print as debug
    //println!("{:?}", cli);
    let cwd = std::env::current_dir().expect("current dir");

    let cli_app = CliApp::new(cli.server_url);

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
                cli_app.command_request(&cwd.to_string_lossy(), "yt-dlp", args)
            }
            Subcommands::List { running } => cli_app.list_tasks(running),
            Subcommands::GenerateCompletion { shell } => {
                print_completions(shell, &mut Cli::command_for_update());
                Ok(())
            }
        }
    } else if !cli.input.is_empty() {
        cli_app.command_request(
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

pub struct CliApp {
    client: Client,
}

impl CliApp {
    fn new(server_url: String) -> Self {
        CliApp {
            client: Client::new(&server_url).expect("failed creating client"),
        }
    }

    fn command_request(
        &self,
        dir: &str,
        program: &str,
        args: Vec<String>,
    ) -> Result<(), CmdqClientError> {
        let _cmd_resp = self.client.queue_command(CommandRequest {
            path: dir.to_string(),
            program: program.to_string(),
            args: args,
        })?;
        Ok(())
    }

    fn list_tasks(&self, running: bool) -> Result<(), CmdqClientError> {
        let state_filter = if running {
            TaskState::Running
        } else {
            TaskState::Queued
        };
        let tasks = self.client.list_tasks(state_filter)?;
        cli_util::print_tasks_as_table(tasks).expect("failed print tasks");
        Ok(())
    }
}
