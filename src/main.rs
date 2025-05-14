mod commands;
mod utils;

use clap::{Parser, Subcommand};
use commands::add::add_file;
use commands::diff::diff_file;
use commands::init::{init_repository, KittyError};
use commands::restore::restore_file;

#[derive(Parser)]
#[command(author, version, about = "A Git-like configuration management tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new kitty repository
    Init,

    /// Add a file to track in the repository
    Add {
        /// Path to the file to add
        path: String,
    },

    /// Remove a file from tracking
    Rm {
        /// Path to the file to remove
        path: String,
    },

    /// Show the status of tracked files
    Status,

    /// Show differences between tracked files and their current state
    Diff {
        /// Path to the file to diff
        path: Option<String>,
    },

    /// Restore files from the repository
    Restore {
        /// Path to the file to restore
        path: String,
    },

    /// List all tracked files
    List,
}

fn main() -> Result<(), KittyError> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init => init_repository(),
        Commands::Add { path } => add_file(path),
        Commands::Rm { path } => {
            println!("Removing file: {}", path);
            // TODO: Implement remove functionality
            Ok(())
        }
        Commands::Status => {
            println!("Checking status of tracked files...");
            // TODO: Implement status functionality
            Ok(())
        }
        Commands::Diff { path } => {
            if let Some(p) = path {
                diff_file(p)
            } else {
                println!("Showing differences for all tracked files");
                // TODO: Implement showing diff for all files
                Ok(())
            }
        }
        Commands::Restore { path } => restore_file(path),
        Commands::List => {
            println!("Listing all tracked files...");
            // TODO: Implement list functionality
            Ok(())
        }
    }
}
