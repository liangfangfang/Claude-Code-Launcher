/// 调试项目加载问题
#[test]
fn debug_projects_loading() {
    let home = dirs::home_dir().unwrap();
    let projects_file = home.join(".claude-launcher").join("projects.json");

    println!("项目文件路径: {}", projects_file.display());
    println!("文件存在: {}", projects_file.exists());

    if !projects_file.exists() {
        return;
    }

    let raw = std::fs::read_to_string(&projects_file).unwrap();
    println!("文件大小: {} 字节", raw.len());
    println!("前200字符: {}", &raw[..200.min(raw.len())]);

    // 尝试解析 JSON
    match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(value) => {
            println!("JSON 解析成功");
            if let Some(obj) = value.as_object() {
                println!("对象键数量: {}", obj.len());
                for (key, val) in obj {
                    println!("  键: {key}");
                    if let Some(project) = val.as_object() {
                        if let Some(created_at) = project.get("created_at") {
                            println!("    created_at: {created_at}");
                            // 尝试解析日期
                            let date_str = created_at.as_str().unwrap();
                            match chrono::DateTime::parse_from_rfc3339(date_str) {
                                Ok(dt) => println!("    日期解析成功: {dt}"),
                                Err(e) => println!("    日期解析失败: {e}"),
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("JSON 解析失败: {e}");
        }
    }

    // 尝试用 ProjectStore 解析
    println!("\n尝试用 ProjectStore 解析...");
    let result: Result<std::collections::HashMap<String, claude_launcher::core::models::Project>, _> =
        serde_json::from_str(&raw);
    match result {
        Ok(store) => {
            println!("ProjectStore 解析成功，项目数量: {}", store.len());
        }
        Err(e) => {
            println!("ProjectStore 解析失败: {e}");
        }
    }
}
