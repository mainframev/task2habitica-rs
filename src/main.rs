use std::{env, process};

use clap::{Parser, Subcommand};
use task2habitica::{commands, Config, Error};

/// Sync Taskwarrior tasks with Habitica
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
}

/// Check if we're running inside a sync operation
fn is_sync_running() -> bool {
    env::var("TASK2HABITICA_RUNNING").is_ok()
}

/// Set environment variable to indicate sync is running
fn set_sync_env() {
    env::set_var("TASK2HABITICA_RUNNING", "1");
}

fn run() -> Result<(), Error> {
    let cli = Cli::parse();

    // Load configuration
    let config = Config::load(cli.verbose)?;

    // Handle commands
    match cli.command {
        Commands::Add => {
            // Skip if sync is running
            if is_sync_running() {
                // Just pass through the input
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
            // Skip if sync is running
            if is_sync_running() {
                // Just pass through the new task
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
            // Set environment variable to prevent hooks from running during sync
            set_sync_env();
            commands::handle_sync(&config)?;
        }
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        // Print error message
        eprintln!("Error: {}", err);

        // Exit with error code
        process::exit(1);
    }
}
