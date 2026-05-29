/// Templates Manager - manages Claude Code settings templates.
///
/// Supports CRUD for templates, default template selection,
/// applying templates to projects, and merging with global settings.
/// The merge logic matches Python: template is base, global overrides
/// env/enabledPlugins/alwaysThinkingEnabled/skipDangerousModePermissionPrompt.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::core::models::SettingsTemplate;

/// Maximum number of templates allowed.
pub const MAX_TEMPLATES: usize = 10;

/// Default template content matching the Python source.
pub fn default_template_content() -> serde_json::Value {
    serde_json::json!({
        "permissions": {
            "allow": [
                "mcp__web-search-prime__web_search_prime",
                "mcp__web-reader__webReader",
                "Bash(npm run:*)",
                "Bash(npm test:*)",
                "Bash(npm install)",
                "Bash(npm update)",
                "Bash(pip install:*)",
                "Bash(python* -m pip install:*)"
            ]
        },
        "env": {
            "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1"
        }
    })
}

/// Errors for template management operations.
#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("模板不存在: {0}")]
    TemplateNotFound(String),

    #[error("模板数量已达上限 (最多 {MAX_TEMPLATES} 个)")]
    TemplateLimitReached,

    #[error("存储文件读取失败: {0}")]
    ReadError(String),

    #[error("存储文件写入失败: {0}")]
    WriteError(String),

    #[error("JSON 解析失败: {0}")]
    ParseError(String),

    #[error("默认模板未设置")]
    NoDefaultTemplate,

    #[error("项目路径无效: {0}")]
    InvalidProjectPath(String),

    #[error("模板应用失败: {0}")]
    ApplyError(String),

    #[error("全局设置读取失败: {0}")]
    GlobalSettingsError(String),

    #[error("旧格式迁移失败: {0}")]
    MigrationError(String),

    #[error("不能删除默认模板")]
    CannotDeleteDefault,
}

/// Storage format: `{id: template_dict}` plus optional `default_template_id`.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TemplateStore {
    /// The ID of the default template.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_template_id: Option<String>,
    /// Maps template ID to template data.
    #[serde(default)]
    pub templates: HashMap<String, SettingsTemplate>,
}

/// Manages Claude Code settings templates with JSON file persistence.
pub struct TemplatesManager {
    storage_path: PathBuf,
}

impl TemplatesManager {
    /// Creates a new `TemplatesManager` with the given storage path.
    pub fn new(storage_path: PathBuf) -> Self {
        let mgr = Self { storage_path };
        mgr.ensure_storage();
        mgr
    }

    /// Creates a `TemplatesManager` with the default storage path
    /// `~/.claude-launcher-v2/templates.json`.
    pub fn default_manager() -> Result<Self, TemplateError> {
        let base = dirs::home_dir()
            .ok_or_else(|| TemplateError::ReadError("无法获取用户主目录".to_string()))?;
        let dir = base.join(".claude-launcher");
        std::fs::create_dir_all(&dir)
            .map_err(|e| TemplateError::WriteError(format!("无法创建目录 {dir:?}: {e}")))?;
        Ok(Self::new(dir.join("templates.json")))
    }

    /// Returns the storage path.
    pub fn storage_path(&self) -> &Path {
        &self.storage_path
    }

    // ── Storage init ─────────────────────────────────────────────

    fn ensure_storage(&self) {
        if self.storage_path.exists() {
            self.validate_storage_structure();
            // 尝试从旧格式迁移（即使 templates.json 已存在）
            self.try_migrate_old_format();
        } else if !self.try_migrate_old_format() {
            self.init_default_storage();
        }
    }

    fn try_migrate_old_format(&self) -> bool {
        let old_path = self
            .storage_path
            .parent()
            .unwrap()
            .join("settings_template.json");
        if !old_path.exists() {
            return false;
        }

        let raw = match std::fs::read_to_string(&old_path) {
            Ok(r) => r,
            Err(_) => return false,
        };
        let old_data: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => return false,
        };

        let template = SettingsTemplate::new("迁移的模板".to_string(), old_data);
        let id = template.id.clone();
        let store = TemplateStore {
            default_template_id: Some(id.clone()),
            templates: {
                let mut map = HashMap::new();
                map.insert(id, template);
                map
            },
        };

        let _ = self.save_store(&store);
        true
    }

    fn init_default_storage(&self) {
        let content = default_template_content();
        let template = SettingsTemplate::new("默认模板".to_string(), content);
        let id = template.id.clone();
        let mut templates = HashMap::new();
        templates.insert(template.id.clone(), template);
        let store = TemplateStore {
            default_template_id: Some(id),
            templates,
        };
        let _ = self.save_store(&store);
    }

    fn validate_storage_structure(&self) {
        match self.load_store_inner() {
            Ok(store) => {
                if store.templates.is_empty() || store.default_template_id.is_none() {
                    self.init_default_storage();
                }
            }
            Err(_) => self.init_default_storage(),
        }
    }

    // ── Persistence ───────────────────────────────────────────────

    fn load_store_inner(&self) -> Result<TemplateStore, TemplateError> {
        if !self.storage_path.exists() {
            return Ok(TemplateStore::default());
        }

        let raw = std::fs::read_to_string(&self.storage_path).map_err(|e| {
            TemplateError::ReadError(format!("无法读取 {:?}: {}", self.storage_path, e))
        })?;

        if raw.trim().is_empty() {
            return Ok(TemplateStore::default());
        }

        let parsed: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| TemplateError::ParseError(format!("JSON 解析失败: {e}")))?;

        // Legacy: { "name": "...", "content": {...} }
        if let Some(obj) = parsed.as_object()
            && obj.contains_key("content")
            && !obj.contains_key("templates")
        {
            return self.migrate_legacy(&parsed);
        }

        let store: TemplateStore = serde_json::from_value(parsed)
            .map_err(|e| TemplateError::ParseError(format!("模板数据解析失败: {e}")))?;
        Ok(store)
    }

    fn load_store(&self) -> Result<TemplateStore, TemplateError> {
        self.load_store_inner()
    }

    fn migrate_legacy(&self, parsed: &serde_json::Value) -> Result<TemplateStore, TemplateError> {
        let content = parsed
            .get("content")
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        let name = parsed
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("默认模板");

        let template = SettingsTemplate::new(name.to_string(), content);
        let id = template.id.clone();
        let store = TemplateStore {
            default_template_id: Some(id.clone()),
            templates: {
                let mut map = HashMap::new();
                map.insert(id, template);
                map
            },
        };
        self.save_store(&store)?;
        Ok(store)
    }

    fn save_store(&self, store: &TemplateStore) -> Result<(), TemplateError> {
        if let Some(parent) = self.storage_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                TemplateError::WriteError(format!("无法创建目录 {parent:?}: {e}"))
            })?;
        }
        let json = serde_json::to_string_pretty(store)
            .map_err(|e| TemplateError::WriteError(format!("JSON 序列化失败: {e}")))?;
        std::fs::write(&self.storage_path, json).map_err(|e| {
            TemplateError::WriteError(format!("无法写入 {:?}: {}", self.storage_path, e))
        })?;
        Ok(())
    }

    // ── CRUD operations ───────────────────────────────────────────

    /// Creates a new template. Returns the created template.
    pub fn create_template(
        &self,
        name: String,
        content: serde_json::Value,
    ) -> Result<SettingsTemplate, TemplateError> {
        let mut store = self.load_store()?;

        if store.templates.len() >= MAX_TEMPLATES {
            return Err(TemplateError::TemplateLimitReached);
        }

        // 检查重名
        let has_duplicate = store.templates.values().any(|t| t.name == name);
        if has_duplicate {
            return Err(TemplateError::WriteError(format!(
                "已存在同名模板「{name}」"
            )));
        }

        let template = SettingsTemplate::new(name, content);
        store
            .templates
            .insert(template.id.clone(), template.clone());
        self.save_store(&store)?;
        Ok(template)
    }

    /// Gets a template by ID.
    pub fn get_template(&self, id: &str) -> Result<SettingsTemplate, TemplateError> {
        let store = self.load_store()?;
        store
            .templates
            .get(id)
            .cloned()
            .ok_or_else(|| TemplateError::TemplateNotFound(id.to_string()))
    }

    /// Lists all templates, sorted by creation time (newest first).
    pub fn list_templates(&self) -> Result<Vec<SettingsTemplate>, TemplateError> {
        let store = self.load_store()?;
        let mut templates: Vec<SettingsTemplate> = store.templates.into_values().collect();
        templates.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(templates)
    }

    /// Updates a template. Pass `None` to leave a field unchanged.
    pub fn update_template(
        &self,
        id: &str,
        name: Option<String>,
        content: Option<serde_json::Value>,
    ) -> Result<SettingsTemplate, TemplateError> {
        let mut store = self.load_store()?;
        let mut template = store
            .templates
            .get(id)
            .cloned()
            .ok_or_else(|| TemplateError::TemplateNotFound(id.to_string()))?;

        if let Some(new_name) = name {
            template.name = new_name;
        }
        if let Some(new_content) = content {
            template.content = new_content;
        }

        store.templates.insert(id.to_string(), template.clone());
        self.save_store(&store)?;
        Ok(template)
    }

    /// Deletes a template by ID. Cannot delete the default template.
    pub fn delete_template(&self, id: &str) -> Result<(), TemplateError> {
        let mut store = self.load_store()?;

        if !store.templates.contains_key(id) {
            return Err(TemplateError::TemplateNotFound(id.to_string()));
        }

        // Prevent deletion of default template
        if store.default_template_id.as_deref() == Some(id) {
            return Err(TemplateError::CannotDeleteDefault);
        }

        store.templates.remove(id);
        self.save_store(&store)?;
        Ok(())
    }

    // ── Default template ──────────────────────────────────────────

    /// Gets the default template. Falls back to first template if
    /// `default_template_id` is not set.
    pub fn get_default_template(&self) -> Result<SettingsTemplate, TemplateError> {
        let store = self.load_store()?;

        let default_id = match &store.default_template_id {
            Some(id) => id.clone(),
            None => store
                .templates
                .keys()
                .next()
                .cloned()
                .ok_or(TemplateError::NoDefaultTemplate)?,
        };

        store
            .templates
            .get(&default_id)
            .cloned()
            .ok_or(TemplateError::TemplateNotFound(default_id))
    }

    /// Sets the default template by ID.
    pub fn set_default_template(&self, id: &str) -> Result<(), TemplateError> {
        let mut store = self.load_store()?;
        if !store.templates.contains_key(id) {
            return Err(TemplateError::TemplateNotFound(id.to_string()));
        }
        store.default_template_id = Some(id.to_string());
        self.save_store(&store)?;
        Ok(())
    }

    /// Gets the default template ID, if set.
    pub fn get_default_template_id(&self) -> Result<Option<String>, TemplateError> {
        let store = self.load_store()?;
        Ok(store.default_template_id)
    }

    // ── Apply & merge ─────────────────────────────────────────────

    /// Applies a template to a project by writing `.claude/settings.local.json`.
    ///
    /// If `template_id` is `None`, uses the default template.
    /// Merges template content with global settings before writing.
    pub fn apply_to_project(
        &self,
        project_path: &str,
        template_id: Option<&str>,
    ) -> Result<(), TemplateError> {
        let project = Path::new(project_path);
        if !project.exists() {
            return Err(TemplateError::InvalidProjectPath(format!(
                "项目路径不存在: {project_path}"
            )));
        }

        let template = match template_id {
            Some(id) => self
                .get_template(id)
                .unwrap_or_else(|_| self.get_default_template().unwrap()),
            None => self.get_default_template()?,
        };

        let merged = merge_with_global(&template.content);

        let claude_dir = project.join(".claude");
        std::fs::create_dir_all(&claude_dir)
            .map_err(|e| TemplateError::ApplyError(format!("无法创建 .claude 目录: {e}")))?;

        let settings_path = claude_dir.join("settings.local.json");
        let json = serde_json::to_string_pretty(&merged)
            .map_err(|e| TemplateError::ApplyError(format!("JSON 序列化失败: {e}")))?;
        std::fs::write(&settings_path, json).map_err(|e| {
            TemplateError::ApplyError(format!("无法写入 {settings_path:?}: {e}"))
        })?;

        Ok(())
    }

    /// Gets the default template content merged with global settings.
    pub fn get_default_template_content(&self) -> Result<serde_json::Value, TemplateError> {
        let template = self.get_default_template()?;
        Ok(merge_with_global(&template.content))
    }
}

// ── Standalone merge functions (matching Python) ──────────────────

/// Loads global settings from `~/.claude/settings.json`.
pub fn load_global_settings() -> serde_json::Value {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return serde_json::Value::Object(serde_json::Map::new()),
    };
    let path = home.join(".claude").join("settings.json");

    if !path.exists() {
        return serde_json::Value::Object(serde_json::Map::new());
    }

    let raw = match std::fs::read_to_string(&path) {
        Ok(r) => r,
        Err(_) => return serde_json::Value::Object(serde_json::Map::new()),
    };

    if raw.trim().is_empty() {
        return serde_json::Value::Object(serde_json::Map::new());
    }

    serde_json::from_str(&raw).unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
}

/// Merges global settings INTO template content.
///
/// Template is the base. Global settings override specific keys:
/// - `env`: global env updates template env (global wins on conflict)
/// - `enabledPlugins`: global replaces template entirely
/// - `alwaysThinkingEnabled`: global overrides template
/// - `skipDangerousModePermissionPrompt`: global overrides template
fn merge_with_global(template_content: &serde_json::Value) -> serde_json::Value {
    let global = load_global_settings();
    let mut result = template_content.clone();

    let global_obj = match global.as_object() {
        Some(obj) => obj,
        None => return result,
    };

    if let Some(result_obj) = result.as_object_mut() {
        // Merge env variables
        if let Some(global_env) = global_obj.get("env").and_then(|v| v.as_object()) {
            let result_env = result_obj
                .entry("env")
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            if let Some(env_map) = result_env.as_object_mut() {
                for (k, v) in global_env {
                    env_map.insert(k.clone(), v.clone());
                }
            }
        }

        // Replace enabledPlugins with global
        if let Some(global_plugins) = global_obj.get("enabledPlugins") {
            result_obj.insert("enabledPlugins".to_string(), global_plugins.clone());
        }

        // Override specific boolean keys
        for key in &["alwaysThinkingEnabled", "skipDangerousModePermissionPrompt"] {
            if let Some(global_val) = global_obj.get(*key) {
                result_obj.insert(key.to_string(), global_val.clone());
            }
        }
    }

    result
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_manager(dir: &std::path::Path) -> TemplatesManager {
        TemplatesManager::new(dir.join("templates.json"))
    }

    fn sample_content() -> serde_json::Value {
        serde_json::json!({
            "permissions": {
                "allow": ["Bash(ls *)"]
            },
            "env": {
                "MY_VAR": "value"
            }
        })
    }

    #[test]
    fn test_init_creates_default() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        // Should auto-create a default template
        let default = mgr.get_default_template().unwrap();
        assert_eq!(default.name, "默认模板");
    }

    #[test]
    fn test_create_and_get_template() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let template = mgr
            .create_template("测试模板".to_string(), sample_content())
            .unwrap();
        assert_eq!(template.name, "测试模板");
        assert!(!template.id.is_empty());

        let fetched = mgr.get_template(&template.id).unwrap();
        assert_eq!(fetched.name, "测试模板");
    }

    #[test]
    fn test_list_templates() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        mgr.create_template("模板1".to_string(), sample_content())
            .unwrap();
        mgr.create_template("模板2".to_string(), serde_json::json!({"key": "val2"}))
            .unwrap();

        let list = mgr.list_templates().unwrap();
        // Default template + 2 created = 3
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn test_update_template() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let template = mgr
            .create_template("旧名称".to_string(), sample_content())
            .unwrap();
        let updated = mgr
            .update_template(
                &template.id,
                Some("新名称".to_string()),
                Some(serde_json::json!({"new_key": "new_val"})),
            )
            .unwrap();
        assert_eq!(updated.name, "新名称");
        assert_eq!(updated.content["new_key"], "new_val");
    }

    #[test]
    fn test_update_name_only() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let template = mgr
            .create_template("旧名称".to_string(), sample_content())
            .unwrap();
        let updated = mgr
            .update_template(&template.id, Some("仅改名称".to_string()), None)
            .unwrap();
        assert_eq!(updated.name, "仅改名称");
        assert_eq!(updated.content["permissions"]["allow"][0], "Bash(ls *)");
    }

    #[test]
    fn test_delete_template() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let template = mgr
            .create_template("待删除".to_string(), sample_content())
            .unwrap();
        mgr.delete_template(&template.id).unwrap();
        assert!(mgr.get_template(&template.id).is_err());
    }

    #[test]
    fn test_cannot_delete_default() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let default = mgr.get_default_template().unwrap();
        let err = mgr.delete_template(&default.id).unwrap_err();
        assert!(matches!(err, TemplateError::CannotDeleteDefault));
    }

    #[test]
    fn test_delete_nonexistent() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());
        assert!(mgr.delete_template("nonexistent-id").is_err());
    }

    #[test]
    fn test_default_template() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let template = mgr
            .create_template("自定义默认".to_string(), sample_content())
            .unwrap();
        mgr.set_default_template(&template.id).unwrap();

        let default = mgr.get_default_template().unwrap();
        assert_eq!(default.id, template.id);
    }

    #[test]
    fn test_set_default_nonexistent() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());
        assert!(mgr.set_default_template("nonexistent-id").is_err());
    }

    #[test]
    fn test_max_templates_limit() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        // Default template already created = 1
        for i in 0..(MAX_TEMPLATES - 1) {
            mgr.create_template(format!("模板{}", i), serde_json::json!({"index": i}))
                .unwrap();
        }

        let result = mgr.create_template(
            "超出限制".to_string(),
            serde_json::json!({"overflow": true}),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_persistence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("templates.json");

        let mgr = TemplatesManager::new(path.clone());
        let template = mgr
            .create_template("持久化".to_string(), sample_content())
            .unwrap();

        let mgr2 = TemplatesManager::new(path);
        let fetched = mgr2.get_template(&template.id).unwrap();
        assert_eq!(fetched.name, "持久化");
    }

    #[test]
    fn test_apply_to_project() {
        let dir = tempdir().unwrap();
        let project_dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let template = mgr
            .create_template("应用测试".to_string(), sample_content())
            .unwrap();
        mgr.apply_to_project(project_dir.path().to_str().unwrap(), Some(&template.id))
            .unwrap();

        let settings_path = project_dir
            .path()
            .join(".claude")
            .join("settings.local.json");
        assert!(settings_path.exists());

        let content = std::fs::read_to_string(&settings_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["env"]["MY_VAR"], "value");
    }

    #[test]
    fn test_apply_to_nonexistent_project() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let template = mgr
            .create_template("测试".to_string(), sample_content())
            .unwrap();
        let result = mgr.apply_to_project("/nonexistent/path", Some(&template.id));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("不存在"));
    }

    #[test]
    fn test_apply_uses_default_if_template_not_found() {
        let dir = tempdir().unwrap();
        let project_dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        // Template ID doesn't exist → falls back to default
        mgr.apply_to_project(project_dir.path().to_str().unwrap(), Some("nonexistent-id"))
            .unwrap();

        let settings_path = project_dir
            .path()
            .join(".claude")
            .join("settings.local.json");
        assert!(settings_path.exists());
    }

    #[test]
    fn test_empty_store_loads() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("templates.json");
        std::fs::write(&path, "").unwrap();
        let mgr = TemplatesManager::new(path);
        // init_default_storage should have been called
        let default = mgr.get_default_template().unwrap();
        assert_eq!(default.name, "默认模板");
    }

    #[test]
    fn test_legacy_format_migration() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("templates.json");

        let legacy = serde_json::json!({
            "name": "旧模板",
            "content": {"permissions": {"allow": ["Bash(test *)"]}}
        });
        std::fs::write(&path, serde_json::to_string(&legacy).unwrap()).unwrap();

        let mgr = TemplatesManager::new(path);
        let list = mgr.list_templates().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "旧模板");

        let default = mgr.get_default_template().unwrap();
        assert_eq!(default.name, "旧模板");
    }

    #[test]
    fn test_update_nonexistent() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());
        assert!(
            mgr.update_template("nonexistent-id", Some("新名称".to_string()), None)
                .is_err()
        );
    }

    #[test]
    fn test_default_template_content_has_expected_keys() {
        let content = default_template_content();
        assert!(content["permissions"]["allow"].is_array());
        assert!(content["env"]["CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS"].is_string());
    }

    #[test]
    fn test_merge_with_global_no_global() {
        let template = serde_json::json!({
            "env": {"TEMPLATE_VAR": "yes"},
            "permissions": {"allow": ["Bash(ls *)"]}
        });
        // Without a real ~/.claude/settings.json, merge returns template as-is
        let merged = merge_with_global(&template);
        assert_eq!(merged["env"]["TEMPLATE_VAR"], "yes");
    }

    #[test]
    fn test_get_default_template_content() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let content = mgr.get_default_template_content().unwrap();
        assert!(content.is_object());
    }
}
