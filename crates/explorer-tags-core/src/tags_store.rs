use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_TAGS: &[&str] = &["🔥 Срочно", "⭐ Важно", "📦 Архив"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsConfig {
    pub tags: Vec<String>,
}

impl Default for TagsConfig {
    fn default() -> Self {
        Self {
            tags: DEFAULT_TAGS.iter().map(|&s| s.to_string()).collect(),
        }
    }
}

pub struct TagsStore {
    config_path: PathBuf,
    pub config: TagsConfig,
}

impl TagsStore {
    /// Получить стандартный путь к директории конфигурации (%APPDATA%\ExplorerTags)
    pub fn app_data_dir() -> PathBuf {
        if let Ok(app_data) = std::env::var("APPDATA") {
            PathBuf::from(app_data).join("ExplorerTags")
        } else {
            PathBuf::from(".").join("ExplorerTags")
        }
    }

    /// Загружает хранилище тегов с поддержкой автомиграции из database.db
    pub fn load() -> Self {
        let app_dir = Self::app_data_dir();
        fs::create_dir_all(&app_dir).ok();
        let config_path = app_dir.join("tags.json");

        if config_path.is_file() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<TagsConfig>(&content) {
                    return Self {
                        config_path,
                        config,
                    };
                }
            }
        }

        // Если tags.json еще нет, проверяем возможность миграции из старой SQLite базы
        let db_path = app_dir.join("database.db");
        let migrated_tags = if db_path.is_file() {
            Self::try_migrate_from_sqlite(&db_path)
        } else {
            None
        };

        let config = TagsConfig {
            tags: migrated_tags.unwrap_or_else(|| TagsConfig::default().tags),
        };

        let store = Self {
            config_path,
            config,
        };
        store.save().ok();
        store
    }

    pub fn load_from_path(config_path: PathBuf) -> Self {
        if config_path.is_file() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<TagsConfig>(&content) {
                    return Self {
                        config_path,
                        config,
                    };
                }
            }
        }

        Self {
            config_path,
            config: TagsConfig::default(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(&self.config).map_err(|e| e.to_string())?;
        fs::write(&self.config_path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn add_tag(&mut self, tag: &str) -> bool {
        let trimmed = tag.trim();
        if trimmed.is_empty() || self.config.tags.iter().any(|t| t == trimmed) {
            return false;
        }
        self.config.tags.push(trimmed.to_string());
        self.save().ok();
        true
    }

    pub fn remove_tag(&mut self, tag: &str) -> bool {
        let initial_len = self.config.tags.len();
        self.config.tags.retain(|t| t != tag);
        if self.config.tags.len() != initial_len {
            self.save().ok();
            true
        } else {
            false
        }
    }

    pub fn tags(&self) -> &[String] {
        &self.config.tags
    }

    fn try_migrate_from_sqlite(db_path: &Path) -> Option<Vec<String>> {
        let conn = rusqlite::Connection::open_with_flags(
            db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
        .ok()?;

        let mut stmt = conn
            .prepare("SELECT name FROM custom_tags ORDER BY id ASC")
            .ok()?;
        let tag_iter = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .ok()?;

        let mut tags = Vec::new();
        for tag in tag_iter.flatten() {
            let trimmed = tag.trim().to_string();
            if !trimmed.is_empty() && !tags.contains(&trimmed) {
                tags.push(trimmed);
            }
        }

        if tags.is_empty() {
            None
        } else {
            Some(tags)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_tags_store_crud() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("tags.json");

        let mut store = TagsStore::load_from_path(config_path.clone());
        assert_eq!(store.tags(), DEFAULT_TAGS);

        // Add
        assert!(store.add_tag("🚀 В работе"));
        assert!(!store.add_tag("🚀 В работе")); // duplicate rejected
        assert_eq!(store.tags().last().map(|s| s.as_str()), Some("🚀 В работе"));

        // Save & reload
        let reloaded = TagsStore::load_from_path(config_path);
        assert_eq!(reloaded.tags(), store.tags());

        // Remove
        assert!(store.remove_tag("🚀 В работе"));
        assert!(!store.tags().contains(&"🚀 В работе".to_string()));
    }

    #[test]
    fn test_migrate_from_sqlite() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute(
                "CREATE TABLE custom_tags (id INTEGER PRIMARY KEY, name TEXT UNIQUE);",
                [],
            )
            .unwrap();
            conn.execute("INSERT INTO custom_tags (name) VALUES ('Тег 1'), ('Тег 2');", [])
                .unwrap();
        }

        let migrated = TagsStore::try_migrate_from_sqlite(&db_path).unwrap();
        assert_eq!(migrated, vec!["Тег 1", "Тег 2"]);
    }
}
