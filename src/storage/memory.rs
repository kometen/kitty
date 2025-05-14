use crate::commands::init::{KittyError, Repository};
use std::{fs, path::Path};

/// In-memory storage for the kitty repository
/// This is the default storage mechanism that uses the filesystem
pub struct MemoryStorage {
    repo_path: std::path::PathBuf,
}

impl MemoryStorage {
    /// Create a new memory storage
    pub fn new(repo_path: &Path) -> Self {
        Self {
            repo_path: repo_path.to_path_buf(),
        }
    }
    
    /// Save repository information to the encrypted config file
    pub fn save_repository(&self, repository: &Repository, encrypted_data: &[u8]) -> Result<(), KittyError> {
        // Write encrypted configuration to file
        fs::write(self.repo_path.join("config.enc"), encrypted_data)?;
        
        // Store the salt in a separate file for easier access
        fs::write(self.repo_path.join("salt.key"), &repository.salt)?;
        
        Ok(())
    }
    
    /// Get the salt from the repository
    pub fn get_salt(&self) -> Result<String, KittyError> {
        // First try to extract salt from a separate salt file
        let salt_path = self.repo_path.join("salt.key");
        if salt_path.exists() {
            return Ok(fs::read_to_string(salt_path)?);
        }
        
        Err(KittyError::RepositoryNotFound)
    }
    
    /// Get the encrypted repository data
    pub fn get_encrypted_repository(&self) -> Result<Vec<u8>, KittyError> {
        let config_path = self.repo_path.join("config.enc");
        if !config_path.exists() {
            return Err(KittyError::RepositoryNotFound);
        }
        
        Ok(fs::read(config_path)?)
    }
    
    /// Save an encrypted file to the repository
    pub fn save_file(&self, path: &str, encrypted_data: &[u8]) -> Result<(), KittyError> {
        fs::write(self.repo_path.join(path), encrypted_data)?;
        Ok(())
    }
    
    /// Get an encrypted file from the repository
    pub fn get_file(&self, path: &str) -> Result<Vec<u8>, KittyError> {
        let file_path = self.repo_path.join(path);
        if !file_path.exists() {
            return Err(KittyError::FileNotTracked(path.to_string()));
        }
        
        Ok(fs::read(file_path)?)
    }
}