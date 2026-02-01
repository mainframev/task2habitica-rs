use std::{env, process};

use clap::{Parser, Subcommand};
use task2habitica::{commands, Config, Error};

#[derive(Parser)]
#[command(name = "task2habitica")]
#[command(about = "Sync Taskwarrior tasks with Habitica", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    Add,
    Modify,
    Exit,
    Sync,
    Setup,
}

fn is_sync_running() -> bool {
    env::var("TASK2HABITICA_RUNNING").is_ok()
}

fn set_sync_env() {
    env::set_var("TASK2HABITICA_RUNNING", "1");
}

fn run() -> Result<(), Error> {
    let cli = Cli::parse();

    if matches!(cli.command, Commands::Setup) {
        return commands::handle_setup();
    }

    let config = Config::load(cli.verbose)?;

    match cli.command {
        Commands::Add => {
            if is_sync_running() {
                use std::io::{self, BufRead};
                let stdin = io::stdin();
                let mut lines = stdin.lock().lines();
                if let Some(Ok(line)) = lines.next() {
                    println!("{}", line);
                }
                return Ok(());
            }
            commands::handle_add(&config)?;
        }

        Commands::Modify => {
            if is_sync_running() {
                use std::io::{self, BufRead};
                let stdin = io::stdin();
                let mut lines = stdin.lock().lines();
                let _ = lines.next(); // Skip old task
                if let Some(Ok(line)) = lines.next() {
                    println!("{}", line);
                }
                return Ok(());
            }
            commands::handle_modify(&config)?;
        }

        Commands::Exit => {
            commands::handle_exit(&config)?;
        }

        Commands::Sync => {
            set_sync_env();
            commands::handle_sync(&config)?;
        }

        Commands::Setup => {
            unreachable!()
        }
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}
