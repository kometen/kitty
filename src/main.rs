mod commands;
mod utils;
mod storage;

use clap::{Parser, Subcommand};
use commands::add::add_file;
use commands::diff::diff_files;
use commands::init::{init_repository, KittyError};
use commands::list::list_files;
use commands::restore::{restore_file, restore_files};

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
        
        /// Show files with changes only
        #[arg(long)]
        only_changed: bool,
        
        /// Show summary of changes
        #[arg(long)]
        summary: bool,
        
        /// Show a unified diff format
        #[arg(long)]
        unified: bool,
    },

    /// Restore files from the repository
    Restore {
        /// Path to the file to restore
        path: String,
        
        /// Don't prompt for confirmation
        #[arg(long)]
        force: bool,
        
        /// Show what would be restored without actually restoring
        #[arg(long)]
        dry_run: bool,
        
        /// Backup existing files before restoring
        #[arg(long, default_value = "true")]
        backup: bool,
    },

    /// List all tracked files
    List {
        /// Filter files by path (partial match)
        #[arg(long)]
        path: Option<String>,
        
        /// Filter files by date (format: YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,
        
        /// Group files by path components
        #[arg(long)]
        group: bool,
        
        /// Use SQLite storage (experimental)
        #[arg(long)]
        sqlite: bool,
    },
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
        Commands::Diff { path, only_changed, summary, unified } => {
            let options = commands::diff::DiffOptions {
                path: path.clone(),
                only_changed: *only_changed,
                summary: *summary,
                unified: *unified,
            };
            commands::diff::diff_files(Some(options))
        }
        Commands::Restore { path, force, dry_run, backup } => {
            let options = commands::restore::RestoreOptions {
                path: Some(path.clone()),
                force: *force,
                dry_run: *dry_run,
                backup: *backup,
            };
            commands::restore::restore_files(Some(options))
        },
        Commands::List { path, date, group, sqlite } => {
            let options = commands::list::ListOptions {
                path: path.clone(),
                date: date.clone(),
                group: *group,
            };
            if *sqlite {
                println!("Note: Using experimental SQLite storage");
                // TODO: Implement SQLite storage integration
            }
            list_files(Some(options))
        },
    }
}
