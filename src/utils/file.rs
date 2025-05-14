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

pub fn get_repository_salt(repo_path: &Path) -> Result<String, KittyError> {
    // In a real implementation, we'd need to properly extract the salt from the config file
    // This is a placeholder implementation
    let encrypted_config = fs::read(repo_path.join("config.enc"))?;

    // In a real implementation, we would decode the config header to extract the salt
    // For now, this is a placeholder approach
    if encrypted_config.len() < 32 {
        return Err(KittyError::Decryption(
            "Invalid repository configuration".to_string(),
        ));
    }

    // Return a placeholder salt - in a real implementation, this would extract from the file
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
