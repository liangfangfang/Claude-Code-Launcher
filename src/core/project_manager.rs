/// Project Manager - manages Claude Code projects with JSON persistence.
///
/// Stores projects as `{id: project_dict}` format in projects.json.
/// Supports legacy format migration from `{"projects": [...]}`.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::core::models::Project;

/// Errors for project management operations.
#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("项目不存在: {0}")]
    ProjectNotFound(String),

    #[error("项目路径已存在: {0}")]
    DuplicatePath(String),

    #[error("项目名称已存在: {0}")]
    DuplicateName(String),

    #[error("存储文件读取失败: {0}")]
    ReadError(String),

    #[error("存储文件写入失败: {0}")]
    WriteError(String),

    #[error("路径解析失败: {0}")]
    PathResolveError(String),

    #[error("项目数量已达上限: {0}")]
    ProjectLimitReached(String),

    #[error("JSON 解析失败: {0}")]
    ParseError(String),
}

/// Storage format for projects: maps project ID -> project data.
pub type ProjectStore = HashMap<String, Project>;

/// Legacy storage format: `{"projects": [project, ...]}`.
#[derive(Debug, Serialize, Deserialize)]
pub struct LegacyProjectStore {
    pub projects: Vec<Project>,
}

/// Maximum number of projects allowed.
const MAX_PROJECTS: usize = 100;

/// Manages Claude Code projects with JSON file persistence.
pub struct ProjectManager {
    storage_path: PathBuf,
}

impl ProjectManager {
    /// Creates a new ProjectManager with the given storage path.
    pub fn new(storage_path: PathBuf) -> Self {
        Self { storage_path }
    }

    /// Creates a ProjectManager with the default storage path
    /// `~/.claude-launcher-v2/projects.json`.
    pub fn default_manager() -> Result<Self, ProjectError> {
        let base = dirs::home_dir()
            .ok_or_else(|| ProjectError::PathResolveError("无法获取用户主目录".to_string()))?;
        let dir = base.join(".claude-launcher-v2");
        std::fs::create_dir_all(&dir)
            .map_err(|e| ProjectError::WriteError(format!("无法创建目录 {:?}: {}", dir, e)))?;
        Ok(Self {
            storage_path: dir.join("projects.json"),
        })
    }

    /// Returns the storage path.
    pub fn storage_path(&self) -> &Path {
        &self.storage_path
    }

    // ── Persistence ───────────────────────────────────────────────

    /// Loads the project store from disk, performing legacy migration if needed.
    fn load_store(&self) -> Result<ProjectStore, ProjectError> {
        if !self.storage_path.exists() {
            return Ok(HashMap::new());
        }

        let raw = std::fs::read_to_string(&self.storage_path).map_err(|e| {
            ProjectError::ReadError(format!("无法读取 {:?}: {}", self.storage_path, e))
        })?;

        if raw.trim().is_empty() {
            return Ok(HashMap::new());
        }

        // Try new format first
        let parsed: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| ProjectError::ParseError(format!("JSON 解析失败: {}", e)))?;

        if parsed.is_object() && parsed.get("projects").is_some() {
            // Legacy format: {"projects": [...]}
            let legacy: LegacyProjectStore = serde_json::from_value(parsed.clone())
                .map_err(|e| ProjectError::ParseError(format!("旧格式迁移失败: {}", e)))?;
            let store: ProjectStore = legacy
                .projects
                .into_iter()
                .map(|p| (p.id.clone(), p))
                .collect();
            // Save in new format
            self.save_store(&store)?;
            Ok(store)
        } else if parsed.is_object() {
            // New format: {id: project_dict}
            let store: ProjectStore = serde_json::from_value(parsed)
                .map_err(|e| ProjectError::ParseError(format!("项目数据解析失败: {}", e)))?;
            Ok(store)
        } else {
            Err(ProjectError::ParseError(
                "无法识别的项目存储格式".to_string(),
            ))
        }
    }

    /// Saves the project store to disk.
    fn save_store(&self, store: &ProjectStore) -> Result<(), ProjectError> {
        if let Some(parent) = self.storage_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ProjectError::WriteError(format!("无法创建目录 {:?}: {}", parent, e))
            })?;
        }
        let json = serde_json::to_string_pretty(store)
            .map_err(|e| ProjectError::WriteError(format!("JSON 序列化失败: {}", e)))?;
        std::fs::write(&self.storage_path, json).map_err(|e| {
            ProjectError::WriteError(format!("无法写入 {:?}: {}", self.storage_path, e))
        })?;
        Ok(())
    }

    // ── Path helpers ──────────────────────────────────────────────

    /// Resolves a path to its canonical form for deduplication.
    /// Falls back to the string representation if canonicalize fails.
    fn resolve_path(path: &str) -> String {
        let p = Path::new(path);
        match p.canonicalize() {
            Ok(canonical) => canonical.to_string_lossy().to_string(),
            Err(_) => {
                // If the path doesn't exist yet, try to resolve parent
                if let Some(parent) = p.parent() {
                    if parent.as_os_str().is_empty() {
                        path.to_string()
                    } else if let Ok(canonical_parent) = parent.canonicalize() {
                        canonical_parent
                            .join(p.file_name().unwrap_or_default())
                            .to_string_lossy()
                            .to_string()
                    } else {
                        path.to_string()
                    }
                } else {
                    path.to_string()
                }
            }
        }
    }

    // ── CRUD operations ───────────────────────────────────────────

    /// Adds a new project. Returns the created project.
    ///
    /// Deduplicates by resolved path. Errors if a project with the
    /// same resolved path already exists.
    pub fn add_project(&self, name: String, path: String) -> Result<Project, ProjectError> {
        let mut store = self.load_store()?;

        if store.len() >= MAX_PROJECTS {
            return Err(ProjectError::ProjectLimitReached(format!(
                "最多支持 {} 个项目",
                MAX_PROJECTS
            )));
        }

        let resolved = Self::resolve_path(&path);

        // Check for duplicate path
        for project in store.values() {
            let existing_resolved = Self::resolve_path(&project.path);
            if existing_resolved == resolved {
                return Err(ProjectError::DuplicatePath(format!(
                    "路径 {} 已被项目 \"{}\" 使用",
                    path, project.name
                )));
            }
        }

        let project = Project::new(name, path);
        store.insert(project.id.clone(), project.clone());
        self.save_store(&store)?;

        Ok(project)
    }

    /// Gets a project by ID.
    pub fn get_project(&self, id: &str) -> Result<Project, ProjectError> {
        let store = self.load_store()?;
        store
            .get(id)
            .cloned()
            .ok_or_else(|| ProjectError::ProjectNotFound(id.to_string()))
    }

    /// Lists all projects, sorted by creation time (newest first).
    pub fn list_projects(&self) -> Result<Vec<Project>, ProjectError> {
        let store = self.load_store()?;
        let mut projects: Vec<Project> = store.into_values().collect();
        projects.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(projects)
    }

    /// Updates a project. Pass `None` to leave a field unchanged.
    pub fn update_project(
        &self,
        id: &str,
        name: Option<String>,
        path: Option<String>,
    ) -> Result<Project, ProjectError> {
        let mut store = self.load_store()?;
        let mut project = store
            .get(id)
            .cloned()
            .ok_or_else(|| ProjectError::ProjectNotFound(id.to_string()))?;

        if let Some(new_name) = name {
            project.name = new_name;
        }

        if let Some(new_path) = path {
            let resolved = Self::resolve_path(&new_path);
            // Check for duplicate path (excluding current project)
            for (other_id, other) in &store {
                if other_id != id {
                    let other_resolved = Self::resolve_path(&other.path);
                    if other_resolved == resolved {
                        return Err(ProjectError::DuplicatePath(format!(
                            "路径 {} 已被项目 \"{}\" 使用",
                            new_path, other.name
                        )));
                    }
                }
            }
            project.path = new_path;
        }

        store.insert(id.to_string(), project.clone());
        self.save_store(&store)?;

        Ok(project)
    }

    /// Deletes a project by ID.
    pub fn delete_project(&self, id: &str) -> Result<(), ProjectError> {
        let mut store = self.load_store()?;
        if store.remove(id).is_none() {
            return Err(ProjectError::ProjectNotFound(id.to_string()));
        }
        self.save_store(&store)?;
        Ok(())
    }

    /// Finds a project by its path (using resolved/canonical comparison).
    pub fn find_by_path(&self, path: &str) -> Result<Option<Project>, ProjectError> {
        let store = self.load_store()?;
        let resolved = Self::resolve_path(path);
        for project in store.values() {
            let project_resolved = Self::resolve_path(&project.path);
            if project_resolved == resolved {
                return Ok(Some(project.clone()));
            }
        }
        Ok(None)
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_manager(dir: &std::path::Path) -> ProjectManager {
        ProjectManager::new(dir.join("projects.json"))
    }

    #[test]
    fn test_add_and_get_project() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let project = mgr
            .add_project("测试项目".to_string(), "/tmp/test".to_string())
            .unwrap();
        assert_eq!(project.name, "测试项目");
        assert_eq!(project.path, "/tmp/test");
        assert!(!project.id.is_empty());

        let fetched = mgr.get_project(&project.id).unwrap();
        assert_eq!(fetched.name, "测试项目");
    }

    #[test]
    fn test_add_duplicate_path() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        mgr.add_project("项目1".to_string(), "/tmp/test".to_string())
            .unwrap();
        let result = mgr.add_project("项目2".to_string(), "/tmp/test".to_string());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("已存在"));
    }

    #[test]
    fn test_list_projects() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        mgr.add_project("项目A".to_string(), "/tmp/a".to_string())
            .unwrap();
        mgr.add_project("项目B".to_string(), "/tmp/b".to_string())
            .unwrap();

        let list = mgr.list_projects().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_update_project() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let project = mgr
            .add_project("旧名称".to_string(), "/tmp/old".to_string())
            .unwrap();
        let updated = mgr
            .update_project(&project.id, Some("新名称".to_string()), None)
            .unwrap();
        assert_eq!(updated.name, "新名称");
        assert_eq!(updated.path, "/tmp/old");

        // Verify persistence
        let fetched = mgr.get_project(&project.id).unwrap();
        assert_eq!(fetched.name, "新名称");
    }

    #[test]
    fn test_update_project_path() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let project = mgr
            .add_project("项目".to_string(), "/tmp/old".to_string())
            .unwrap();
        let updated = mgr
            .update_project(&project.id, None, Some("/tmp/new".to_string()))
            .unwrap();
        assert_eq!(updated.path, "/tmp/new");
    }

    #[test]
    fn test_delete_project() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let project = mgr
            .add_project("待删除".to_string(), "/tmp/del".to_string())
            .unwrap();
        mgr.delete_project(&project.id).unwrap();

        let result = mgr.get_project(&project.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_nonexistent() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let result = mgr.delete_project("nonexistent-id");
        assert!(result.is_err());
    }

    #[test]
    fn test_find_by_path() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let project = mgr
            .add_project("项目".to_string(), "/tmp/findme".to_string())
            .unwrap();
        let found = mgr.find_by_path("/tmp/findme").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, project.id);

        let not_found = mgr.find_by_path("/tmp/nothere").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_empty_store() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let list = mgr.list_projects().unwrap();
        assert!(list.is_empty());

        let result = mgr.get_project("anything");
        assert!(result.is_err());
    }

    #[test]
    fn test_persistence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("projects.json");

        // Create and add a project
        let mgr = ProjectManager::new(path.clone());
        let project = mgr
            .add_project("持久化测试".to_string(), "/tmp/persist".to_string())
            .unwrap();

        // Create a new manager instance pointing to the same file
        let mgr2 = ProjectManager::new(path);
        let fetched = mgr2.get_project(&project.id).unwrap();
        assert_eq!(fetched.name, "持久化测试");
    }

    #[test]
    fn test_legacy_format_migration() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("projects.json");

        // Write legacy format
        let legacy = LegacyProjectStore {
            projects: vec![Project::new(
                "旧项目".to_string(),
                "/tmp/legacy".to_string(),
            )],
        };
        let json = serde_json::to_string(&legacy).unwrap();
        std::fs::write(&path, json).unwrap();

        // Load with new manager should auto-migrate
        let mgr = ProjectManager::new(path.clone());
        let list = mgr.list_projects().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "旧项目");

        // Verify file is now in new format
        let raw = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert!(parsed.get("projects").is_none()); // No longer legacy format
        assert!(parsed.as_object().unwrap().contains_key(&list[0].id));
    }

    #[test]
    fn test_empty_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("projects.json");
        std::fs::write(&path, "").unwrap();

        let mgr = ProjectManager::new(path);
        let list = mgr.list_projects().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_update_nonexistent() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let result = mgr.update_project("nonexistent", Some("新名称".to_string()), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_path_duplicate() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let _p1 = mgr
            .add_project("项目1".to_string(), "/tmp/a".to_string())
            .unwrap();
        let p2 = mgr
            .add_project("项目2".to_string(), "/tmp/b".to_string())
            .unwrap();

        // Try to update p2's path to p1's path
        let result = mgr.update_project(&p2.id, None, Some("/tmp/a".to_string()));
        assert!(result.is_err());
    }
}
