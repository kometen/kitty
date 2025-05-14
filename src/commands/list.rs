use crate::{
    commands::init::{KittyError, Repository},
    utils::file::{get_repository_path, get_repository_salt},
};
use chrono::Local;
use rpassword::read_password;
use std::{
    fs,
    io::{self, Write},
};

/// Lists all files tracked in the kitty repository
pub fn list_files() -> Result<(), KittyError> {
    let repo_path = get_repository_path()?;

    if !repo_path.exists() {
        return Err(KittyError::RepositoryNotFound);
    }

    // Get password from user
    print!("Enter repository password: ");
    io::stdout().flush()?;
    let password = read_password()?;
    println!();  // Add a newline after password input

    // Read and decrypt repository configuration
    let encrypted_config = fs::read(repo_path.join("config.enc"))?;
    
    // Get salt and create crypto instance
    let salt_str = get_repository_salt(&repo_path)?;
    let config_salt = hex::decode(&salt_str)?;
    let crypto = crate::commands::init::Crypto::from_password_and_salt(&password, &config_salt);
    
    // Decrypt configuration
    let decrypted_config = crypto.decrypt(&encrypted_config)?;
    let repository: Repository = serde_json::from_slice(&decrypted_config)?;

    if repository.files.is_empty() {
        println!("No files are currently tracked in the repository.");
        return Ok(());
    }

    // Display the tracked files in a formatted table
    println!("\n{:<5} {:<50} {:<25}", "ID", "Path", "Last Updated");
    println!("{:<5} {:<50} {:<25}", "---", "----", "------------");

    for (idx, file) in repository.files.iter().enumerate() {
        let path_display = if file.original_path.len() > 50 {
            format!("...{}", &file.original_path[file.original_path.len() - 47..])
        } else {
            file.original_path.clone()
        };

        // Format the last updated date in a human-readable format
        let last_updated = file.last_updated.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S");

        println!("{:<5} {:<50} {:<25}", idx + 1, path_display, last_updated);
    }

    // Display total count
    println!("\nTotal tracked files: {}", repository.files.len());

    Ok(())
}