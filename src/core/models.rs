use chrono::{DateTime, Utc};
/// Core data models for Claude Code Launcher.
use serde::{Deserialize, Serialize};

/// 自定义日期反序列化函数，兼容旧格式（没有时区后缀）
fn deserialize_datetime_compat<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;

    // 尝试 RFC 3339 格式（带 Z）
    if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
        return Ok(dt.with_timezone(&Utc));
    }

    // 尝试没有时区的格式（假设是 UTC）
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Ok(DateTime::from_naive_utc_and_offset(dt, Utc));
    }

    // 尝试更简单的格式
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(DateTime::from_naive_utc_and_offset(dt, Utc));
    }

    Err(serde::de::Error::custom(format!(
        "无法解析日期格式: {s}"
    )))
}

/// Represents a Claude Code project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    /// 项目所属分组 ID，None 表示默认分组
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(deserialize_with = "deserialize_datetime_compat")]
    pub created_at: DateTime<Utc>,
}

impl Project {
    pub fn new(name: String, path: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            path,
            group_id: None,
            created_at: chrono::Utc::now(),
        }
    }

    pub fn with_group(name: String, path: String, group_id: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            path,
            group_id: Some(group_id),
            created_at: chrono::Utc::now(),
        }
    }
}

/// Represents a project group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectGroup {
    pub id: String,
    pub name: String,
    pub order: i32,
    #[serde(deserialize_with = "deserialize_datetime_compat")]
    pub created_at: DateTime<Utc>,
}

impl ProjectGroup {
    pub fn new(name: String, order: i32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            order,
            created_at: chrono::Utc::now(),
        }
    }
}

/// Maximum number of groups allowed.
pub const MAX_GROUPS: usize = 10;

/// Template for project settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsTemplate {
    pub id: String,
    pub name: String,
    pub content: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl SettingsTemplate {
    pub fn new(name: String, content: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            content,
            created_at: chrono::Utc::now(),
        }
    }
}

/// Backup status for ~/.claude/settings.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStatus {
    pub settings_exists: bool,
    pub settings_disabled: bool,
    pub backup_exists: bool,
}

impl BackupStatus {
    pub fn status_text(&self) -> &str {
        if self.settings_exists {
            "正常"
        } else if self.settings_disabled {
            "已失效"
        } else {
            "无文件"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Project tests ──────────────────────────────────────────────

    #[test]
    fn test_project_new_assigns_id_and_timestamp() {
        let p = Project::new("测试项目".to_string(), "/tmp/test".to_string());
        assert!(!p.id.is_empty());
        assert_eq!(p.name, "测试项目");
        assert_eq!(p.path, "/tmp/test");
        // created_at should be close to now
        let now = chrono::Utc::now();
        let diff = now.signed_duration_since(p.created_at);
        assert!(
            diff.num_seconds() < 5,
            "created_at should be near current time"
        );
    }

    #[test]
    fn test_project_new_generates_unique_ids() {
        let p1 = Project::new("A".to_string(), "/a".to_string());
        let p2 = Project::new("B".to_string(), "/b".to_string());
        assert_ne!(p1.id, p2.id);
    }

    #[test]
    fn test_project_serialize_deserialize_roundtrip() {
        let original = Project::new("序列化测试".to_string(), "/tmp/serialize".to_string());
        let json = serde_json::to_string(&original).expect("serialization should succeed");
        let restored: Project =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(original.id, restored.id);
        assert_eq!(original.name, restored.name);
        assert_eq!(original.path, restored.path);
        assert_eq!(original.created_at, restored.created_at);
    }

    #[test]
    fn test_project_serialize_json_structure() {
        let p = Project::new("结构测试".to_string(), "/tmp/struct".to_string());
        let val: serde_json::Value = serde_json::to_value(&p).expect("should serialize");
        assert!(val["id"].is_string());
        assert_eq!(val["name"], "结构测试");
        assert_eq!(val["path"], "/tmp/struct");
        assert!(val["created_at"].is_string());
    }

    #[test]
    fn test_project_deserialize_missing_field_fails() {
        let json = r#"{"id": "123", "name": "test"}"#;
        let result: Result<Project, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_project_deserialize_old_format_no_id() {
        // Old format might not have an `id` field
        let json =
            r#"{"name": "旧项目", "path": "/tmp/old", "created_at": "2024-01-01T00:00:00Z"}"#;
        let result: Result<Project, _> = serde_json::from_str(json);
        // Should fail because id is missing
        assert!(result.is_err());
    }

    #[test]
    fn test_project_clone_is_equal() {
        let p = Project::new("克隆测试".to_string(), "/tmp/clone".to_string());
        let cloned = p.clone();
        assert_eq!(p.id, cloned.id);
        assert_eq!(p.name, cloned.name);
        assert_eq!(p.path, cloned.path);
        assert_eq!(p.created_at, cloned.created_at);
    }

    // ── SettingsTemplate tests ─────────────────────────────────────

    #[test]
    fn test_template_new_assigns_id_and_timestamp() {
        let t = SettingsTemplate::new("测试模板".to_string(), serde_json::json!({"key": "value"}));
        assert!(!t.id.is_empty());
        assert_eq!(t.name, "测试模板");
        assert_eq!(t.content["key"], "value");
    }

    #[test]
    fn test_template_new_generates_unique_ids() {
        let t1 = SettingsTemplate::new("A".to_string(), serde_json::json!({}));
        let t2 = SettingsTemplate::new("B".to_string(), serde_json::json!({}));
        assert_ne!(t1.id, t2.id);
    }

    #[test]
    fn test_template_serialize_deserialize_roundtrip() {
        let original = SettingsTemplate::new(
            "序列化模板".to_string(),
            serde_json::json!({"permissions": {"allow": ["Bash(ls *)"]}}),
        );
        let json = serde_json::to_string(&original).expect("serialization should succeed");
        let restored: SettingsTemplate =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(original.id, restored.id);
        assert_eq!(original.name, restored.name);
        assert_eq!(original.content, restored.content);
        assert_eq!(original.created_at, restored.created_at);
    }

    #[test]
    fn test_template_content_preserves_complex_json() {
        let content = serde_json::json!({
            "permissions": {
                "allow": ["Bash(npm run:*)", "Bash(pip install:*)"]
            },
            "env": {
                "KEY1": "value1",
                "KEY2": 42,
                "KEY3": true
            },
            "nested": {
                "deep": {
                    "value": [1, 2, 3]
                }
            }
        });
        let t = SettingsTemplate::new("复杂内容".to_string(), content.clone());
        let json = serde_json::to_string(&t).expect("serialize ok");
        let restored: SettingsTemplate = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(restored.content, content);
    }

    #[test]
    fn test_template_clone_is_equal() {
        let t = SettingsTemplate::new("克隆模板".to_string(), serde_json::json!({"x": 1}));
        let cloned = t.clone();
        assert_eq!(t.id, cloned.id);
        assert_eq!(t.name, cloned.name);
        assert_eq!(t.content, cloned.content);
    }

    // ── BackupStatus tests ─────────────────────────────────────────

    #[test]
    fn test_backup_status_normal() {
        let s = BackupStatus {
            settings_exists: true,
            settings_disabled: false,
            backup_exists: true,
        };
        assert_eq!(s.status_text(), "正常");
    }

    #[test]
    fn test_backup_status_disabled() {
        let s = BackupStatus {
            settings_exists: false,
            settings_disabled: true,
            backup_exists: true,
        };
        assert_eq!(s.status_text(), "已失效");
    }

    #[test]
    fn test_backup_status_no_file() {
        let s = BackupStatus {
            settings_exists: false,
            settings_disabled: false,
            backup_exists: false,
        };
        assert_eq!(s.status_text(), "无文件");
    }

    #[test]
    fn test_backup_status_has_backup_but_no_settings() {
        // Has backup but no settings and not disabled
        let s = BackupStatus {
            settings_exists: false,
            settings_disabled: false,
            backup_exists: true,
        };
        assert_eq!(s.status_text(), "无文件");
    }

    #[test]
    fn test_backup_status_serialization_roundtrip() {
        let s = BackupStatus {
            settings_exists: true,
            settings_disabled: false,
            backup_exists: true,
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let restored: BackupStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.settings_exists, s.settings_exists);
        assert_eq!(restored.settings_disabled, s.settings_disabled);
        assert_eq!(restored.backup_exists, s.backup_exists);
    }

    #[test]
    fn test_backup_status_priority_settings_over_disabled() {
        // When settings_exists is true, it takes priority even if disabled is also true
        let s = BackupStatus {
            settings_exists: true,
            settings_disabled: true,
            backup_exists: false,
        };
        assert_eq!(s.status_text(), "正常");
    }
}
