use crate::{
    commands::init::{Crypto, KittyError, Repository},
    utils::file::{get_repository_path, get_repository_salt},
};
use rpassword::read_password;
use similar::{ChangeTag, TextDiff};
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

pub fn diff_file(path: &str) -> Result<(), KittyError> {
    let repo_path = get_repository_path()?;

    if !repo_path.exists() {
        return Err(KittyError::RepositoryNotFound);
    }

    // Get absolute path to the file
    let file_path = Path::new(path).canonicalize()?;

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

    // Read the current file content
    let current_content = fs::read_to_string(&file_path)?;

    // Read and decrypt the stored file content
    let encrypted_stored_content = fs::read(repo_path.join(&tracked_file.repo_path))?;
    let decrypted_stored_content = crypto.decrypt(&encrypted_stored_content)?;
    let stored_content = String::from_utf8_lossy(&decrypted_stored_content).to_string();

    // Calculate diff
    let diff = TextDiff::from_lines(&stored_content, &current_content);

    // Display diff
    println!("Diff for {}", path);
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        print!("{}{}", sign, change);
    }

    Ok(())
}
