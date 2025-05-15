use crate::{
    commands::init::{Crypto, KittyError, TrackedFile},
    storage::sqlite::SqliteStorage,
    utils::file::{get_repository_path, get_repository_salt, get_storage_type},
};

use blake3;
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

    // Get the storage type
    let storage_type = get_storage_type(&repo_path)?;
    println!("Using storage type: {}", storage_type);

    // Get the salt from the repository
    let salt_str = get_repository_salt(&repo_path)?;
    println!(
        "Retrieved salt (length={}): {}",
        salt_str.len(),
        &salt_str[..10]
    );

    // Decode the hex-encoded salt
    let config_salt = match hex::decode(&salt_str) {
        Ok(salt) => {
            println!("Decoded salt successfully, length: {} bytes", salt.len());
            salt
        }
        Err(e) => {
            println!("Error decoding salt: {}", e);
            return Err(KittyError::HexDecoding(e));
        }
    };

    // Create crypto instance with password and salt
    let crypto = Crypto::from_password_and_salt(&password, &config_salt);

    // Load repository based on storage type
    let mut repository = if storage_type == "sqlite" {
        // Use SQLite storage
        let storage = SqliteStorage::new(&repo_path)?;
        storage.load_repository()?
    } else {
        // Read and decrypt repository configuration
        let encrypted_config = fs::read(repo_path.join("config.enc"))?;

        // Decrypt configuration
        println!("Attempting to decrypt configuration...");
        let decrypted_config = match crypto.decrypt(&encrypted_config) {
            Ok(config) => {
                println!(
                    "Decryption successful! Config length: {} bytes",
                    config.len()
                );
                config
            }
            Err(e) => {
                println!("Decryption failed: {}", e);
                return Err(e);
            }
        };

        // Parse the JSON configuration
        serde_json::from_slice(&decrypted_config)?
    };

    // Check if this file is already tracked
    let file_path_str = file_path.to_string_lossy().to_string();
    let existing_file_index = repository
        .files
        .iter()
        .position(|f| f.original_path == file_path_str);

    // Encrypt file content
    let encrypted_content = crypto.encrypt(&file_content)?;

    let hash = blake3::hash(&file_content).to_hex().to_string();

    let now = Utc::now();

    if let Some(index) = existing_file_index {
        // File is already tracked, update the existing entry
        println!("File is already tracked, updating existing entry.");
        let tracked_file = &mut repository.files[index];

        // Save the repo_path as we'll reuse it
        let repo_file_path = tracked_file.repo_path.clone();

        // Update the tracked file metadata
        tracked_file.last_updated = now;
        tracked_file.hash = hash; // Updated hash

        // For file-based storage, save file immediately
        if storage_type != "sqlite" {
            // Save to filesystem for file-based storage
            fs::write(repo_path.join(&repo_file_path), &encrypted_content)?;
        }
        // For SQLite storage, we'll save the file content after updating the repository metadata
    } else {
        // File is not tracked yet, create a new entry
        // Generate a unique filename for the repository
        let file_id = Uuid::new_v4().to_string();
        let repo_file_path = format!("files/{}", file_id);

        // For file-based storage, save file immediately
        if storage_type != "sqlite" {
            // Save to filesystem for file-based storage
            fs::write(repo_path.join(&repo_file_path), &encrypted_content)?;
        }

        // Add new entry to repository config
        repository.files.push(TrackedFile {
            original_path: file_path_str,
            repo_path: repo_file_path,
            added_at: now,
            last_updated: now,
            // In a real implementation, you would compute a hash here
            hash: hash,
        });
    }

    // Save repository based on storage type
    if storage_type == "sqlite" {
        // Use SQLite storage
        let mut storage = SqliteStorage::new(&repo_path)?;

        // First save the repository metadata
        storage.save_repository(&repository)?;

        // Now save the file content after the metadata is saved
        // This is crucial for SQLite storage to work correctly
        if let Some(index) = existing_file_index {
            // Use existing file's repo_path
            let repo_file_path = &repository.files[index].repo_path;
            storage.save_file(repo_file_path, &encrypted_content)?;
        } else {
            // Use the newly created repo_file_path
            let repo_file_path = &repository.files[0].repo_path;
            storage.save_file(&repo_file_path, &encrypted_content)?;
        }
    } else {
        // Serialize and encrypt updated configuration
        let updated_config_json = serde_json::to_string(&repository)?;
        let encrypted_updated_config = crypto.encrypt(updated_config_json.as_bytes())?;

        // Write updated encrypted configuration
        fs::write(repo_path.join("config.enc"), encrypted_updated_config)?;
    }

    if existing_file_index.is_some() {
        println!("File updated successfully: {}", path);
    } else {
        println!("File added successfully: {}", path);
    }
    Ok(())
}
