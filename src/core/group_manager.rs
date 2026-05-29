/// Project Group Manager - manages project groups with JSON persistence.
///
/// Stores groups in groups.json and supports CRUD operations.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::core::models::{ProjectGroup, MAX_GROUPS};

/// Errors for group management operations.
#[derive(Debug, Error)]
pub enum GroupError {
    #[error("分组不存在: {0}")]
    GroupNotFound(String),

    #[error("分组数量已达上限 (最多 {MAX_GROUPS} 个)")]
    GroupLimitReached,

    #[error("分组名称已存在: {0}")]
    DuplicateName(String),

    #[error("存储文件读取失败: {0}")]
    ReadError(String),

    #[error("存储文件写入失败: {0}")]
    WriteError(String),

    #[error("JSON 解析失败: {0}")]
    ParseError(String),

    #[error("无法获取用户主目录")]
    HomeDirNotFound,
}

/// Storage format for groups.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GroupStore {
    /// Maps group ID to group data.
    #[serde(default)]
    pub groups: HashMap<String, ProjectGroup>,
}

/// Manages project groups with JSON file persistence.
pub struct GroupManager {
    storage_path: PathBuf,
}

impl GroupManager {
    /// Creates a new GroupManager with the given storage path.
    pub fn new(storage_path: PathBuf) -> Self {
        let mgr = Self { storage_path };
        mgr.ensure_storage();
        mgr
    }

    /// Creates a GroupManager with the default storage path.
    pub fn default_manager() -> Result<Self, GroupError> {
        let base = dirs::home_dir()
            .ok_or(GroupError::HomeDirNotFound)?;
        let dir = base.join(".claude-launcher");
        std::fs::create_dir_all(&dir)
            .map_err(|e| GroupError::WriteError(format!("无法创建目录 {:?}: {}", dir, e)))?;
        Ok(Self::new(dir.join("groups.json")))
    }

    /// Returns the storage path.
    pub fn storage_path(&self) -> &Path {
        &self.storage_path
    }

    // ── Storage init ─────────────────────────────────────────────

    fn ensure_storage(&self) {
        if !self.storage_path.exists() {
            self.init_default_storage();
        }
    }

    fn init_default_storage(&self) {
        let store = GroupStore::default();
        let _ = self.save_store(&store);
    }

    // ── Persistence ───────────────────────────────────────────────

    fn load_store(&self) -> Result<GroupStore, GroupError> {
        if !self.storage_path.exists() {
            return Ok(GroupStore::default());
        }

        let raw = std::fs::read_to_string(&self.storage_path).map_err(|e| {
            GroupError::ReadError(format!("无法读取 {:?}: {}", self.storage_path, e))
        })?;

        if raw.trim().is_empty() {
            return Ok(GroupStore::default());
        }

        let store: GroupStore = serde_json::from_str(&raw)
            .map_err(|e| GroupError::ParseError(format!("JSON 解析失败: {e}")))?;
        Ok(store)
    }

    fn save_store(&self, store: &GroupStore) -> Result<(), GroupError> {
        if let Some(parent) = self.storage_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                GroupError::WriteError(format!("无法创建目录 {:?}: {}", parent, e))
            })?;
        }
        let json = serde_json::to_string_pretty(store)
            .map_err(|e| GroupError::WriteError(format!("JSON 序列化失败: {e}")))?;
        std::fs::write(&self.storage_path, json).map_err(|e| {
            GroupError::WriteError(format!("无法写入 {:?}: {}", self.storage_path, e))
        })?;
        Ok(())
    }

    // ── CRUD operations ───────────────────────────────────────────

    /// Creates a new group. Returns the created group.
    pub fn create_group(&self, name: String) -> Result<ProjectGroup, GroupError> {
        let mut store = self.load_store()?;

        if store.groups.len() >= MAX_GROUPS {
            return Err(GroupError::GroupLimitReached);
        }

        // Check for duplicate name
        let has_duplicate = store.groups.values().any(|g| g.name == name);
        if has_duplicate {
            return Err(GroupError::DuplicateName(name));
        }

        let order = store.groups.len() as i32;
        let group = ProjectGroup::new(name, order);
        store.groups.insert(group.id.clone(), group.clone());
        self.save_store(&store)?;
        Ok(group)
    }

    /// Gets a group by ID.
    pub fn get_group(&self, id: &str) -> Result<ProjectGroup, GroupError> {
        let store = self.load_store()?;
        store.groups.get(id)
            .cloned()
            .ok_or_else(|| GroupError::GroupNotFound(id.to_string()))
    }

    /// Lists all groups, sorted by order.
    pub fn list_groups(&self) -> Result<Vec<ProjectGroup>, GroupError> {
        let store = self.load_store()?;
        let mut groups: Vec<ProjectGroup> = store.groups.into_values().collect();
        groups.sort_by_key(|g| g.order);
        Ok(groups)
    }

    /// Updates a group. Pass `None` to leave a field unchanged.
    pub fn update_group(
        &self,
        id: &str,
        name: Option<String>,
        order: Option<i32>,
    ) -> Result<ProjectGroup, GroupError> {
        let mut store = self.load_store()?;
        let mut group = store.groups.get(id)
            .cloned()
            .ok_or_else(|| GroupError::GroupNotFound(id.to_string()))?;

        if let Some(new_name) = name {
            // Check for duplicate name
            let has_duplicate = store.groups.values().any(|g| g.id != id && g.name == new_name);
            if has_duplicate {
                return Err(GroupError::DuplicateName(new_name));
            }
            group.name = new_name;
        }

        if let Some(new_order) = order {
            group.order = new_order;
        }

        store.groups.insert(id.to_string(), group.clone());
        self.save_store(&store)?;
        Ok(group)
    }

    /// Deletes a group by ID.
    pub fn delete_group(&self, id: &str) -> Result<(), GroupError> {
        let mut store = self.load_store()?;
        if store.groups.remove(id).is_none() {
            return Err(GroupError::GroupNotFound(id.to_string()));
        }
        self.save_store(&store)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_manager(dir: &std::path::Path) -> GroupManager {
        GroupManager::new(dir.join("groups.json"))
    }

    #[test]
    fn test_create_and_get_group() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let group = mgr.create_group("开发".to_string()).unwrap();
        assert_eq!(group.name, "开发");
        assert!(!group.id.is_empty());

        let fetched = mgr.get_group(&group.id).unwrap();
        assert_eq!(fetched.name, "开发");
    }

    #[test]
    fn test_create_duplicate_name() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        mgr.create_group("开发".to_string()).unwrap();
        let result = mgr.create_group("开发".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_list_groups() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        mgr.create_group("开发".to_string()).unwrap();
        mgr.create_group("测试".to_string()).unwrap();

        let groups = mgr.list_groups().unwrap();
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_update_group() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let group = mgr.create_group("开发".to_string()).unwrap();
        let updated = mgr.update_group(&group.id, Some("新名称".to_string()), None).unwrap();
        assert_eq!(updated.name, "新名称");
    }

    #[test]
    fn test_delete_group() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let group = mgr.create_group("开发".to_string()).unwrap();
        mgr.delete_group(&group.id).unwrap();
        assert!(mgr.get_group(&group.id).is_err());
    }

    #[test]
    fn test_max_groups_limit() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        for i in 0..MAX_GROUPS {
            mgr.create_group(format!("分组{i}")).unwrap();
        }

        let result = mgr.create_group("超出限制".to_string());
        assert!(result.is_err());
    }
}
