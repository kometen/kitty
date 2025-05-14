use crate::{
    commands::init::{Crypto, KittyError, Repository, TrackedFile},
    utils::file::{get_repository_path, get_repository_salt},
};
use chrono::Local;
use rpassword::read_password;
use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    path::Path,
};

/// Options for the list command
pub struct ListOptions {
    /// Filter files by path (partial match)
    pub path: Option<String>,
    
    /// Filter files by date (format: YYYY-MM-DD)
    pub date: Option<String>,
    
    /// Group files by path components
    pub group: bool,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            path: None,
            date: None,
            group: false,
        }
    }
}

/// Filter files based on the provided options
fn filter_files(files: &[TrackedFile], options: &ListOptions) -> Vec<TrackedFile> {
    let mut result = Vec::new();

    for file in files {
        let mut include = true;
    
        // Apply path filter if specified
        if let Some(path_filter) = &options.path {
            if !file.original_path.contains(path_filter) {
                include = false;
            }
        }
    
        // Apply date filter if specified
        if let Some(date_filter) = &options.date {
            let file_date = file.last_updated.format("%Y-%m-%d").to_string();
            if file_date != *date_filter {
                include = false;
            }
        }
    
        if include {
            result.push(file.clone());
        }
    }

    result
}

/// Display files grouped by common directories
fn display_grouped_files(files: &[TrackedFile]) {
    let mut groups: HashMap<String, Vec<TrackedFile>> = HashMap::new();

    // Group files by directory
    for file in files {
        let path = Path::new(&file.original_path);
        let parent = path.parent()
            .and_then(|p| p.to_str())
            .unwrap_or("Other");
    
        groups.entry(parent.to_string())
            .or_insert_with(Vec::new)
            .push(file.clone());
    }

    // Display each group
    for (group, group_files) in groups.iter() {
        println!("\n[{}] - {} file(s)", group, group_files.len());
        println!("{:<5} {:<50} {:<25}", "ID", "Filename", "Last Updated");
        println!("{:<5} {:<50} {:<25}", "---", "--------", "------------");
    
        for (idx, file) in group_files.iter().enumerate() {
            // Get just the filename instead of the full path
            let filename = Path::new(&file.original_path)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or(&file.original_path);
            
            let last_updated = file.last_updated
                .with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S");
            
            println!("{:<5} {:<50} {:<25}", idx + 1, filename, last_updated);
        }
    }
}

/// Lists all files tracked in the kitty repository
pub fn list_files(options: Option<ListOptions>) -> Result<(), KittyError> {
    let options = options.unwrap_or_default();
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
    let crypto = Crypto::from_password_and_salt(&password, &config_salt);
    
    // Decrypt configuration
    let decrypted_config = crypto.decrypt(&encrypted_config)?;
    let repository: Repository = serde_json::from_slice(&decrypted_config)?;

    // Apply filters to the file list
    let filtered_files = filter_files(&repository.files, &options);
    
    if filtered_files.is_empty() {
        if options.path.is_some() || options.date.is_some() {
            println!("No files match the specified filters.");
        } else {
            println!("No files are currently tracked in the repository.");
        }
        return Ok(());
    }

    // If grouping is enabled, display files by group
    if options.group {
        display_grouped_files(&filtered_files);
    } else {
        // Display the tracked files in a formatted table
        println!("\n{:<5} {:<50} {:<25}", "ID", "Path", "Last Updated");
        println!("{:<5} {:<50} {:<25}", "---", "----", "------------");

        for (idx, file) in filtered_files.iter().enumerate() {
            let path_display = if file.original_path.len() > 50 {
                format!("...{}", &file.original_path[file.original_path.len() - 47..])
            } else {
                file.original_path.clone()
            };

            // Format the last updated date in a human-readable format
            let last_updated = file.last_updated.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S");

            println!("{:<5} {:<50} {:<25}", idx + 1, path_display, last_updated);
        }
    }

    // Display total count
    println!("\nTotal tracked files: {}", filtered_files.len());

    Ok(())
}