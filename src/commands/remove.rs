use crate::{
    commands::init::{Crypto, KittyError, Repository},
    utils::file::{get_repository_path, get_repository_salt},
};
use colored::Colorize;
use rpassword::read_password;
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

/// Options for the remove command
pub struct RemoveOptions {
    /// Path to the file to remove
    pub path: String,
    
    /// Don't prompt for confirmation
    pub force: bool,
    
    /// Keep the file content in the repository, just stop tracking it
    pub keep_content: bool,
}

impl Default for RemoveOptions {
    fn default() -> Self {
        Self {
            path: String::new(),
            force: false,
            keep_content: false,
        }
    }
}

/// Remove a file from tracking in the repository
pub fn remove_file(options: &RemoveOptions) -> Result<(), KittyError> {
    let repo_path = get_repository_path()?;

    if !repo_path.exists() {
        return Err(KittyError::RepositoryNotFound);
    }

    // Resolve the file path
    let file_path = Path::new(&options.path).canonicalize()
        .unwrap_or_else(|_| Path::new(&options.path).to_path_buf());
    let file_path_str = file_path.to_string_lossy().to_string();

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
    let crypto = Crypto::from_password_and_salt(&password, &config_salt);
    
    // Decrypt configuration
    let decrypted_config = crypto.decrypt(&encrypted_config)?;
    let mut repository: Repository = serde_json::from_slice(&decrypted_config)?;

    // Find the file in the repository
    let file_index = repository.files.iter().position(|f| 
        f.original_path == file_path_str || 
        Path::new(&f.original_path) == file_path
    );

    // If file not found, check if it's a partial path match
    let file_index = match file_index {
        Some(index) => Some(index),
        None => repository.files.iter().position(|f| f.original_path.contains(&options.path))
    };

    if let Some(index) = file_index {
        // Get file information before removing it
        let original_path = repository.files[index].original_path.clone();
        let repo_file_path = repository.files[index].repo_path.clone();
            
        // Get confirmation from user if not forced
        if !options.force {
            println!("About to remove file from tracking: {}", original_path.bold());
            print!("Continue? [y/N] ");
            io::stdout().flush()?;
                
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
                
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Remove operation canceled.");
                return Ok(());
            }
        }
            
        // Remove the file from the repository list
        repository.files.remove(index);
            
        // Delete the file content from the repository if keep_content is false
        if !options.keep_content {
            let file_repo_path = repo_path.join(&repo_file_path);
            if file_repo_path.exists() {
                fs::remove_file(file_repo_path)?;
            }
        }
            
        // Serialize and encrypt updated configuration
        let updated_config_json = serde_json::to_string(&repository)?;
        let encrypted_updated_config = crypto.encrypt(updated_config_json.as_bytes())?;
            
        // Write updated encrypted configuration
        fs::write(repo_path.join("config.enc"), encrypted_updated_config)?;
            
        println!("{} File removed from tracking: {}", "SUCCESS:".green().bold(), original_path);
            
        // Show a reminder that the actual file wasn't deleted
        println!("Note: The original file at {} was not modified.", original_path);
        
        Ok(())
    } else {
        Err(KittyError::FileNotTracked(options.path.clone()))
    }
}