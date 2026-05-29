/// 真实场景测试 - 使用真实用户数据验证问题
///
/// 测试目标：用真实数据和场景来发现实际问题
use std::path::PathBuf;

// ── 测试环境设置 ──────────────────────────────────────────────────────────

/// 获取用户的真实数据目录
fn get_real_data_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(".claude-launcher")
}

/// 测试 1: 验证能否加载真实用户的项目数据
#[test]
fn test_load_real_projects() {
    let data_dir = get_real_data_dir();
    let projects_file = data_dir.join("projects.json");

    // 如果文件不存在，跳过测试
    if !projects_file.exists() {
        println!("跳过：projects.json 不存在");
        return;
    }

    let mgr = claude_launcher::core::project_manager::ProjectManager::new(projects_file);

    // 尝试加载项目列表
    match mgr.list_projects() {
        Ok(projects) => {
            println!("成功加载 {} 个项目", projects.len());
            for p in &projects {
                println!("  - {} ({})", p.name, p.path);
            }
            // 验证中文路径项目
            let chinese_projects: Vec<_> = projects
                .iter()
                .filter(|p| p.name.chars().any(|c| c > '\u{7F}'))
                .collect();
            if !chinese_projects.is_empty() {
                println!("发现 {} 个中文名称项目", chinese_projects.len());
            }
        }
        Err(e) => {
            panic!("加载项目失败: {e}");
        }
    }
}

/// 测试 2: 验证能否加载真实用户的模板数据
#[test]
fn test_load_real_templates() {
    let data_dir = get_real_data_dir();
    let templates_file = data_dir.join("templates.json");
    let old_template_file = data_dir.join("settings_template.json");

    // 检查旧格式文件是否存在
    if old_template_file.exists() {
        println!("发现旧格式模板文件: {}", old_template_file.display());
        let content = std::fs::read_to_string(&old_template_file).unwrap();
        println!("旧模板内容: {}", &content[..200.min(content.len())]);
    }

    // 创建模板管理器（这会触发迁移）
    let mgr = claude_launcher::core::templates_manager::TemplatesManager::new(templates_file);

    // 尝试加载模板列表
    match mgr.list_templates() {
        Ok(templates) => {
            println!("成功加载 {} 个模板", templates.len());
            for t in &templates {
                println!("  - {}", t.name);
            }
        }
        Err(e) => {
            panic!("加载模板失败: {e}");
        }
    }

    // 尝试获取默认模板
    match mgr.get_default_template() {
        Ok(default) => {
            println!("默认模板: {}", default.name);
        }
        Err(e) => {
            panic!("获取默认模板失败: {e}");
        }
    }
}

/// 测试 3: 验证旧格式配置迁移
#[test]
fn test_legacy_config_migration_real() {
    let data_dir = get_real_data_dir();
    let old_file = data_dir.join("settings_template.json");
    let new_file = data_dir.join("templates.json");

    if !old_file.exists() {
        println!("跳过：旧格式文件不存在");
        return;
    }

    // 读取旧格式
    let old_content = std::fs::read_to_string(&old_file).unwrap();
    let old_value: serde_json::Value = serde_json::from_str(&old_content).unwrap();

    println!("旧格式结构:");
    if let Some(obj) = old_value.as_object() {
        for key in obj.keys() {
            println!("  - {key}");
        }
    }

    // 读取新格式（如果存在）
    if new_file.exists() {
        let new_content = std::fs::read_to_string(&new_file).unwrap();
        let new_value: serde_json::Value = serde_json::from_str(&new_content).unwrap();

        println!("新格式结构:");
        if let Some(obj) = new_value.as_object() {
            for key in obj.keys() {
                println!("  - {key}");
            }
        }

        // 检查是否包含旧配置
        if let Some(templates) = new_value.get("templates") {
            if let Some(map) = templates.as_object() {
                println!("模板数量: {}", map.len());
                for (id, template) in map {
                    if let Some(name) = template.get("name") {
                        println!("  模板: {name} (id: {id})");
                    }
                }
            }
        }
    }
}

/// 测试 4: 验证中文路径处理
#[test]
fn test_chinese_path_handling() {
    // 测试各种中文路径
    let test_paths = vec![
        "G:/develop/claude/本地claude",
        "G:/develop/claude/agentic方案提升",
        "C:\\Users\\Akc\\AppData\\Local\\Temp\\claude-projects\\temp_20260419_193231",
    ];

    for path in test_paths {
        println!("测试路径: {path}");

        // 测试路径是否存在
        let exists = std::path::Path::new(path).exists();
        println!("  路径存在: {exists}");

        // 测试是否能作为 UTF-8 字符串处理
        let bytes = path.as_bytes();
        println!("  字节长度: {}", bytes.len());
        println!("  字符长度: {}", path.chars().count());
    }
}

/// 测试 5: 验证项目管理器的完整流程
#[test]
fn test_project_manager_full_flow() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mgr = claude_launcher::core::project_manager::ProjectManager::new(
        temp_dir.path().join("projects.json"),
    );

    // 创建项目
    let p1 = mgr
        .add_project("测试项目1".to_string(), "/tmp/test1".to_string())
        .unwrap();
    println!("创建项目: {} ({})", p1.name, p1.id);

    let p2 = mgr
        .add_project("测试项目2".to_string(), "/tmp/test2".to_string())
        .unwrap();
    println!("创建项目: {} ({})", p2.name, p2.id);

    // 列出项目
    let projects = mgr.list_projects().unwrap();
    println!("项目数量: {}", projects.len());
    assert_eq!(projects.len(), 2);

    // 更新项目
    let updated = mgr
        .update_project(&p1.id, Some("新名称".to_string()), None)
        .unwrap();
    println!("更新项目: {} -> 新名称", p1.name);
    assert_eq!(updated.name, "新名称");

    // 删除项目
    mgr.delete_project(&p2.id).unwrap();
    println!("删除项目: {}", p2.name);

    let projects = mgr.list_projects().unwrap();
    assert_eq!(projects.len(), 1);
}

/// 测试 6: 验证模板管理器的完整流程
#[test]
fn test_template_manager_full_flow() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mgr = claude_launcher::core::templates_manager::TemplatesManager::new(
        temp_dir.path().join("templates.json"),
    );

    // 应该有默认模板
    let default = mgr.get_default_template().unwrap();
    println!("默认模板: {}", default.name);

    // 创建新模板
    let t1 = mgr
        .create_template(
            "我的模板".to_string(),
            serde_json::json!({"test": true}),
        )
        .unwrap();
    println!("创建模板: {} ({})", t1.name, t1.id);

    // 列出模板
    let templates = mgr.list_templates().unwrap();
    println!("模板数量: {}", templates.len());
    assert_eq!(templates.len(), 2); // 默认 + 新建

    // 尝试删除默认模板（应该失败）
    let result = mgr.delete_template(&default.id);
    assert!(result.is_err());
    println!("删除默认模板失败（符合预期）");

    // 删除非默认模板
    mgr.delete_template(&t1.id).unwrap();
    println!("删除模板: {}", t1.name);

    let templates = mgr.list_templates().unwrap();
    assert_eq!(templates.len(), 1);
}
