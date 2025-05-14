use crate::{
    commands::init::{Crypto, KittyError, TrackedFile},
    storage::sqlite::SqliteStorage,
    utils::file::{get_repository_path, get_repository_salt, get_storage_type},
};

use colored::Colorize;
use rpassword::read_password;
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

/// Options for the restore command
pub struct RestoreOptions {
    /// Path to the file to restore
    pub path: Option<String>,

    /// Don't prompt for confirmation
    pub force: bool,

    /// Show what would be restored without actually restoring
    pub dry_run: bool,

    /// Backup existing files before restoring
    pub backup: bool,
}

impl Default for RestoreOptions {
    fn default() -> Self {
        Self {
            path: None,
            force: false,
            dry_run: false,
            backup: true,
        }
    }
}

/// Restore files from the repository
pub fn restore_files(options: Option<RestoreOptions>) -> Result<(), KittyError> {
    let options = options.unwrap_or_default();
    let repo_path = get_repository_path()?;

    if !repo_path.exists() {
        return Err(KittyError::RepositoryNotFound);
    }

    // Get password from user
    print!("Enter repository password: ");
    io::stdout().flush()?;
    let password = read_password()?;
    println!(); // Add a newline after password input

    // Get the storage type
    let storage_type = get_storage_type(&repo_path)?;

    // Get salt and create crypto instance
    let config_salt = hex::decode(get_repository_salt(&repo_path)?)?;
    let crypto = Crypto::from_password_and_salt(&password, &config_salt);

    // Load repository based on storage type
    let repository = if storage_type == "sqlite" {
        // Use SQLite storage
        let storage = SqliteStorage::new(&repo_path)?;
        storage.load_repository()?
    } else {
        // Use file-based storage
        let encrypted_config = fs::read(repo_path.join("config.enc"))?;
        let decrypted_config = crypto.decrypt(&encrypted_config)?;
        serde_json::from_slice(&decrypted_config)?
    };

    if repository.files.is_empty() {
        println!("No files are currently tracked in the repository.");
        return Ok(());
    }

    // Filter files based on path option
    // Store the files we'll restore in a Vec
    let files_to_process: Vec<&TrackedFile> = match &options.path {
        Some(path) => {
            // If path is provided, find matching files
            let file_path = Path::new(path)
                .canonicalize()
                .unwrap_or_else(|_| Path::new(path).to_path_buf());

            let matching_files: Vec<&TrackedFile> = repository
                .files
                .iter()
                .filter(|f| {
                    Path::new(&f.original_path) == file_path || f.original_path.contains(path)
                })
                .collect();

            if matching_files.is_empty() {
                return Err(KittyError::FileNotTracked(path.to_string()));
            }

            matching_files
        }
        None => {
            // If no path is provided, prompt user for files to restore
            if !options.force && !options.dry_run {
                println!("No specific path provided. This will restore all tracked files.");
                print!("Continue? [y/N] ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Restore operation canceled.");
                    return Ok(());
                }
            }

            // Restore all files
            repository.files.iter().collect()
        }
    };

    println!("Files to restore: {}", files_to_process.len());

    // Process each file to restore
    let mut restored_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;
    let files_count = files_to_process.len();

    for file in &files_to_process {
        let file_path = Path::new(&file.original_path);
        println!("\nProcessing: {}", file.original_path.bold());

        // Read and decrypt the stored file content
        let encrypted_stored_content = match fs::read(repo_path.join(&file.repo_path)) {
            Ok(content) => content,
            Err(e) => {
                println!(
                    "  {} Could not read repository file: {}",
                    "ERROR:".red().bold(),
                    e
                );
                error_count += 1;
                continue;
            }
        };

        let decrypted_stored_content = match crypto.decrypt(&encrypted_stored_content) {
            Ok(content) => content,
            Err(e) => {
                println!("  {} Failed to decrypt file: {}", "ERROR:".red().bold(), e);
                error_count += 1;
                continue;
            }
        };

        // Check if the file exists
        let file_exists = file_path.exists();

        // If dry run, just report what would happen
        if options.dry_run {
            if file_exists {
                println!("  Would restore file (exists)");
            } else {
                println!("  Would restore file (doesn't exist)");
            }
            skipped_count += 1;
            continue;
        }

        // Create parent directories if they don't exist
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                println!("  Creating parent directory: {}", parent.display());
                if let Err(e) = fs::create_dir_all(parent) {
                    println!(
                        "  {} Failed to create directory: {}",
                        "ERROR:".red().bold(),
                        e
                    );
                    error_count += 1;
                    continue;
                }
            }
        }

        // Create backup if file exists and backup option is enabled
        if file_exists && options.backup {
            let backup_path = format!("{}.bak", file_path.to_string_lossy());
            println!("  Creating backup at {}", backup_path);
            match fs::copy(file_path, &backup_path) {
                Ok(_) => {}
                Err(e) => println!(
                    "  {} Failed to create backup: {}",
                    "WARNING:".yellow().bold(),
                    e
                ),
            }
        }

        // Check if we need elevated privileges to write to the file
        let needs_privileges = if file_exists {
            let metadata = fs::metadata(&file_path).ok();
            metadata
                .map(|m| !m.permissions().readonly())
                .unwrap_or(false)
        } else {
            false
        };

        if needs_privileges {
            // TODO: Implement privilege escalation
            println!(
                "  {} This file may require elevated privileges to modify.",
                "NOTE:".yellow()
            );
            println!("  Consider running the command with sudo.");
        }

        // Write the file content
        match fs::write(file_path, &decrypted_stored_content) {
            Ok(_) => {
                println!("  {} File restored successfully", "SUCCESS:".green().bold());
                restored_count += 1;
            }
            Err(e) => {
                println!("  {} Failed to write file: {}", "ERROR:".red().bold(), e);
                error_count += 1;
            }
        }
    }

    // Print summary
    println!("\nRestore Summary");
    println!("==============");
    println!("Files processed: {}", files_count);
    println!("Restored: {}", restored_count);
    println!("Skipped: {}", skipped_count);
    println!("Errors: {}", error_count);

    Ok(())
}

// Legacy function for backward compatibility
pub fn restore_file(path: &str) -> Result<(), KittyError> {
    let options = RestoreOptions {
        path: Some(path.to_string()),
        force: false,
        dry_run: false,
        backup: true,
    };

    restore_files(Some(options))
}
