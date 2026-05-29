/// 集成测试 - 验证核心功能
///
/// 测试目标：
/// 1. 中文路径支持
/// 2. 配置兼容性（旧版本数据迁移）
/// 3. 临时项目创建
/// 4. 模板删除后确认弹窗消失
/// 5. 项目列表正确加载
use std::path::PathBuf;
use tempfile::tempdir;

// ── 测试 1: 中文路径支持 ──────────────────────────────────────────────────

#[test]
fn test_chinese_path_support() {
    let dir = tempdir().unwrap();
    let mgr = claude_launcher::core::project_manager::ProjectManager::new(
        dir.path().join("projects.json"),
    );

    // 测试中文项目名称
    let result = mgr.add_project(
        "我的测试项目".to_string(),
        "/tmp/test".to_string(),
    );
    assert!(result.is_ok(), "应该支持中文项目名称");

    let project = result.unwrap();
    assert_eq!(project.name, "我的测试项目");

    // 测试中文路径
    let result2 = mgr.add_project(
        "项目2".to_string(),
        "/tmp/中文路径/测试".to_string(),
    );
    assert!(result2.is_ok(), "应该支持中文路径");
}

// ── 测试 2: 配置兼容性（旧版本数据迁移）──────────────────────────────────

#[test]
fn test_legacy_config_migration() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("projects.json");

    // 写入旧版本格式数据
    let legacy_data = r#"{
        "projects": [
            {
                "id": "old-id-1",
                "name": "旧项目1",
                "path": "/tmp/old1",
                "created_at": "2024-01-01T00:00:00Z"
            },
            {
                "id": "old-id-2",
                "name": "旧项目2",
                "path": "/tmp/old2",
                "created_at": "2024-01-02T00:00:00Z"
            }
        ]
    }"#;
    std::fs::write(&path, legacy_data).unwrap();

    // 使用新版本加载
    let mgr = claude_launcher::core::project_manager::ProjectManager::new(path.clone());
    let projects = mgr.list_projects().unwrap();

    // 应该能正确加载旧数据
    assert_eq!(projects.len(), 2, "应该加载2个旧项目");
    assert!(projects.iter().any(|p| p.name == "旧项目1"));
    assert!(projects.iter().any(|p| p.name == "旧项目2"));

    // 验证数据已迁移到新格式
    let content = std::fs::read_to_string(&path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed.is_object(), "应该是新格式的对象");
    assert!(
        parsed.get("projects").is_none(),
        "不应该再有 projects 字段"
    );
}

// ── 测试 3: 临时项目创建 ──────────────────────────────────────────────────

#[test]
fn test_temp_project_creation() {
    let dir = tempdir().unwrap();
    let mgr = claude_launcher::core::project_manager::ProjectManager::new(
        dir.path().join("projects.json"),
    );

    // 创建临时项目
    let now = chrono::Local::now();
    let name = format!("temp_{}", now.format("%Y%m%d_%H%M%S"));
    let temp_base = std::env::temp_dir().join("claude-projects");
    let _ = std::fs::create_dir_all(&temp_base);
    let temp_path = temp_base.join(&name);
    let _ = std::fs::create_dir_all(&temp_path);
    let path = temp_path.to_string_lossy().to_string();

    let result = mgr.add_project(name.clone(), path.clone());
    assert!(result.is_ok(), "临时项目创建应该成功");

    let project = result.unwrap();
    assert_eq!(project.name, name);
    assert_eq!(project.path, path);

    // 验证项目已保存
    let projects = mgr.list_projects().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, name);

    // 清理
    let _ = std::fs::remove_dir_all(&temp_path);
}

// ── 测试 4: 模板删除功能 ──────────────────────────────────────────────────

#[test]
fn test_template_deletion() {
    let dir = tempdir().unwrap();
    let mgr = claude_launcher::core::templates_manager::TemplatesManager::new(
        dir.path().join("templates.json"),
    );

    // 获取默认模板
    let default = mgr.get_default_template().unwrap();
    let default_id = default.id.clone();

    // 创建新模板
    let template = mgr
        .create_template(
            "测试模板".to_string(),
            serde_json::json!({"test": true}),
        )
        .unwrap();

    // 删除非默认模板应该成功
    let result = mgr.delete_template(&template.id);
    assert!(result.is_ok(), "删除非默认模板应该成功");

    // 验证模板已删除
    let templates = mgr.list_templates().unwrap();
    assert!(
        !templates.iter().any(|t| t.id == template.id),
        "模板应该已被删除"
    );

    // 删除默认模板应该失败
    let result = mgr.delete_template(&default_id);
    assert!(result.is_err(), "不能删除默认模板");
}

// ── 测试 5: 项目列表正确加载 ──────────────────────────────────────────────

#[test]
fn test_project_list_loading() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("projects.json");

    // 创建项目管理器并添加项目
    let mgr = claude_launcher::core::project_manager::ProjectManager::new(path.clone());
    mgr.add_project("项目A".to_string(), "/tmp/a".to_string())
        .unwrap();
    mgr.add_project("项目B".to_string(), "/tmp/b".to_string())
        .unwrap();
    mgr.add_project("项目C".to_string(), "/tmp/c".to_string())
        .unwrap();

    // 重新加载项目列表
    let projects = mgr.list_projects().unwrap();
    assert_eq!(projects.len(), 3, "应该有3个项目");

    // 验证项目名称
    let names: Vec<_> = projects.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"项目A"));
    assert!(names.contains(&"项目B"));
    assert!(names.contains(&"项目C"));
}

// ── 测试 6: 配置注入功能 ──────────────────────────────────────────────────

#[test]
fn test_config_injection() {
    let config = serde_json::json!({});
    let mut enabled = std::collections::HashMap::new();
    enabled.insert("agent_teams".to_string(), true);
    enabled.insert("thinking_mode".to_string(), true);

    let result = claude_launcher::core::config_injector::inject_items(&config, &enabled);

    assert_eq!(
        result["env"]["CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS"],
        "1"
    );
    assert_eq!(result["alwaysThinkingEnabled"], true);
}

// ── 测试 7: 备份管理功能 ──────────────────────────────────────────────────

#[test]
fn test_backup_operations() {
    let dir = tempdir().unwrap();
    let mgr = claude_launcher::core::backup_manager::BackupManager::new(
        dir.path().to_path_buf(),
    );

    // 初始状态
    let status = mgr.get_status();
    assert!(!status.settings_exists);
    assert!(!status.backup_exists);

    // 写入设置
    mgr.write_settings(r#"{"test": true}"#).unwrap();
    let status = mgr.get_status();
    assert!(status.settings_exists);

    // 备份
    mgr.backup().unwrap();
    let status = mgr.get_status();
    assert!(status.backup_exists);

    // 禁用
    mgr.disable().unwrap();
    let status = mgr.get_status();
    assert!(!status.settings_exists);
    assert!(status.settings_disabled);

    // 恢复
    mgr.restore().unwrap();
    let status = mgr.get_status();
    assert!(status.settings_exists);
    assert!(!status.settings_disabled);
}
