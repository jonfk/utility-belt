use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "codex-commit",
    about = "Use Codex to propose and create a git commit",
    version
)]
pub struct Cli {
    #[arg(value_name = "ADDITIONAL_CONTEXT")]
    pub additional_context: Vec<String>,
}

impl Cli {
    pub fn extra_context(&self) -> String {
        self.additional_context.join(" ").trim().to_string()
    }
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
