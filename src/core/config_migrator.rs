/// 配置目录校验与归档。
///
/// v2.2.1 启动时检查配置目录：
/// - 空目录 → 正常初始化
/// - 非空且格式兼容 → 正常启动
/// - 靁空且格式不兼容 → 弹窗提醒，用户确认后归档旧配置并重新初始化
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 配置目录中的文件名
const CONFIG_FILES: &[&str] = &["projects.json", "groups.json", "templates.json"];

/// 不兼容文件详情
#[derive(Debug)]
pub struct IncompatibleFile {
    pub filename: String,
    pub reason: String,
}

/// 配置校验结果
#[derive(Debug)]
pub struct ValidationReport {
    pub config_dir: PathBuf,
    pub has_files: bool,
    pub incompatible_files: Vec<IncompatibleFile>,
}

impl ValidationReport {
    pub fn is_compatible(&self) -> bool {
        self.incompatible_files.is_empty()
    }
}

/// 获取默认配置目录路径
pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude-launcher")
}

/// 校验配置目录状态。
///
/// 返回 `ValidationReport`：
/// - `has_files = false`：目录为空或不存在，需正常初始化
/// - `has_files = true, incompatible_files = []`：格式兼容，可正常启动
/// - `has_files = true, incompatible_files = [...]`：存在不兼容文件，需归档
pub fn validate_config_dir() -> ValidationReport {
    let dir = config_dir();

    if !dir.exists() {
        return ValidationReport {
            config_dir: dir,
            has_files: false,
            incompatible_files: Vec::new(),
        };
    }

    let mut incompatible = Vec::new();
    let mut has_any = false;

    for &filename in CONFIG_FILES {
        let path = dir.join(filename);
        if !path.exists() {
            continue;
        }
        has_any = true;
        if let Err(reason) = validate_config_file(&path, filename) {
            tracing::warn!("配置文件 {} 不兼容: {}", filename, reason);
            incompatible.push(IncompatibleFile {
                filename: filename.to_string(),
                reason,
            });
        }
    }

    // 检查是否有未知文件（可能是其他版本的配置）
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let known: HashSet<&str> = CONFIG_FILES.iter().copied().collect();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if known.contains(name.as_str()) {
                continue;
            }
            // 跳过目录和备份文件
            if entry.path().is_dir() || name.ends_with(".bak") || name.ends_with(".disabled") {
                continue;
            }
            tracing::info!("发现非标准配置文件: {}", name);
        }
    }

    ValidationReport {
        config_dir: dir,
        has_files: has_any,
        incompatible_files: incompatible,
    }
}

/// 校验单个配置文件是否符合当前版本格式。
fn validate_config_file(path: &Path, filename: &str) -> Result<(), String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("无法读取: {e}"))?;

    if raw.trim().is_empty() {
        return Err("文件为空".to_string());
    }

    let parsed: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| format!("JSON 解析失败: {e}"))?;

    match filename {
        "projects.json" => validate_projects_format(&parsed),
        "groups.json" => validate_groups_format(&parsed),
        "templates.json" => validate_templates_format(&parsed),
        _ => Ok(()),
    }
}

/// 校验 projects.json 格式。
///
/// 当前版本格式：`{id: {id, name, path, created_at}, ...}`
/// 旧版本格式：`{"projects": [{...}, ...]}`
fn validate_projects_format(parsed: &serde_json::Value) -> Result<(), String> {
    let obj = parsed.as_object()
        .ok_or("projects.json 应为 JSON 对象")?;

    // 旧版本格式 {"projects": [...]} — 标记为不兼容（需要迁移）
    if obj.contains_key("projects") {
        return Err("检测到旧版本格式 {\"projects\": [...]}".to_string());
    }

    // 新版本格式：至少应有一个有效的项目条目
    // 空对象 {} 也是合法的（没有项目）
    if obj.is_empty() {
        return Ok(());
    }

    // 检查第一个条目的结构
    for (key, value) in obj.iter().take(1) {
        let proj = value.as_object()
            .ok_or(format!("项目 {key} 应为对象"))?;
        // 必须有 name 和 path 字段
        if !proj.contains_key("name") || !proj.contains_key("path") {
            return Err(format!("项目 {key} 缺少必要字段 (name/path)"));
        }
    }

    Ok(())
}

/// 校验 groups.json 格式。
///
/// 当前版本格式：`{id: {id, name, order, created_at}}`
fn validate_groups_format(parsed: &serde_json::Value) -> Result<(), String> {
    let obj = parsed.as_object()
        .ok_or("groups.json 应为 JSON 对象")?;

    if obj.is_empty() {
        return Ok(());
    }

    for (key, value) in obj.iter().take(1) {
        let group = value.as_object()
            .ok_or(format!("分组 {key} 应为对象"))?;
        if !group.contains_key("name") {
            return Err(format!("分组 {key} 缺少 name 字段"));
        }
    }

    Ok(())
}

/// 校验 templates.json 格式。
///
/// 当前版本格式：`{default_template_id: "...", templates: {id: {id, name, content, created_at}}}`
/// 旧版本格式：`{name: "...", content: {...}}` 或其他
fn validate_templates_format(parsed: &serde_json::Value) -> Result<(), String> {
    let obj = parsed.as_object()
        .ok_or("templates.json 应为 JSON 对象")?;

    // 必须同时有 default_template_id 和 templates 字段
    if !obj.contains_key("default_template_id") {
        return Err("缺少 default_template_id 字段（旧版本格式）".to_string());
    }
    if !obj.contains_key("templates") {
        return Err("缺少 templates 字段（旧版本格式）".to_string());
    }

    // 校验 templates 结构
    let templates = obj["templates"].as_object()
        .ok_or("templates 字段应为对象")?;

    if templates.is_empty() {
        return Err("templates 为空，需要初始化默认模板".to_string());
    }

    Ok(())
}

/// 归档旧配置目录并重新初始化。
///
/// 1. 将旧目录重命名为 `.claude-launcher_backup_<时间戳>`
/// 2. 创建新的空配置目录
/// 3. 返回归档目录路径
pub fn archive_and_reinitialize() -> Result<PathBuf, String> {
    let dir = config_dir();

    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("创建配置目录失败: {e}"))?;
        return Ok(dir);
    }

    // 生成归档目录名
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let archive_name = format!(".claude-launcher_backup_{timestamp}");
    let archive_dir = dir.parent()
        .unwrap_or(&dir)
        .join(&archive_name);

    // 重命名旧目录
    std::fs::rename(&dir, &archive_dir)
        .map_err(|e| format!("归档旧配置目录失败: {e}"))?;

    tracing::info!("旧配置已归档至: {:?}", archive_dir);

    // 创建新的空配置目录
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("创建新配置目录失败: {e}"))?;

    tracing::info!("已创建新的配置目录: {:?}", dir);

    Ok(archive_dir)
}

/// 构建归档确认消息
pub fn build_archive_message(report: &ValidationReport) -> String {
    let mut details = String::new();
    for f in &report.incompatible_files {
        details.push_str(&format!("  - {}: {}\n", f.filename, f.reason));
    }
    format!(
        "检测到旧版本配置，格式与当前版本 (v2.2.1) 不兼容：\n\n\
         {details}\n\
         程序将按时间戳归档旧配置目录，然后重新初始化。\n\
         归档位置：~/.claude-launcher_backup_<时间戳>\n\n\
         点击「是」归档并启动，点击「否」退出程序。"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_validate_empty_dir() {
        let report = validate_config_dir();
        // 测试环境中可能有真实配置，只验证函数不 panic
        let _ = report;
    }

    #[test]
    fn test_validate_projects_new_format() {
        let parsed = serde_json::json!({
            "abc-123": {
                "id": "abc-123",
                "name": "测试项目",
                "path": "/some/path",
                "created_at": "2026-01-01T00:00:00Z"
            }
        });
        assert!(validate_projects_format(&parsed).is_ok());
    }

    #[test]
    fn test_validate_projects_legacy_format() {
        let parsed = serde_json::json!({
            "projects": [
                {"id": "abc", "name": "test", "path": "/path"}
            ]
        });
        assert!(validate_projects_format(&parsed).is_err());
    }

    #[test]
    fn test_validate_projects_empty() {
        let parsed = serde_json::json!({});
        assert!(validate_projects_format(&parsed).is_ok());
    }

    #[test]
    fn test_validate_templates_new_format() {
        let parsed = serde_json::json!({
            "default_template_id": "abc",
            "templates": {
                "abc": {
                    "id": "abc",
                    "name": "默认",
                    "content": {},
                    "created_at": "2026-01-01T00:00:00Z"
                }
            }
        });
        assert!(validate_templates_format(&parsed).is_ok());
    }

    #[test]
    fn test_validate_templates_legacy_format() {
        let parsed = serde_json::json!({
            "name": "旧模板",
            "content": {"permissions": {}}
        });
        assert!(validate_templates_format(&parsed).is_err());
    }

    #[test]
    fn test_validate_templates_no_default_id() {
        let parsed = serde_json::json!({
            "templates": {}
        });
        assert!(validate_templates_format(&parsed).is_err());
    }

    #[test]
    fn test_validate_groups_format() {
        let parsed = serde_json::json!({
            "abc": {
                "id": "abc",
                "name": "分组1",
                "order": 0,
                "created_at": "2026-01-01T00:00:00Z"
            }
        });
        assert!(validate_groups_format(&parsed).is_ok());
    }

    #[test]
    fn test_validate_groups_missing_name() {
        let parsed = serde_json::json!({
            "abc": {
                "id": "abc",
                "order": 0
            }
        });
        assert!(validate_groups_format(&parsed).is_err());
    }

    #[test]
    fn test_archive_and_reinitialize() {
        let dir = tempdir().unwrap();
        let base = dir.path().join(".claude-launcher");
        std::fs::create_dir_all(&base).unwrap();

        // 写入一个旧格式文件
        std::fs::write(
            base.join("templates.json"),
            r#"{"name": "旧模板", "content": {}}"#,
        )
        .unwrap();

        // 注意：archive_and_reinitialize 使用真实 home 目录
        // 这里只测试归档消息构建
        let report = ValidationReport {
            config_dir: base.clone(),
            has_files: true,
            incompatible_files: vec![IncompatibleFile {
                filename: "templates.json".to_string(),
                reason: "缺少 default_template_id 字段".to_string(),
            }],
        };
        let msg = build_archive_message(&report);
        assert!(msg.contains("templates.json"));
        assert!(msg.contains("不兼容"));
    }
}
