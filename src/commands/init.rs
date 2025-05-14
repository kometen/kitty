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
    path::Path,
};
use thiserror::Error;

use crate::utils::{get_repository_path, get_repository_salt};

const REPOSITORY_DIR: &str = ".kitty";
const SALT_LEN: usize = 16;
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
}

#[derive(Serialize, Deserialize)]
pub struct Repository {
    pub created_at: DateTime<Utc>,
    pub salt: String, // Hex encoded
    pub files: Vec<TrackedFile>,
}

#[derive(Serialize, Deserialize)]
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
        salt_array.copy_from_slice(salt);

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

pub fn init_repository() -> Result<(), KittyError> {
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

    // Serialize and encrypt the repository configuration
    let config_json = serde_json::to_string(&repository)?;
    let encrypted_config = crypto.encrypt(config_json.as_bytes())?;

    // Write encrypted configuration to file
    fs::write(repo_path.join("config.enc"), encrypted_config)?;

    println!("Repository initialized successfully.");
    Ok(())
}

// This is duplicated in add.rs, should be removed from here
fn _unused_add_file(path: &str) -> Result<(), KittyError> {
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

    // Read and decrypt repository configuration
    let encrypted_config = fs::read(repo_path.join("config.enc"))?;

    // Extract salt from encrypted config (first SALT_LEN bytes in our format)
    let config_salt = hex::decode(get_repository_salt(&repo_path)?)?;

    // Create crypto instance with password and salt
    let crypto = Crypto::from_password_and_salt(&password, &config_salt);

    // Decrypt configuration
    let decrypted_config = crypto.decrypt(&encrypted_config)?;
    let mut repository: Repository = serde_json::from_slice(&decrypted_config)?;

    // Generate a unique filename for the repository
    let file_id = format!("{}", uuid::Uuid::new_v4());
    let repo_file_path = format!("files/{}", file_id);

    // Encrypt file content
    let encrypted_content = crypto.encrypt(&file_content)?;

    // Save encrypted file to repository
    fs::write(repo_path.join(&repo_file_path), encrypted_content)?;

    // Update repository config
    let now = Utc::now();
    repository.files.push(TrackedFile {
        original_path: file_path.to_string_lossy().to_string(),
        repo_path: repo_file_path,
        added_at: now,
        last_updated: now,
        // In a real implementation, you would compute a hash here
        hash: "placeholder_hash".to_string(),
    });

    // Serialize and encrypt updated configuration
    let updated_config_json = serde_json::to_string(&repository)?;
    let encrypted_updated_config = crypto.encrypt(updated_config_json.as_bytes())?;

    // Write updated encrypted configuration
    fs::write(repo_path.join("config.enc"), encrypted_updated_config)?;

    println!("File added successfully: {}", path);
    Ok(())
}
