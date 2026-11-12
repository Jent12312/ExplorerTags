use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaggedFolderRecord {
    pub path: PathBuf,
    pub tag: Option<String>,
    pub icon_path: Option<String>,
    pub color_hex: Option<String>,
    pub volume_serial: Option<u64>,
    pub file_id: Option<String>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegistryData {
    pub folders: Vec<TaggedFolderRecord>,
}

pub struct FolderRegistry {
    storage_path: PathBuf,
    data: RegistryData,
}

impl FolderRegistry {
    pub fn default_storage_path() -> PathBuf {
        let app_dir = if let Ok(app_data) = std::env::var("APPDATA") {
            PathBuf::from(app_data).join("ExplorerTags")
        } else {
            PathBuf::from(".").join("ExplorerTags")
        };
        app_dir.join("registry.json")
    }

    pub fn load() -> Self {
        Self::load_from_path(Self::default_storage_path())
    }

    pub fn load_from_path(storage_path: PathBuf) -> Self {
        if storage_path.is_file() {
            if let Ok(content) = fs::read_to_string(&storage_path) {
                if let Ok(data) = serde_json::from_str::<RegistryData>(&content) {
                    return Self { storage_path, data };
                }
            }
        }

        Self {
            storage_path,
            data: RegistryData::default(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(&self.data).map_err(|e| e.to_string())?;
        fs::write(&self.storage_path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn all(&self) -> &[TaggedFolderRecord] {
        &self.data.folders
    }

    pub fn upsert(&mut self, record: TaggedFolderRecord) {
        if let Some(existing) = self.data.folders.iter_mut().find(|r| r.path == record.path) {
            *existing = record;
        } else {
            self.data.folders.push(record);
        }
    }

    pub fn remove(&mut self, path: &Path) -> bool {
        let prev_len = self.data.folders.len();
        self.data.folders.retain(|r| r.path != path);
        self.data.folders.len() != prev_len
    }

    pub fn find_by_path(&self, path: &Path) -> Option<&TaggedFolderRecord> {
        self.data.folders.iter().find(|r| r.path == path)
    }

    pub fn find_by_file_id(&self, volume: u64, file_id: &str) -> Option<&TaggedFolderRecord> {
        self.data.folders.iter().find(|r| {
            r.volume_serial == Some(volume)
                && r.file_id.as_deref().map(|id| id.eq_ignore_ascii_case(file_id)).unwrap_or(false)
        })
    }

    pub fn update_path_by_file_id(&mut self, volume: u64, file_id: &str, new_path: PathBuf) -> bool {
        if let Some(record) = self.data.folders.iter_mut().find(|r| {
            r.volume_serial == Some(volume)
                && r.file_id.as_deref().map(|id| id.eq_ignore_ascii_case(file_id)).unwrap_or(false)
        }) {
            record.path = new_path;
            record.updated_at = current_timestamp();
            true
        } else {
            false
        }
    }
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
