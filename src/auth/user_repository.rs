//! User data model and repository.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// User model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user ID.
    pub id: Uuid,
    /// Username (unique).
    pub username: String,
    /// Argon2 password hash.
    #[serde(skip_serializing)]
    pub password_hash: String,
    /// Whether the user has admin privileges.
    pub is_admin: bool,
    /// Account creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last login timestamp.
    pub last_login: Option<DateTime<Utc>>,
}

impl User {
    /// Create a new user.
    pub fn new(username: String, password_hash: String, is_admin: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            username,
            password_hash,
            is_admin,
            created_at: Utc::now(),
            last_login: None,
        }
    }

    /// Convert to a public representation (without sensitive data).
    pub fn to_public(&self) -> PublicUser {
        PublicUser {
            id: self.id,
            username: self.username.clone(),
            is_admin: self.is_admin,
            created_at: self.created_at,
        }
    }
}

/// Public user representation (safe to expose via API).
#[derive(Debug, Clone, Serialize)]
pub struct PublicUser {
    pub id: Uuid,
    pub username: String,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
}

/// User storage format for JSON file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct UserStore {
    users: Vec<User>,
}

/// Trait for user repository operations.
pub trait UserRepository: Send + Sync {
    /// Find a user by ID.
    fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>>;

    /// Find a user by username.
    fn find_by_username(&self, username: &str) -> AppResult<Option<User>>;

    /// Create a new user.
    fn create(&self, user: User) -> AppResult<User>;

    /// Update a user.
    fn update(&self, user: User) -> AppResult<User>;

    /// Delete a user by ID.
    fn delete(&self, id: Uuid) -> AppResult<bool>;

    /// Get all users.
    fn list_all(&self) -> AppResult<Vec<User>>;

    /// Count total users.
    fn count(&self) -> AppResult<usize>;

    /// Check if a username exists.
    fn username_exists(&self, username: &str) -> AppResult<bool> {
        Ok(self.find_by_username(username)?.is_some())
    }
}

/// JSON file-based user repository.
#[derive(Debug)]
pub struct JsonUserRepository {
    file_path: PathBuf,
    /// In-memory cache for fast reads.
    cache: RwLock<HashMap<Uuid, User>>,
}

impl JsonUserRepository {
    /// Create a new JSON user repository.
    pub fn new(file_path: impl AsRef<Path>) -> AppResult<Self> {
        let file_path = file_path.as_ref().to_path_buf();
        let repo = Self {
            file_path,
            cache: RwLock::new(HashMap::new()),
        };

        // Load existing users or create empty store
        repo.load()?;

        Ok(repo)
    }

    /// Load users from file into cache.
    fn load(&self) -> AppResult<()> {
        if !self.file_path.exists() {
            tracing::info!(path = %self.file_path.display(), "Users file not found, starting fresh");
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.file_path)?;
        let store: UserStore = serde_json::from_str(&content)?;

        let mut cache = self.cache.write();
        cache.clear();
        for user in store.users {
            cache.insert(user.id, user);
        }

        tracing::info!(count = cache.len(), "Loaded users from file");
        Ok(())
    }

    /// Save users from cache to file.
    fn save(&self) -> AppResult<()> {
        let cache = self.cache.read();
        let store = UserStore {
            users: cache.values().cloned().collect(),
        };

        let content = serde_json::to_string_pretty(&store)?;

        // Ensure parent directory exists
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write atomically using temp file
        let temp_path = self.file_path.with_extension("json.tmp");
        std::fs::write(&temp_path, &content)?;
        std::fs::rename(&temp_path, &self.file_path)?;

        tracing::debug!(path = %self.file_path.display(), count = cache.len(), "Saved users to file");
        Ok(())
    }
}

impl UserRepository for JsonUserRepository {
    fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>> {
        let cache = self.cache.read();
        Ok(cache.get(&id).cloned())
    }

    fn find_by_username(&self, username: &str) -> AppResult<Option<User>> {
        let cache = self.cache.read();
        let username_lower = username.to_lowercase();
        Ok(cache
            .values()
            .find(|u| u.username.to_lowercase() == username_lower)
            .cloned())
    }

    fn create(&self, user: User) -> AppResult<User> {
        // Check for duplicate username
        if self.username_exists(&user.username)? {
            return Err(AppError::Conflict(format!(
                "Username '{}' already exists",
                user.username
            )));
        }

        {
            let mut cache = self.cache.write();
            cache.insert(user.id, user.clone());
        }

        self.save()?;
        tracing::info!(user_id = %user.id, username = %user.username, "Created new user");
        Ok(user)
    }

    fn update(&self, user: User) -> AppResult<User> {
        {
            let mut cache = self.cache.write();
            if !cache.contains_key(&user.id) {
                return Err(AppError::NotFound(format!("User {} not found", user.id)));
            }
            cache.insert(user.id, user.clone());
        }

        self.save()?;
        tracing::debug!(user_id = %user.id, "Updated user");
        Ok(user)
    }

    fn delete(&self, id: Uuid) -> AppResult<bool> {
        let removed = {
            let mut cache = self.cache.write();
            cache.remove(&id).is_some()
        };

        if removed {
            self.save()?;
            tracing::info!(user_id = %id, "Deleted user");
        }

        Ok(removed)
    }

    fn list_all(&self) -> AppResult<Vec<User>> {
        let cache = self.cache.read();
        Ok(cache.values().cloned().collect())
    }

    fn count(&self) -> AppResult<usize> {
        let cache = self.cache.read();
        Ok(cache.len())
    }
}

/// Thread-safe wrapper for user repository.
pub type SharedUserRepository = Arc<dyn UserRepository>;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_repo() -> JsonUserRepository {
        let dir = tempdir().unwrap();
        let path = dir.path().join("users.json");
        JsonUserRepository::new(&path).unwrap()
    }

    #[test]
    fn test_create_user() {
        let repo = create_test_repo();
        let user = User::new("testuser".to_string(), "hash".to_string(), false);

        let created = repo.create(user.clone()).unwrap();
        assert_eq!(created.username, "testuser");

        let found = repo.find_by_username("testuser").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, created.id);
    }

    #[test]
    fn test_duplicate_username() {
        let repo = create_test_repo();
        let user1 = User::new("testuser".to_string(), "hash1".to_string(), false);
        let user2 = User::new("testuser".to_string(), "hash2".to_string(), false);

        repo.create(user1).unwrap();
        let result = repo.create(user2);

        assert!(matches!(result, Err(AppError::Conflict(_))));
    }

    #[test]
    fn test_case_insensitive_username() {
        let repo = create_test_repo();
        let user = User::new("TestUser".to_string(), "hash".to_string(), false);
        repo.create(user).unwrap();

        assert!(repo.find_by_username("testuser").unwrap().is_some());
        assert!(repo.find_by_username("TESTUSER").unwrap().is_some());
    }
}
