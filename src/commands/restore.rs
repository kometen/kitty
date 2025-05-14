use crate::commands::init::{Crypto, KittyError, Repository};
use crate::utils::{get_repository_path, get_repository_salt};
use rpassword::read_password;
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

pub fn restore_file(path: &str) -> Result<(), KittyError> {
    let repo_path = get_repository_path()?;

    if !repo_path.exists() {
        return Err(KittyError::RepositoryNotFound);
    }

    // Get absolute path to the file
    let file_path = Path::new(path)
        .canonicalize()
        .unwrap_or_else(|_| Path::new(path).to_path_buf());

    // Get password from user
    print!("Enter repository password: ");
    io::stdout().flush()?;
    let password = read_password()?;

    // Read and decrypt repository configuration
    let encrypted_config = fs::read(repo_path.join("config.enc"))?;
    let config_salt = hex::decode(get_repository_salt(&repo_path)?)?;
    let crypto = Crypto::from_password_and_salt(&password, &config_salt);
    let decrypted_config = crypto.decrypt(&encrypted_config)?;
    let repository: Repository = serde_json::from_slice(&decrypted_config)?;

    // Find the file in the repository
    let tracked_file = repository
        .files
        .iter()
        .find(|f| Path::new(&f.original_path) == file_path)
        .ok_or_else(|| KittyError::FileNotTracked(path.to_string()))?;

    // Read and decrypt the stored file content
    let encrypted_stored_content = fs::read(repo_path.join(&tracked_file.repo_path))?;
    let decrypted_stored_content = crypto.decrypt(&encrypted_stored_content)?;

    // Check if we need elevated privileges to write to the file
    let metadata = fs::metadata(&file_path).ok();
    let needs_privileges = metadata
        .map(|m| !m.permissions().readonly())
        .unwrap_or(false);

    if needs_privileges {
        // TODO: Implement privilege escalation
        println!("Note: This file may require elevated privileges to modify.");
        // For now, just ask the user to run with sudo
        println!("Consider running the command with sudo.");
    }

    // Write the file content
    // In a real implementation, you would use privilege escalation if needed
    fs::write(&file_path, decrypted_stored_content)?;

    println!("File restored successfully: {}", path);
    Ok(())
}
