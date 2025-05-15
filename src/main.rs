mod commands;
mod storage;
mod utils;

use clap::{Parser, Subcommand};
use commands::{
    add::add_file,
    init::{init_repository_with_options, InitOptions, KittyError},
    list::list_files,
    remove::remove_file,
};

#[derive(Parser)]
#[command(author, version, about = "A Git-like configuration management tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new kitty repository
    Init {
        /// Use SQLite for storage instead of files
        #[arg(long)]
        sqlite: bool,
    },

    /// Add a file to track in the repository
    Add {
        /// Path to the file to add
        path: String,
    },

    /// Remove a file from tracking
    Rm {
        /// Path to the file to remove
        path: String,

        /// Don't prompt for confirmation
        #[arg(long)]
        force: bool,

        /// Keep the file content in the repository, just stop tracking it
        #[arg(long)]
        keep_content: bool,
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

        /// Show a unified diff format with context
        #[arg(long)]
        context: bool,

        /// Number of context lines to show
        #[arg(long, default_value = "3")]
        context_lines: usize,
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
    
    /// Migrate file content to SQLite database (for SQLite storage mode)
    MigrateSqlite {
        /// Run migration without prompt
        #[arg(long)]
        force: bool,
    },
}

fn main() -> Result<(), KittyError> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init { sqlite } => {
            let options = InitOptions {
                use_sqlite: *sqlite,
            };
            init_repository_with_options(&options)
        }
        Commands::Add { path } => add_file(path),
        Commands::Rm {
            path,
            force,
            keep_content,
        } => {
            let options = commands::remove::RemoveOptions {
                path: path.clone(),
                force: *force,
                keep_content: *keep_content,
            };
            remove_file(&options)
        }
        Commands::Status => {
            println!("Checking status of tracked files...");
            // TODO: Implement status functionality
            Ok(())
        }
        Commands::Diff {
            path,
            only_changed,
            summary,
            context,
            context_lines,
        } => {
            let options = commands::diff::DiffOptions {
                path: path.clone(),
                only_changed: *only_changed,
                summary: *summary,
                context: *context,
                context_lines: *context_lines,
            };
            commands::diff::diff_files(Some(options))
        }
        Commands::Restore {
            path,
            force,
            dry_run,
            backup,
        } => {
            let options = commands::restore::RestoreOptions {
                path: Some(path.clone()),
                force: *force,
                dry_run: *dry_run,
                backup: *backup,
            };
            commands::restore::restore_files(Some(options))
        }
        Commands::List {
            path,
            date,
            group,
            sqlite,
        } => {
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
        }
        Commands::MigrateSqlite { force } => {
            use std::process::Command;
            
            let repo_path = utils::file::get_repository_path()?;
            if !repo_path.exists() {
                return Err(KittyError::RepositoryNotFound);
            }
            
            let storage_type = utils::file::get_storage_type(&repo_path)?;
            if storage_type != "sqlite" {
                println!("Error: This repository is not using SQLite storage.");
                println!("Only SQLite repositories need migration.");
                return Ok(());
            }
            
            if !*force {
                use std::io::{self, Write};
                
                print!("This will migrate file content from the filesystem to the SQLite database. Continue? [y/N] ");
                io::stdout().flush()?;
                
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                
                if !["y", "yes"].contains(&input.trim().to_lowercase().as_str()) {
                    println!("Migration aborted.");
                    return Ok(());
                }
            }
            
            println!("Running migration script...");
            
            // Find the script path relative to the current executable
            let current_exe = std::env::current_exe()?;
            let script_dir = current_exe.parent().unwrap_or(std::path::Path::new("."));
            let script_path = script_dir.join("migrate_sqlite.sh");
            
            let status = if script_path.exists() {
                Command::new(&script_path)
                    .status()
            } else {
                // Fallback to searching in the current directory
                Command::new("./migrate_sqlite.sh")
                    .status()
            };
            
            match status {
                Ok(exit_status) => {
                    if exit_status.success() {
                        println!("Migration completed successfully.");
                    } else {
                        println!("Migration failed with status: {}", exit_status);
                    }
                },
                Err(e) => {
                    println!("Failed to run migration script: {}", e);
                    println!("Please run the migrate_sqlite.sh script manually.");
                }
            }
            
            Ok(())
        }
    }
}
