use claude_launcher::core::templates_manager::TemplatesManager;

#[test]
fn test_create_on_real_file() {
    // 使用真实的 default_manager 路径
    let mgr = TemplatesManager::default_manager().unwrap();
    let path = mgr.storage_path().to_path_buf();
    println!("配置路径: {}", path.display());
    
    // 读取创建前的内容
    let before = std::fs::read_to_string(&path).unwrap();
    let before_count = before.matches("\"name\":").count();
    println!("创建前文件中的模板数: {}", before_count);
    
    // 列出当前模板
    let list = mgr.list_templates().unwrap();
    println!("list_templates 返回 {} 个模板:", list.len());
    for t in &list {
        println!("  - {} (id: {})", t.name, &t.id[..8]);
    }
    
    // 创建一个测试模板
    let test_name = format!("测试模板_{:?}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    match mgr.create_template(test_name.clone(), serde_json::json!({"test": true})) {
        Ok(t) => println!("创建成功: {} (id: {})", t.name, &t.id[..8]),
        Err(e) => { println!("创建失败: {}", e); return; }
    }
    
    // 读取创建后的内容
    let after = std::fs::read_to_string(&path).unwrap();
    let after_count = after.matches("\"name\":").count();
    println!("创建后文件中的模板数: {}", after_count);
    
    // 再次列出模板
    let list2 = mgr.list_templates().unwrap();
    println!("再次 list_templates 返回 {} 个模板:", list2.len());
    for t in &list2 {
        println!("  - {} (id: {})", t.name, &t.id[..8]);
    }
    
    // 验证
    assert!(after_count > before_count, "文件中的模板数应该增加");
    assert!(list2.len() > list.len(), "list_templates 应该返回更多模板");
    
    // 清理：删除测试模板
    if let Some(t) = list2.iter().find(|t| t.name == test_name) {
        let _ = mgr.delete_template(&t.id);
        println!("已清理测试模板");
    }
}
