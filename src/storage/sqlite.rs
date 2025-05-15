use crate::commands::init::{KittyError, Repository, TrackedFile};
use chrono::{DateTime, Utc};
use rusqlite::{params, types::Type, Connection};
use std::path::Path;

/// SQLite storage for the kitty repository
pub struct SqliteStorage {
    connection: Connection,
}

impl SqliteStorage {
    /// Create a new SQLite storage
    pub fn new(repo_path: &Path) -> Result<Self, KittyError> {
        let db_path = repo_path.join("kitty.db");
        let connection =
            Connection::open(db_path).map_err(|e| KittyError::Database(e.to_string()))?;

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
                hash TEXT NOT NULL,
                content BLOB
            )",
            [],
        )
        .map_err(|e| {
            KittyError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        Ok(())
    }

    /// Save repository information
    pub fn save_repository(&mut self, repository: &Repository) -> Result<(), KittyError> {
        // Use a transaction to ensure database consistency
        let tx = self
            .connection
            .transaction()
            .map_err(|e| KittyError::Database(e.to_string()))?;

        // Update repository info
        tx.execute("DELETE FROM repository", [])
            .map_err(|e| KittyError::Database(e.to_string()))?;

        tx.execute(
            "INSERT INTO repository (id, created_at, salt) VALUES (1, ?1, ?2)",
            params![repository.created_at.to_rfc3339(), repository.salt],
        )
        .map_err(|e| KittyError::Database(e.to_string()))?;

        // Get existing files with their content and store them in a HashMap
        // Use a block scope to ensure stmt is dropped before tx is committed
        let file_contents = {
            let mut stmt = tx
                .prepare("SELECT repo_path, content FROM files")
                .map_err(|e| KittyError::Database(e.to_string()))?;

            let file_rows = stmt
                .query_map([], |row| {
                    let repo_path: String = row.get(0)?;
                    let content: Option<Vec<u8>> = row.get(1)?;
                    Ok((repo_path, content))
                })
                .map_err(|e| KittyError::Database(e.to_string()))?;

            // Create a map of repo_path -> content for quick lookup
            let mut file_contents = std::collections::HashMap::new();
            for file_result in file_rows {
                let (repo_path, content) =
                    file_result.map_err(|e| KittyError::Database(e.to_string()))?;
                file_contents.insert(repo_path, content);
            }
            file_contents
        }; // stmt is dropped here, releasing the borrow on tx

        // Now update the files table
        tx.execute("DELETE FROM files", [])
            .map_err(|e| KittyError::Database(e.to_string()))?;

        for file in &repository.files {
            // Look up content for this file
            let content = file_contents.get(&file.repo_path);

            if let Some(Some(content_data)) = content {
                // The file has content, preserve it
                tx.execute(
                        "INSERT INTO files (original_path, repo_path, added_at, last_updated, hash, content)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            file.original_path,
                            file.repo_path,
                            file.added_at.to_rfc3339(),
                            file.last_updated.to_rfc3339(),
                            file.hash,
                            content_data
                        ],
                    )
                    .map_err(|e| KittyError::Database(e.to_string()))?;
            } else {
                // No content available, insert with NULL content
                tx.execute(
                        "INSERT INTO files (original_path, repo_path, added_at, last_updated, hash, content)
                         VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
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
        }

        // Commit the transaction
        tx.commit()
            .map_err(|e| KittyError::Database(e.to_string()))?;

        Ok(())
    }

    /// Load repository information
    pub fn load_repository(&self) -> Result<Repository, KittyError> {
        let mut stmt = self
            .connection
            .prepare("SELECT created_at, salt FROM repository WHERE id = 1")
            .map_err(|e| {
                KittyError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        let mut rows = stmt.query([]).map_err(|e| {
            KittyError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        // Get repository information
        let row = rows
            .next()
            .map_err(|e| {
                KittyError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?
            .ok_or_else(|| KittyError::RepositoryNotFound)?;

        let created_at_str: String = row.get(0).map_err(|e| {
            KittyError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| {
                KittyError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?
            .with_timezone(&Utc);

        let salt: String = row.get(1).map_err(|e| {
            KittyError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        // Get files
        let mut files = Vec::new();
        let mut stmt = self
            .connection
            .prepare("SELECT original_path, repo_path, added_at, last_updated, hash FROM files")
            .map_err(|e| {
                KittyError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        let file_rows = stmt
            .query_map([], |row| {
                let added_at_str: String = row.get(2)?;
                let last_updated_str: String = row.get(3)?;

                let added_at = DateTime::parse_from_rfc3339(&added_at_str)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(e))
                    })?
                    .with_timezone(&Utc);

                let last_updated = DateTime::parse_from_rfc3339(&last_updated_str)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(e))
                    })?
                    .with_timezone(&Utc);

                Ok(TrackedFile {
                    original_path: row.get(0)?,
                    repo_path: row.get(1)?,
                    added_at,
                    last_updated,
                    hash: row.get(4)?,
                })
            })
            .map_err(|e| {
                KittyError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

        for file_result in file_rows {
            files.push(file_result.map_err(|e| {
                KittyError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?);
        }

        Ok(Repository {
            created_at,
            salt,
            files,
        })
    }

    /// Get the salt from the repository
    pub fn get_salt(&self) -> Result<String, KittyError> {
        let mut stmt = self
            .connection
            .prepare("SELECT salt FROM repository WHERE id = 1")
            .map_err(|e| KittyError::Database(e.to_string()))?;

        let salt: String = stmt.query_row([], |row| row.get(0)).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => KittyError::RepositoryNotFound,
            _ => KittyError::Database(e.to_string()),
        })?;

        Ok(salt)
    }

    /// Save an encrypted file to the repository
    pub fn save_file(&self, path: &str, encrypted_data: &[u8]) -> Result<(), KittyError> {
        println!("Saving file content to database for path: {}", path);
        println!("Content size: {} bytes", encrypted_data.len());

        // Find the file record in the database
        let result = self.connection.query_row(
            "SELECT id FROM files WHERE repo_path = ?",
            params![path],
            |row| row.get::<_, i64>(0),
        );

        match result {
            Ok(id) => {
                println!("Found existing file record with ID: {}", id);
                // Update the existing file content
                self.connection
                    .execute(
                        "UPDATE files SET content = ? WHERE id = ?",
                        params![encrypted_data, id],
                    )
                    .map_err(|e| {
                        println!("Error updating file content: {}", e);
                        KittyError::Database(e.to_string())
                    })?;

                // Verify the update worked
                let content_size = self
                    .connection
                    .query_row(
                        "SELECT length(content) FROM files WHERE id = ?",
                        params![id],
                        |row| row.get::<_, i64>(0),
                    )
                    .unwrap_or(0);

                println!(
                    "Updated file content size in database: {} bytes",
                    content_size
                );
            }
            Err(e) => {
                println!("File not found in database: {}", e);
                // File not found in database, but this is unlikely since we should
                // always add the metadata first before saving the content
                println!(
                    "Warning: Storing file content for path not yet in database: {}",
                    path
                );
                // We'll still store it, but there may be orphaned content
                self.connection.execute(
                    "INSERT INTO files (repo_path, original_path, added_at, last_updated, hash, content)
                     VALUES (?, 'unknown', datetime('now'), datetime('now'), 'unknown', ?)",
                    params![path, encrypted_data],
                ).map_err(|e| {
                    println!("Error inserting file content: {}", e);
                    KittyError::Database(e.to_string())
                })?;

                println!("Created new file record with content");
            }
        }

        Ok(())
    }

    /// Get an encrypted file from the repository
    pub fn get_file(&self, path: &str) -> Result<Vec<u8>, KittyError> {
        println!("Getting file content from database for path: {}", path);

        // Try to get the file content directly from the database
        let result = self.connection.query_row(
            "SELECT content, id FROM files WHERE repo_path = ?",
            params![path],
            |row| {
                let content: Option<Vec<u8>> = row.get(0)?;
                let id: i64 = row.get(1)?;
                Ok((content, id))
            },
        );

        match result {
            Ok((content, id)) => {
                match content {
                    Some(data) if !data.is_empty() => {
                        println!(
                            "Found file content in database for ID {}: {} bytes",
                            id,
                            data.len()
                        );
                        return Ok(data);
                    }
                    _ => {
                        println!("File found (ID: {}), but content is NULL or empty", id);
                        // Fall back to filesystem for backward compatibility
                        let repo_path = self.connection.path().unwrap();
                        let repo_dir = Path::new(repo_path).parent().unwrap();
                        let file_path = repo_dir.join(path);

                        if file_path.exists() {
                            println!("Found file in filesystem: {}", file_path.display());
                            let data = std::fs::read(&file_path)?;
                            return Ok(data);
                        }

                        return Err(KittyError::Decryption(format!(
                            "File has no content in database and no file at {}",
                            file_path.display()
                        )));
                    }
                }
            }
            Err(e) => {
                println!("Error finding file in database: {}", e);
                // Try with original_path if repo_path didn't work
                let result = self.connection.query_row(
                    "SELECT content, id FROM files WHERE original_path = ?",
                    params![path],
                    |row| {
                        let content: Option<Vec<u8>> = row.get(0)?;
                        let id: i64 = row.get(1)?;
                        Ok((content, id))
                    },
                );

                match result {
                    Ok((content, id)) => match content {
                        Some(data) if !data.is_empty() => {
                            println!(
                                "Found file content by original path for ID {}: {} bytes",
                                id,
                                data.len()
                            );
                            return Ok(data);
                        }
                        _ => {
                            println!("File found by original path (ID: {}), but content is NULL or empty", id);
                            return Err(KittyError::Decryption(format!(
                                "File with original path {} has no content in database",
                                path
                            )));
                        }
                    },
                    Err(_) => {
                        // File not found in database
                        println!(
                            "File not found in database by path or original path: {}",
                            path
                        );
                        return Err(KittyError::FileNotTracked(path.to_string()));
                    }
                }
            }
        }
    }
}
