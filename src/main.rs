use anyhow::Result;
use vkt::cli::{Commands, parse_args};
use vkt::commands::{Command, config::ConfigCommand, get::GetCommand, list::ListCommand, submit::SubmitCommand};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = parse_args();
    match cli.command {
        Commands::List(args) => {
            let cmd = ListCommand::new(args);
            cmd.execute().await?;
        }
        Commands::Get(args) => {
            let cmd = GetCommand::new(args);
            cmd.execute().await?;
        }
        Commands::Submit(args) => {
            let cmd = SubmitCommand::new(args);
            cmd.execute().await?;
        }
        Commands::Config(args) => {
            let cmd = ConfigCommand::new(args);
            cmd.execute().await?;
        }
    }
    Ok(())
}
