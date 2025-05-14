use crate::{
    commands::init::{Crypto, KittyError, Repository, TrackedFile},
    utils::file::{get_repository_path, get_repository_salt},
};
use colored::Colorize;
use rpassword::read_password;
use similar::{ChangeTag, TextDiff};
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

/// Options for the diff command
pub struct DiffOptions {
    /// Path to the file to diff
    pub path: Option<String>,
    
    /// Show files with changes only
    pub only_changed: bool,
    
    /// Show summary of changes
    pub summary: bool,
    
    /// Show a unified diff format with context
    pub context: bool,
    
    /// Number of context lines to show (when context is true)
    pub context_lines: usize,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            path: None,
            only_changed: false,
            summary: false,
            context: false,
            context_lines: 3,
        }
    }
}

/// Holds the result of a diff operation
struct DiffResult {
    path: String,
    has_changes: bool,
    additions: usize,
    deletions: usize,
    diff_text: String,
}

/// Perform diff on a single file
fn diff_single_file(
    repo_path: &Path,
    crypto: &Crypto,
    file: &TrackedFile,
    options: &DiffOptions,
) -> Result<DiffResult, KittyError> {
    // Get the original file path
    let file_path = Path::new(&file.original_path);
    
    // Try to read the current file content
    let current_content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(_) => {
            // File doesn't exist or can't be read
            return Ok(DiffResult {
                path: file.original_path.clone(),
                has_changes: true,
                additions: 0,
                deletions: 0,
                diff_text: format!("File {} no longer exists or cannot be read\n", file.original_path),
            });
        }
    };

    // Read and decrypt the stored file content
    let encrypted_stored_content = fs::read(repo_path.join(&file.repo_path))?;
    let decrypted_stored_content = crypto.decrypt(&encrypted_stored_content)?;
    let stored_content = String::from_utf8_lossy(&decrypted_stored_content).to_string();

    // Calculate diff
    let diff = TextDiff::from_lines(&stored_content, &current_content);
    
    // Count additions and deletions
    let mut additions = 0;
    let mut deletions = 0;
    let mut diff_text = String::new();
    
    // First pass: identify if there are any changes
    let mut has_any_changes = false;
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete | ChangeTag::Insert => {
                has_any_changes = true;
                break;
            },
            _ => {}
        }
    }

    // If no changes, just indicate files are identical
    if !has_any_changes {
        return Ok(DiffResult {
            path: file.original_path.clone(),
            has_changes: false,
            additions: 0,
            deletions: 0,
            diff_text: "Files are identical.\n".to_string(),
        });
    }

    // Second pass: track changes with proper formatting
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => {
                deletions += 1;
                diff_text.push_str(&format!("{}{}", "-".red(), change));
            },
            ChangeTag::Insert => {
                additions += 1;
                diff_text.push_str(&format!("{}{}", "+".green(), change));
            },
            ChangeTag::Equal => {
                // Only include unchanged lines if context mode is enabled
                if options.context {
                    diff_text.push_str(&format!(" {}", change));
                }
            },
        }
    }
    
    let has_changes = additions > 0 || deletions > 0;
    
    Ok(DiffResult {
        path: file.original_path.clone(),
        has_changes,
        additions,
        deletions,
        diff_text,
    })
}

/// List files with differences
pub fn diff_files(options: Option<DiffOptions>) -> Result<(), KittyError> {
    let options = options.unwrap_or_default();
    let show_context = options.context;
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
    let config_salt = hex::decode(get_repository_salt(&repo_path)?)?;
    let crypto = Crypto::from_password_and_salt(&password, &config_salt);
    let decrypted_config = crypto.decrypt(&encrypted_config)?;
    let repository: Repository = serde_json::from_slice(&decrypted_config)?;

    if repository.files.is_empty() {
        println!("No files are currently tracked in the repository.");
        return Ok(());
    }

    // Filter files based on path option
    let files_to_diff: Vec<&TrackedFile> = match &options.path {
        Some(path) => {
            // If path is provided, find the specific file
            let file_path = Path::new(path).canonicalize().unwrap_or_else(|_| Path::new(path).to_path_buf());
            
            let matching_file = repository
                .files
                .iter()
                .find(|f| Path::new(&f.original_path) == file_path || f.original_path.contains(path));
                
            match matching_file {
                Some(file) => vec![file],
                None => {
                    return Err(KittyError::FileNotTracked(path.to_string()));
                }
            }
        },
        None => {
            // If no path is provided, diff all files
            repository.files.iter().collect()
        }
    };

    // Run diff for each file
    let mut diff_results = Vec::new();
    let mut total_additions = 0;
    let mut total_deletions = 0;
    let mut files_with_changes = 0;
    
    for file in files_to_diff {
        let result = diff_single_file(&repo_path, &crypto, file, &options)?;
        
        if result.has_changes {
            files_with_changes += 1;
            total_additions += result.additions;
            total_deletions += result.deletions;
        }
        
        if !options.only_changed || result.has_changes {
            diff_results.push(result);
        }
    }
    
    // Display results
    if options.summary {
        println!("Summary of changes:");
        println!("  Files changed: {}", files_with_changes);
        println!("  Additions: {}", total_additions);
        println!("  Deletions: {}", total_deletions);
        println!();
    }
    
    if diff_results.is_empty() {
        println!("No changes found in tracked files.");
        return Ok(());
    }
    
    for result in diff_results {
        println!("File: {}", result.path.bold());
        if options.summary {
            println!("  +{} -{}", result.additions, result.deletions);
        } else {
            println!("{}", result.diff_text);
        }
        println!(); // Add a blank line between files
    }

    Ok(())
}

// Legacy function for backward compatibility
pub fn diff_file(path: &str) -> Result<(), KittyError> {
    let options = DiffOptions {
        path: Some(path.to_string()),
        only_changed: false,
        summary: false,
        context: false,
        context_lines: 3,
    };
    
    diff_files(Some(options))
}
