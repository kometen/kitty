use crate::utils::file::{get_repository_path, get_storage_type};
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, Nonce};
use chrono::{DateTime, Utc};
use hex::FromHexError;
use rand::{rngs::OsRng, Rng};
use ring::pbkdf2;
use rpassword::read_password;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
};
use thiserror::Error;

//const REPOSITORY_DIR: &str = ".kitty";
const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const PBKDF2_ITERATIONS: u32 = 100_000;

#[derive(Error, Debug)]
pub enum KittyError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Repository already exists")]
    RepositoryExists,

    #[error("Repository not found")]
    RepositoryNotFound,

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("File not tracked: {0}")]
    FileNotTracked(String),

    #[error("Privilege escalation required for {0}")]
    PrivilegeRequired(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Hex decoding error: {0}")]
    HexDecoding(#[from] FromHexError),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Storage type error: {0}")]
    StorageType(String),
}

#[derive(Serialize, Deserialize)]
pub struct Repository {
    pub created_at: DateTime<Utc>,
    pub salt: String, // Hex encoded
    pub files: Vec<TrackedFile>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TrackedFile {
    pub original_path: String,
    pub repo_path: String, // Relative path in repository
    pub added_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub hash: String, // Hash of file content for quick comparison
}

pub struct Crypto {
    salt: [u8; SALT_LEN],
    key: [u8; KEY_LEN],
}

impl Crypto {
    pub fn new_from_password(password: &str) -> Self {
        let mut salt = [0u8; SALT_LEN];
        let mut rng = OsRng;
        rng.fill(&mut salt);

        let mut key = [0u8; KEY_LEN];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            std::num::NonZeroU32::new(PBKDF2_ITERATIONS).unwrap(),
            &salt,
            password.as_bytes(),
            &mut key,
        );

        Self { salt, key }
    }

    pub fn from_password_and_salt(password: &str, salt: &[u8]) -> Self {
        let mut salt_array = [0u8; SALT_LEN];

        // Handle potential size mismatch between input salt and expected size
        if salt.len() == SALT_LEN {
            salt_array.copy_from_slice(salt);
        } else {
            // If salt doesn't match expected size, use as much as possible and pad with zeros
            let copy_len = std::cmp::min(salt.len(), SALT_LEN);
            salt_array[..copy_len].copy_from_slice(&salt[..copy_len]);
            println!("Warning: Salt size mismatch, using partial salt");
        }

        let mut key = [0u8; KEY_LEN];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            std::num::NonZeroU32::new(PBKDF2_ITERATIONS).unwrap(),
            &salt_array,
            password.as_bytes(),
            &mut key,
        );

        Self {
            salt: salt_array,
            key,
        }
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, KittyError> {
        let mut nonce = [0u8; NONCE_LEN];
        let mut rng = OsRng;
        rng.fill(&mut nonce);

        let cipher = ChaCha20Poly1305::new(Key::from_slice(&self.key));
        let nonce = Nonce::from_slice(&nonce);

        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| KittyError::Encryption(e.to_string()))?;

        // Prepend the nonce to the ciphertext
        let mut result = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        result.extend_from_slice(nonce);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, KittyError> {
        if data.len() < NONCE_LEN {
            return Err(KittyError::Decryption("Invalid ciphertext".to_string()));
        }

        let nonce = &data[..NONCE_LEN];
        let ciphertext = &data[NONCE_LEN..];

        let cipher = ChaCha20Poly1305::new(Key::from_slice(&self.key));
        let nonce = Nonce::from_slice(nonce);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| KittyError::Decryption(e.to_string()))?;

        Ok(plaintext)
    }
}

/// Options for initializing a repository
pub struct InitOptions {
    /// Use SQLite for storage instead of files
    pub use_sqlite: bool,
}

impl Default for InitOptions {
    fn default() -> Self {
        Self {
            use_sqlite: false,
        }
    }
}

pub fn init_repository() -> Result<(), KittyError> {
    init_repository_with_options(&InitOptions::default())
}

pub fn init_repository_with_options(options: &InitOptions) -> Result<(), KittyError> {
    let repo_path = get_repository_path()?;

    if repo_path.exists() {
        return Err(KittyError::RepositoryExists);
    }

    // Create repository directory structure
    fs::create_dir_all(&repo_path)?;
    fs::create_dir_all(repo_path.join("files"))?;

    // Get password from user
    print!("Enter a password for the repository: ");
    io::stdout().flush()?;
    let password = read_password()?;

    // Create crypto instance
    let crypto = Crypto::new_from_password(&password);

    // Create initial repository configuration
    let repository = Repository {
        created_at: Utc::now(),
        salt: hex::encode(crypto.salt),
        files: Vec::new(),
    };

    if options.use_sqlite {
        // Initialize SQLite storage
        println!("Using SQLite storage backend");
        
        use crate::storage::sqlite::SqliteStorage;
        
        // Create and initialize the SQLite database
        let storage = SqliteStorage::new(&repo_path)?;
        
        // Save the repository configuration to SQLite
        storage.save_repository(&repository)?;
        
        // Create a marker file to indicate we're using SQLite
        fs::write(repo_path.join("storage.type"), "sqlite")?;
    } else {
        // Use file-based storage
        println!("Using file-based storage backend");
        
        // Serialize and encrypt the repository configuration
        let config_json = serde_json::to_string(&repository)?;
        let encrypted_config = crypto.encrypt(config_json.as_bytes())?;
        
        // Write encrypted configuration to file
        fs::write(repo_path.join("config.enc"), encrypted_config)?;
        
        // Create a marker file to indicate we're using file-based storage
        fs::write(repo_path.join("storage.type"), "file")?;
    }

    // Store the salt in a separate file for easier access
    fs::write(repo_path.join("salt.key"), hex::encode(&crypto.salt))?;

    println!("Repository initialized successfully.");
    Ok(())
}
