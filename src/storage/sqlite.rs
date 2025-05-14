use crate::commands::init::{KittyError, Repository, TrackedFile};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, types::Type};
use std::path::{Path, PathBuf};
use std::fs;

/// SQLite storage for the kitty repository
pub struct SqliteStorage {
    connection: Connection,
}

impl SqliteStorage {
    /// Create a new SQLite storage
    pub fn new(repo_path: &Path) -> Result<Self, KittyError> {
        let db_path = repo_path.join("kitty.db");
        let connection = Connection::open(db_path)
            .map_err(|e| KittyError::Database(e.to_string()))?;
        
        // Initialize the database if needed
        Self::initialize_db(&connection)?;
        
        Ok(Self { connection })
    }
    
    /// Initialize the database schema
    fn initialize_db(conn: &Connection) -> Result<(), KittyError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS repository (
                id INTEGER PRIMARY KEY,
                created_at TEXT NOT NULL,
                salt TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| KittyError::Database(e.to_string()))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY,
                original_path TEXT NOT NULL,
                repo_path TEXT NOT NULL,
                added_at TEXT NOT NULL,
                last_updated TEXT NOT NULL,
                hash TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| KittyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        Ok(())
    }
    
    /// Save repository information
    pub fn save_repository(&self, repository: &Repository) -> Result<(), KittyError> {
        // First, delete existing repository info
        self.connection
            .execute("DELETE FROM repository", [])
            .map_err(|e| KittyError::Database(e.to_string()))?;
        
        // Insert the new repository info
        self.connection
            .execute(
                "INSERT INTO repository (id, created_at, salt) VALUES (1, ?1, ?2)",
                params![repository.created_at.to_rfc3339(), repository.salt],
            )
            .map_err(|e| KittyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        
        // Now save all the files
        self.connection
            .execute("DELETE FROM files", [])
            .map_err(|e| KittyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        
        for file in &repository.files {
            self.connection
                .execute(
                    "INSERT INTO files (original_path, repo_path, added_at, last_updated, hash) 
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        file.original_path,
                        file.repo_path,
                        file.added_at.to_rfc3339(),
                        file.last_updated.to_rfc3339(),
                        file.hash
                    ],
                )
                .map_err(|e| KittyError::Database(e.to_string()))?;
        }
        
        Ok(())
    }
    
    /// Load repository information
    pub fn load_repository(&self) -> Result<Repository, KittyError> {
        let mut stmt = self.connection
            .prepare("SELECT created_at, salt FROM repository WHERE id = 1")
            .map_err(|e| KittyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            
        let mut rows = stmt.query([])
            .map_err(|e| KittyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            
        // Get repository information
        let row = rows.next()
            .map_err(|e| KittyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?
            .ok_or_else(|| KittyError::RepositoryNotFound)?;
            
        let created_at_str: String = row.get(0)
            .map_err(|e| KittyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| KittyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?
            .with_timezone(&Utc);
            
        let salt: String = row.get(1)
            .map_err(|e| KittyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            
        // Get files
        let mut files = Vec::new();
        let mut stmt = self.connection
            .prepare("SELECT original_path, repo_path, added_at, last_updated, hash FROM files")
            .map_err(|e| KittyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            
        let file_rows = stmt.query_map([], |row| {
            let added_at_str: String = row.get(2)?;
            let last_updated_str: String = row.get(3)?;
            
            let added_at = DateTime::parse_from_rfc3339(&added_at_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(e)))?
                .with_timezone(&Utc);
                
            let last_updated = DateTime::parse_from_rfc3339(&last_updated_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(e)))?
                .with_timezone(&Utc);
                
            Ok(TrackedFile {
                original_path: row.get(0)?,
                repo_path: row.get(1)?,
                added_at,
                last_updated,
                hash: row.get(4)?
            })
        })
        .map_err(|e| KittyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        
        for file_result in file_rows {
            files.push(file_result
                .map_err(|e| KittyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?);
        }
        
        Ok(Repository {
            created_at,
            salt,
            files
        })
    }
    
    /// Get the salt from the repository
    pub fn get_salt(&self) -> Result<String, KittyError> {
        let mut stmt = self.connection
            .prepare("SELECT salt FROM repository WHERE id = 1")
            .map_err(|e| KittyError::Database(e.to_string()))?;
            
        let salt: String = stmt.query_row([], |row| row.get(0))
            .map_err(|e| {
                match e {
                    rusqlite::Error::QueryReturnedNoRows => KittyError::RepositoryNotFound,
                    _ => KittyError::Database(e.to_string())
                }
            })?;
            
        Ok(salt)
    }
    
    /// Save an encrypted file to the repository
    pub fn save_file(&self, path: &str, encrypted_data: &[u8]) -> Result<(), KittyError> {
        // For simplicity, we'll still use the filesystem to store file content
        // But we could store it in the database as well
        let db_path = self.connection.path().unwrap();
        let db_path = Path::new(db_path);
        let repo_path = db_path.parent().unwrap();
        fs::write(repo_path.join(path), encrypted_data)?;
        Ok(())
    }
    
    /// Get an encrypted file from the repository
    pub fn get_file(&self, path: &str) -> Result<Vec<u8>, KittyError> {
        let db_path = self.connection.path().unwrap();
        let db_path = Path::new(db_path);
        let repo_path = db_path.parent().unwrap();
        let file_path = repo_path.join(path);
        if !file_path.exists() {
            return Err(KittyError::FileNotTracked(path.to_string()));
        }
        
        Ok(fs::read(file_path)?)
    }
}

/// A helper function to get the SQLite database path
pub fn get_db_path(repo_path: &Path) -> PathBuf {
    repo_path.join("kitty.db")
}