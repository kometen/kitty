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
    pub fn save_repository(&self, repository: &Repository) -> Result<(), KittyError> {
        use crate::commands::init::Crypto;
    
        // Get the salt from the repository
        let salt = repository.salt.clone();
    
        // Create crypto instance with an empty password (just for serialization)
        // In a real implementation, we'd use the user's password
        let salt_bytes = hex::decode(&salt).map_err(|e| KittyError::HexDecoding(e))?;
        let crypto = Crypto::from_password_and_salt("placeholder", &salt_bytes);
    
        // Serialize and encrypt the repository
        let repo_json = serde_json::to_string(repository).map_err(|e| KittyError::Serialization(e))?;
        let encrypted_data = crypto.encrypt(repo_json.as_bytes())?;
    
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
    
    /// Load the repository data
    pub fn load_repository(&self) -> Result<Repository, KittyError> {
        use crate::commands::init::Crypto;
    
        // Get the salt
        let salt = self.get_salt()?;
    
        // Read the encrypted data
        let config_path = self.repo_path.join("config.enc");
        if !config_path.exists() {
            return Err(KittyError::RepositoryNotFound);
        }
    
        let encrypted_data = fs::read(config_path)?;
    
        // Decrypt the data using a placeholder password
        // In a real implementation, we'd use the user's password
        let salt_bytes = hex::decode(&salt).map_err(|e| KittyError::HexDecoding(e))?;
        let crypto = Crypto::from_password_and_salt("placeholder", &salt_bytes);
    
        let decrypted_data = crypto.decrypt(&encrypted_data)?;
    
        // Parse the repository
        let repository: Repository = serde_json::from_slice(&decrypted_data)
            .map_err(|e| KittyError::Serialization(e))?;
    
        Ok(repository)
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