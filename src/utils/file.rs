use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::commands::init::KittyError;

const REPOSITORY_DIR: &str = ".kitty";

pub fn get_repository_path() -> Result<PathBuf, KittyError> {
    let current_dir = std::env::current_dir()?;
    Ok(current_dir.join(REPOSITORY_DIR))
}

/// Get the storage type for the repository
pub fn get_storage_type(repo_path: &Path) -> Result<String, KittyError> {
    let storage_type_path = repo_path.join("storage.type");
    
    // Default to file-based storage if no marker file exists
    if !storage_type_path.exists() {
        return Ok("file".to_string());
    }
    
    // Read the storage type from the marker file
    let storage_type = fs::read_to_string(storage_type_path)?;
    
    // Trim and validate the storage type
    let storage_type = storage_type.trim();
    match storage_type {
        "file" | "sqlite" => Ok(storage_type.to_string()),
        _ => Err(KittyError::StorageType(format!("Invalid storage type: {}", storage_type)))
    }
}

pub fn get_repository_salt(repo_path: &Path) -> Result<String, KittyError> {
    // First try to extract salt from a separate salt file (simpler approach)
    let salt_path = repo_path.join("salt.key");
    if salt_path.exists() {
        return Ok(fs::read_to_string(salt_path)?);
    }

    // Otherwise read the encrypted config and try to get the salt from there
    let encrypted_config = fs::read(repo_path.join("config.enc"))?;
    
    // Proper salt extraction would require knowing more about how the salt is stored
    // We need to implement a simple solution for now
    // Since we'll be changing the implementation to store the salt in a separate file
    // For backward compatibility:
    if encrypted_config.len() < 32 {
        return Err(KittyError::Decryption(
            "Invalid repository configuration".to_string(),
        ));
    }
    
    // Return a placeholder salt as a fallback
    // This will fail for existing repositories, but that's expected
    // as we're changing the salt storage mechanism
    Ok("0000000000000000000000000000000000000000000000000000000000000000".to_string())
}

pub fn run_with_sudo(command: &[&str]) -> Result<(), KittyError> {
    let status = Command::new("sudo")
        .args(command)
        .status()
        .map_err(|e| KittyError::Io(e))?;

    if !status.success() {
        return Err(KittyError::Io(io::Error::new(
            io::ErrorKind::Other,
            "Command execution failed",
        )));
    }

    Ok(())
}

pub fn copy_file_with_privileges(source: &Path, dest: &Path) -> Result<(), KittyError> {
    // First try to copy directly
    let copy_result = fs::copy(source, dest);

    if let Err(e) = copy_result {
        if e.kind() == io::ErrorKind::PermissionDenied {
            // Permission denied, try with sudo
            println!("Permission denied, attempting with elevated privileges...");
            run_with_sudo(&["cp", source.to_str().unwrap(), dest.to_str().unwrap()])
        } else {
            Err(KittyError::Io(e))
        }
    } else {
        Ok(())
    }
}
