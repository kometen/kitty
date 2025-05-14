use crate::{
    commands::init::{Crypto, KittyError, Repository, TrackedFile},
    utils::file::{get_repository_path, get_repository_salt},
};
use chrono::Utc;
use rpassword::read_password;
use std::{
    fs,
    io::{self, Write},
    path::Path,
};
use uuid::Uuid;

pub fn add_file(path: &str) -> Result<(), KittyError> {
    let repo_path = get_repository_path()?;

    if !repo_path.exists() {
        return Err(KittyError::RepositoryNotFound);
    }

    // Get the absolute path to the file
    let file_path = Path::new(path).canonicalize()?;

    // Check if file exists
    if !file_path.exists() {
        return Err(KittyError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found: {}", path),
        )));
    }

    // Check if we have permission to read the file
    let metadata = fs::metadata(&file_path)?;

    // If we can't read the file normally, we might need elevated privileges
    if !metadata.permissions().readonly() {
        // TODO: Implement privilege escalation here
        println!("Note: This file may require elevated privileges to access.");
    }

    // Read the file content
    // In a real implementation, you would use privilege escalation if needed
    let file_content = fs::read(&file_path)?;

    // Get password from user
    print!("Enter repository password: ");
    io::stdout().flush()?;
    let password = read_password()?;

    // Read and decrypt repository configuration
    let encrypted_config = fs::read(repo_path.join("config.enc"))?;

    // Get the salt from the repository
    let salt_str = get_repository_salt(&repo_path)?;
    println!("Retrieved salt (length={}): {}", salt_str.len(), &salt_str[..10]);
    
    // Decode the hex-encoded salt
    let config_salt = match hex::decode(&salt_str) {
        Ok(salt) => {
            println!("Decoded salt successfully, length: {} bytes", salt.len());
            salt
        },
        Err(e) => {
            println!("Error decoding salt: {}", e);
            return Err(KittyError::HexDecoding(e));
        }
    };

    // Create crypto instance with password and salt
    let crypto = Crypto::from_password_and_salt(&password, &config_salt);

    // Decrypt configuration
    println!("Attempting to decrypt configuration...");
    let decrypted_config = match crypto.decrypt(&encrypted_config) {
        Ok(config) => {
            println!("Decryption successful! Config length: {} bytes", config.len());
            config
        },
        Err(e) => {
            println!("Decryption failed: {}", e);
            return Err(e);
        }
    };
    
    // Parse the JSON configuration
    let mut repository: Repository = serde_json::from_slice(&decrypted_config)?;

    // Generate a unique filename for the repository
    let file_id = Uuid::new_v4().to_string();
    let repo_file_path = format!("files/{}", file_id);

    // Encrypt file content
    let encrypted_content = crypto.encrypt(&file_content)?;

    // Save encrypted file to repository
    fs::write(repo_path.join(&repo_file_path), encrypted_content)?;

    // Update repository config
    let now = Utc::now();
    repository.files.push(TrackedFile {
        original_path: file_path.to_string_lossy().to_string(),
        repo_path: repo_file_path,
        added_at: now,
        last_updated: now,
        // In a real implementation, you would compute a hash here
        hash: "placeholder_hash".to_string(),
    });

    // Serialize and encrypt updated configuration
    let updated_config_json = serde_json::to_string(&repository)?;
    let encrypted_updated_config = crypto.encrypt(updated_config_json.as_bytes())?;

    // Write updated encrypted configuration
    fs::write(repo_path.join("config.enc"), encrypted_updated_config)?;

    println!("File added successfully: {}", path);
    Ok(())
}
