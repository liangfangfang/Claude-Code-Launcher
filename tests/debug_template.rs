/// 调试模板创建和显示问题
use std::collections::HashMap;

#[test]
fn debug_template_create_and_list() {
    let temp_dir = tempfile::tempdir().unwrap();
    let templates_file = temp_dir.path().join("templates.json");

    // 创建模板管理器
    let mgr = claude_launcher::core::templates_manager::TemplatesManager::new(
        templates_file.clone(),
    );

    // 初始状态：应该有默认模板
    let initial = mgr.list_templates().unwrap();
    println!("初始模板数量: {}", initial.len());
    for t in &initial {
        println!("  - {} ({})", t.name, t.id);
    }

    // 创建新模板
    let new_template = mgr
        .create_template("我的模板".to_string(), serde_json::json!({"test": true}))
        .unwrap();
    println!("创建模板: {} ({})", new_template.name, new_template.id);

    // 再次列出模板
    let after_create = mgr.list_templates().unwrap();
    println!("创建后模板数量: {}", after_create.len());
    for t in &after_create {
        println!("  - {} ({})", t.name, t.id);
    }

    // 验证新模板存在
    let found = after_create.iter().find(|t| t.name == "我的模板");
    assert!(found.is_some(), "新创建的模板应该在列表中");

    // 验证文件内容
    let file_content = std::fs::read_to_string(&templates_file).unwrap();
    println!("文件内容:\n{file_content}");

    // 重新加载验证持久化
    let mgr2 = claude_launcher::core::templates_manager::TemplatesManager::new(
        templates_file,
    );
    let reloaded = mgr2.list_templates().unwrap();
    println!("重新加载后模板数量: {}", reloaded.len());
    assert_eq!(reloaded.len(), after_create.len(), "重新加载后模板数量应该一致");
}

#[test]
fn debug_template_names_list() {
    let temp_dir = tempfile::tempdir().unwrap();
    let templates_file = temp_dir.path().join("templates.json");

    let mgr = claude_launcher::core::templates_manager::TemplatesManager::new(
        templates_file,
    );

    // 创建几个模板
    mgr.create_template("模板A".to_string(), serde_json::json!({})).unwrap();
    mgr.create_template("模板B".to_string(), serde_json::json!({})).unwrap();

    // 获取名称列表
    let names: Vec<String> = mgr
        .list_templates()
        .unwrap()
        .into_iter()
        .map(|t| t.name)
        .collect();

    println!("模板名称列表: {:?}", names);
    assert!(names.contains(&"模板A".to_string()));
    assert!(names.contains(&"模板B".to_string()));
}

#[test]
fn debug_group_create_and_list() {
    let temp_dir = tempfile::tempdir().unwrap();
    let groups_file = temp_dir.path().join("groups.json");

    let mgr = claude_launcher::core::group_manager::GroupManager::new(groups_file);

    // 创建分组
    let group = mgr.create_group("开发".to_string()).unwrap();
    println!("创建分组: {} ({})", group.name, group.id);

    // 列出分组
    let groups = mgr.list_groups().unwrap();
    println!("分组数量: {}", groups.len());
    for g in &groups {
        println!("  - {} ({})", g.name, g.id);
    }

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "开发");
}

#[test]
fn debug_project_with_group() {
    let temp_dir = tempfile::tempdir().unwrap();
    let projects_file = temp_dir.path().join("projects.json");
    let groups_file = temp_dir.path().join("groups.json");

    // 创建分组
    let group_mgr = claude_launcher::core::group_manager::GroupManager::new(groups_file);
    let group = group_mgr.create_group("测试分组".to_string()).unwrap();

    // 创建项目
    let project_mgr = claude_launcher::core::project_manager::ProjectManager::new(projects_file);
    let project = project_mgr
        .add_project("测试项目".to_string(), "/tmp/test".to_string())
        .unwrap();

    // 使用 update_project_group 设置分组
    let updated = project_mgr
        .update_project_group(&project.id, Some(group.id.clone()))
        .unwrap();

    // 验证分组
    assert_eq!(updated.group_id, Some(group.id.clone()));
    println!("项目分组: {:?}", updated.group_id);

    // 验证持久化
    let projects = project_mgr.list_projects().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].group_id, Some(group.id));
}
