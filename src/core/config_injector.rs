/// Config Injector - detects and injects optional configuration items
/// into Claude Code settings.
///
/// Provides 7 injectable items matching the Python source exactly:
/// agent_teams, thinking_mode, skip_dangerous, mcp_web_search,
/// mcp_web_reader, npm_permissions, pip_permissions.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Injection mode for a config item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InjectionMode {
    /// Set a value at the given path (replaces existing).
    Set,
    /// Append values to a list at the given path.
    AppendList,
    /// Append multiple values to a list at the given path.
    AppendListMulti,
}

/// Description of an injectable configuration item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectableItem {
    /// Display label (Chinese).
    pub label: String,
    /// Description of what this item does (Chinese).
    pub description: String,
    /// JSON path segments to reach the target location.
    pub path: Vec<String>,
    /// Single value to set (used with `Set` mode).
    pub value: Option<serde_json::Value>,
    /// Multiple values to append (used with `AppendList` / `AppendListMulti`).
    pub values: Vec<serde_json::Value>,
    /// How to apply this item.
    pub mode: InjectionMode,
}

/// Builds the complete set of 7 injectable configuration items
/// matching the Python source definitions exactly.
pub fn build_injectable_items() -> HashMap<String, InjectableItem> {
    let mut items = HashMap::new();

    items.insert(
        "agent_teams".to_string(),
        InjectableItem {
            label: "启用 Agent Teams".to_string(),
            description: "启用实验性的 Agent Teams 功能".to_string(),
            path: vec![
                "env".to_string(),
                "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS".to_string(),
            ],
            value: Some(serde_json::Value::String("1".to_string())),
            values: vec![],
            mode: InjectionMode::Set,
        },
    );

    items.insert(
        "thinking_mode".to_string(),
        InjectableItem {
            label: "启用思考模式".to_string(),
            description: "始终启用思考模式，AI 会展示思考过程".to_string(),
            path: vec!["alwaysThinkingEnabled".to_string()],
            value: Some(serde_json::Value::Bool(true)),
            values: vec![],
            mode: InjectionMode::Set,
        },
    );

    items.insert(
        "skip_dangerous".to_string(),
        InjectableItem {
            label: "跳过危险操作确认".to_string(),
            description: "跳过危险操作的权限确认提示（谨慎使用）".to_string(),
            path: vec!["skipDangerousModePermissionPrompt".to_string()],
            value: Some(serde_json::Value::Bool(true)),
            values: vec![],
            mode: InjectionMode::Set,
        },
    );

    items.insert(
        "mcp_web_search".to_string(),
        InjectableItem {
            label: "MCP Web 搜索权限".to_string(),
            description: "允许使用 MCP Web 搜索工具".to_string(),
            path: vec!["permissions".to_string(), "allow".to_string()],
            value: None,
            values: vec![serde_json::Value::String(
                "mcp__web-search-prime__web_search_prime".to_string(),
            )],
            mode: InjectionMode::AppendList,
        },
    );

    items.insert(
        "mcp_web_reader".to_string(),
        InjectableItem {
            label: "MCP Web 读取器权限".to_string(),
            description: "允许使用 MCP Web 读取器工具".to_string(),
            path: vec!["permissions".to_string(), "allow".to_string()],
            value: None,
            values: vec![serde_json::Value::String(
                "mcp__web-reader__webReader".to_string(),
            )],
            mode: InjectionMode::AppendList,
        },
    );

    items.insert(
        "npm_permissions".to_string(),
        InjectableItem {
            label: "NPM 权限".to_string(),
            description: "允许执行 npm 常用命令（install、update、run、test）".to_string(),
            path: vec!["permissions".to_string(), "allow".to_string()],
            value: None,
            values: vec![
                serde_json::Value::String("Bash(npm run:*)".to_string()),
                serde_json::Value::String("Bash(npm test:*)".to_string()),
                serde_json::Value::String("Bash(npm install)".to_string()),
                serde_json::Value::String("Bash(npm update)".to_string()),
            ],
            mode: InjectionMode::AppendListMulti,
        },
    );

    items.insert(
        "pip_permissions".to_string(),
        InjectableItem {
            label: "PIP 权限".to_string(),
            description: "允许执行 pip install 安装 Python 包".to_string(),
            path: vec!["permissions".to_string(), "allow".to_string()],
            value: None,
            values: vec![
                serde_json::Value::String("Bash(pip install:*)".to_string()),
                serde_json::Value::String("Bash(python* -m pip install:*)".to_string()),
            ],
            mode: InjectionMode::AppendListMulti,
        },
    );

    items
}

/// Returns all injectable configuration items.
pub fn get_all_items() -> HashMap<String, InjectableItem> {
    build_injectable_items()
}

/// Detects which injectable items are currently active in the config.
///
/// For `Set` mode: the value at the path must equal the expected value.
/// For `AppendList`/`AppendListMulti` mode: ALL expected values must be
/// present in the target list (matching Python behavior).
pub fn detect_active_items(config: &serde_json::Value) -> HashMap<String, bool> {
    let items = build_injectable_items();
    let mut result = HashMap::new();
    for (key, item) in &items {
        result.insert(key.clone(), is_item_active(config, item));
    }
    result
}

/// Checks if a specific injectable item is active in the config.
fn is_item_active(config: &serde_json::Value, item: &InjectableItem) -> bool {
    match item.mode {
        InjectionMode::Set => {
            let current = get_by_path(config, &item.path);
            match (&item.value, current) {
                (Some(expected), Some(actual)) => expected == actual,
                _ => false,
            }
        }
        InjectionMode::AppendList | InjectionMode::AppendListMulti => {
            let current = get_by_path(config, &item.path);
            match current.and_then(|v| v.as_array()) {
                Some(arr) => item.values.iter().all(|v| arr.contains(v)),
                None => false,
            }
        }
    }
}

/// Injects or removes items. Keys with `true` are injected, `false` are removed.
///
/// Returns a new config (does not modify the original).
pub fn inject_items(
    config: &serde_json::Value,
    enabled_keys: &HashMap<String, bool>,
) -> serde_json::Value {
    let items = build_injectable_items();
    let mut result = config.clone();

    for (key, should_enable) in enabled_keys {
        if let Some(item) = items.get(key) {
            if *should_enable {
                inject_single(&mut result, item);
            } else {
                remove_single(&mut result, item);
            }
        }
    }

    result
}

/// Injects a single item (idempotent).
fn inject_single(config: &mut serde_json::Value, item: &InjectableItem) {
    match item.mode {
        InjectionMode::Set => {
            if let Some(value) = &item.value {
                set_by_path(config, &item.path, value.clone());
            }
        }
        InjectionMode::AppendList | InjectionMode::AppendListMulti => {
            let current = get_by_path(config, &item.path);
            let is_list = current.and_then(|v| v.as_array()).is_some();
            if !is_list {
                set_by_path(config, &item.path, serde_json::Value::Array(vec![]));
            }
            // Re-fetch mutable reference after possible mutation
            if let Some(arr) = get_by_path_mut(config, &item.path).and_then(|v| v.as_array_mut()) {
                for value in &item.values {
                    if !arr.contains(value) {
                        arr.push(value.clone());
                    }
                }
            }
        }
    }
}

/// Removes a single item.
fn remove_single(config: &mut serde_json::Value, item: &InjectableItem) {
    match item.mode {
        InjectionMode::Set => {
            remove_by_path(config, &item.path);
        }
        InjectionMode::AppendList | InjectionMode::AppendListMulti => {
            if let Some(arr) = get_by_path_mut(config, &item.path).and_then(|v| v.as_array_mut()) {
                for value in &item.values {
                    arr.retain(|v| v != value);
                }
            }
        }
    }
}

// ── Path-based helpers ──────────────────────────────────────────

fn get_by_path<'a>(
    config: &'a serde_json::Value,
    path: &[String],
) -> Option<&'a serde_json::Value> {
    let mut current = config;
    for key in path {
        current = current.get(key)?;
    }
    Some(current)
}

fn get_by_path_mut<'a>(
    config: &'a mut serde_json::Value,
    path: &[String],
) -> Option<&'a mut serde_json::Value> {
    let mut current = config;
    for key in path {
        current = current.get_mut(key)?;
    }
    Some(current)
}

fn set_by_path(config: &mut serde_json::Value, path: &[String], value: serde_json::Value) {
    if path.is_empty() {
        *config = value;
        return;
    }
    ensure_object(config);
    let mut current = config;
    for segment in &path[..path.len() - 1] {
        if !current[segment].is_object() {
            current.as_object_mut().unwrap().insert(
                segment.clone(),
                serde_json::Value::Object(serde_json::Map::new()),
            );
        }
        current = current.get_mut(segment).unwrap();
    }
    if let Some(obj) = current.as_object_mut() {
        obj.insert(path.last().unwrap().clone(), value);
    }
}

fn remove_by_path(config: &mut serde_json::Value, path: &[String]) {
    if path.is_empty() {
        return;
    }
    if path.len() == 1 {
        if let Some(obj) = config.as_object_mut() {
            obj.remove(&path[0]);
        }
        return;
    }
    let mut current = config;
    for segment in &path[..path.len() - 1] {
        match current.get_mut(segment) {
            Some(next) if next.is_object() => current = next,
            _ => return,
        }
    }
    if let Some(obj) = current.as_object_mut() {
        obj.remove(path.last().unwrap());
    }
}

fn ensure_object(val: &mut serde_json::Value) {
    if val.is_null() {
        *val = serde_json::Value::Object(serde_json::Map::new());
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seven_items_exist() {
        let items = get_all_items();
        assert_eq!(items.len(), 7);
        for key in &[
            "agent_teams",
            "thinking_mode",
            "skip_dangerous",
            "mcp_web_search",
            "mcp_web_reader",
            "npm_permissions",
            "pip_permissions",
        ] {
            assert!(items.contains_key(*key), "missing key: {}", key);
        }
    }

    #[test]
    fn test_agent_teams_definition() {
        let items = get_all_items();
        let at = items.get("agent_teams").unwrap();
        assert_eq!(at.label, "启用 Agent Teams");
        assert_eq!(at.path, vec!["env", "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS"]);
        assert_eq!(at.value, Some(serde_json::Value::String("1".to_string())));
        assert_eq!(at.mode, InjectionMode::Set);
    }

    #[test]
    fn test_mcp_web_search_definition() {
        let items = get_all_items();
        let ws = items.get("mcp_web_search").unwrap();
        assert_eq!(ws.label, "MCP Web 搜索权限");
        assert_eq!(ws.path, vec!["permissions", "allow"]);
        assert_eq!(
            ws.values,
            vec![serde_json::Value::String(
                "mcp__web-search-prime__web_search_prime".to_string()
            )]
        );
        assert_eq!(ws.mode, InjectionMode::AppendList);
    }

    #[test]
    fn test_npm_permissions_definition() {
        let items = get_all_items();
        let npm = items.get("npm_permissions").unwrap();
        assert_eq!(npm.values.len(), 4);
        assert!(
            npm.values
                .contains(&serde_json::Value::String("Bash(npm run:*)".to_string()))
        );
        assert!(
            npm.values
                .contains(&serde_json::Value::String("Bash(npm install)".to_string()))
        );
        assert_eq!(npm.mode, InjectionMode::AppendListMulti);
    }

    #[test]
    fn test_pip_permissions_definition() {
        let items = get_all_items();
        let pip = items.get("pip_permissions").unwrap();
        assert_eq!(pip.values.len(), 2);
        assert!(pip.values.contains(&serde_json::Value::String(
            "Bash(pip install:*)".to_string()
        )));
        assert!(pip.values.contains(&serde_json::Value::String(
            "Bash(python* -m pip install:*)".to_string()
        )));
    }

    #[test]
    fn test_detect_empty_config() {
        let config = serde_json::json!({});
        let active = detect_active_items(&config);
        for (key, is_active) in &active {
            assert!(!is_active, "{} should not be active in empty config", key);
        }
    }

    #[test]
    fn test_detect_agent_teams_active() {
        let config = serde_json::json!({"env": {"CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1"}});
        let active = detect_active_items(&config);
        assert!(active["agent_teams"]);
    }

    #[test]
    fn test_detect_agent_teams_wrong_value() {
        let config = serde_json::json!({"env": {"CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "0"}});
        let active = detect_active_items(&config);
        assert!(!active["agent_teams"]);
    }

    #[test]
    fn test_detect_thinking_mode() {
        let config = serde_json::json!({"alwaysThinkingEnabled": true});
        let active = detect_active_items(&config);
        assert!(active["thinking_mode"]);
    }

    #[test]
    fn test_detect_skip_dangerous() {
        let config = serde_json::json!({"skipDangerousModePermissionPrompt": true});
        let active = detect_active_items(&config);
        assert!(active["skip_dangerous"]);
    }

    #[test]
    fn test_detect_mcp_web_search_all_required() {
        // ALL values must be present
        let config = serde_json::json!({"permissions": {"allow": ["mcp__web-search-prime__web_search_prime"]}});
        let active = detect_active_items(&config);
        assert!(active["mcp_web_search"]);
    }

    #[test]
    fn test_detect_npm_partial_not_active() {
        // Only some values present → NOT active (Python: all() check)
        let config = serde_json::json!({"permissions": {"allow": ["Bash(npm run:*)"]}});
        let active = detect_active_items(&config);
        assert!(!active["npm_permissions"]);
    }

    #[test]
    fn test_detect_npm_all_active() {
        let config = serde_json::json!({"permissions": {"allow": [
            "Bash(npm run:*)", "Bash(npm test:*)", "Bash(npm install)", "Bash(npm update)"
        ]}});
        let active = detect_active_items(&config);
        assert!(active["npm_permissions"]);
    }

    #[test]
    fn test_inject_set_mode() {
        let config = serde_json::json!({});
        let mut keys = HashMap::new();
        keys.insert("agent_teams".to_string(), true);

        let result = inject_items(&config, &keys);
        assert_eq!(result["env"]["CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS"], "1");
    }

    #[test]
    fn test_inject_thinking_mode() {
        let config = serde_json::json!({});
        let mut keys = HashMap::new();
        keys.insert("thinking_mode".to_string(), true);

        let result = inject_items(&config, &keys);
        assert_eq!(result["alwaysThinkingEnabled"], true);
    }

    #[test]
    fn test_inject_append_creates_list() {
        let config = serde_json::json!({});
        let mut keys = HashMap::new();
        keys.insert("mcp_web_search".to_string(), true);

        let result = inject_items(&config, &keys);
        let arr = result["permissions"]["allow"].as_array().unwrap();
        assert!(arr.contains(&serde_json::Value::String(
            "mcp__web-search-prime__web_search_prime".to_string()
        )));
    }

    #[test]
    fn test_inject_append_dedup() {
        let config = serde_json::json!({"permissions": {"allow": ["mcp__web-search-prime__web_search_prime"]}});
        let mut keys = HashMap::new();
        keys.insert("mcp_web_search".to_string(), true);

        let result = inject_items(&config, &keys);
        let arr = result["permissions"]["allow"].as_array().unwrap();
        let count = arr
            .iter()
            .filter(|v| {
                *v == &serde_json::Value::String(
                    "mcp__web-search-prime__web_search_prime".to_string(),
                )
            })
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_inject_multiple() {
        let config = serde_json::json!({});
        let mut keys = HashMap::new();
        keys.insert("thinking_mode".to_string(), true);
        keys.insert("skip_dangerous".to_string(), true);
        keys.insert("agent_teams".to_string(), true);

        let result = inject_items(&config, &keys);
        assert_eq!(result["alwaysThinkingEnabled"], true);
        assert_eq!(result["skipDangerousModePermissionPrompt"], true);
        assert_eq!(result["env"]["CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS"], "1");
    }

    #[test]
    fn test_remove_set_mode() {
        let config = serde_json::json!({"alwaysThinkingEnabled": true, "keep": "me"});
        let mut keys = HashMap::new();
        keys.insert("thinking_mode".to_string(), false);

        let result = inject_items(&config, &keys);
        assert!(result.get("alwaysThinkingEnabled").is_none());
        assert_eq!(result["keep"], "me");
    }

    #[test]
    fn test_remove_append_values() {
        let config = serde_json::json!({"permissions": {"allow": [
            "mcp__web-search-prime__web_search_prime", "other_perm"
        ]}});
        let mut keys = HashMap::new();
        keys.insert("mcp_web_search".to_string(), false);

        let result = inject_items(&config, &keys);
        let arr = result["permissions"]["allow"].as_array().unwrap();
        assert!(!arr.contains(&serde_json::Value::String(
            "mcp__web-search-prime__web_search_prime".to_string()
        )));
        assert!(arr.contains(&serde_json::Value::String("other_perm".to_string())));
    }

    #[test]
    fn test_inject_preserves_existing() {
        let config = serde_json::json!({"custom_key": "custom_value", "permissions": {"allow": ["custom_perm"]}});
        let mut keys = HashMap::new();
        keys.insert("npm_permissions".to_string(), true);

        let result = inject_items(&config, &keys);
        assert_eq!(result["custom_key"], "custom_value");
        let arr = result["permissions"]["allow"].as_array().unwrap();
        assert!(arr.contains(&serde_json::Value::String("custom_perm".to_string())));
        assert!(arr.contains(&serde_json::Value::String("Bash(npm run:*)".to_string())));
    }

    #[test]
    fn test_unknown_key_ignored() {
        let config = serde_json::json!({"keep": true});
        let mut keys = HashMap::new();
        keys.insert("nonexistent".to_string(), true);

        let result = inject_items(&config, &keys);
        assert_eq!(result["keep"], true);
    }

    #[test]
    fn test_detect_multiple_active() {
        let config = serde_json::json!({
            "alwaysThinkingEnabled": true,
            "skipDangerousModePermissionPrompt": true,
            "env": {"CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1"},
            "permissions": {"allow": [
                "mcp__web-reader__webReader",
                "Bash(pip install:*)",
                "Bash(python* -m pip install:*)"
            ]}
        });
        let active = detect_active_items(&config);
        assert!(active["thinking_mode"]);
        assert!(active["skip_dangerous"]);
        assert!(active["agent_teams"]);
        assert!(active["mcp_web_reader"]);
        assert!(active["pip_permissions"]);
        assert!(!active["mcp_web_search"]);
        assert!(!active["npm_permissions"]);
    }

    // ── Path helper tests ───────────────────────────────────────

    #[test]
    fn test_get_by_path() {
        let config = serde_json::json!({"a": {"b": 42}});
        assert_eq!(
            get_by_path(&config, &["a".to_string(), "b".to_string()]),
            Some(&serde_json::json!(42))
        );
    }

    #[test]
    fn test_get_by_path_missing() {
        let config = serde_json::json!({"a": 1});
        assert!(get_by_path(&config, &["a".to_string(), "b".to_string()]).is_none());
    }

    #[test]
    fn test_set_by_path_creates_nested() {
        let mut config = serde_json::json!({});
        set_by_path(
            &mut config,
            &["a".to_string(), "b".to_string()],
            serde_json::json!(42),
        );
        assert_eq!(config["a"]["b"], 42);
    }

    #[test]
    fn test_remove_by_path() {
        let mut config = serde_json::json!({"a": {"b": "del", "c": "keep"}});
        remove_by_path(&mut config, &["a".to_string(), "b".to_string()]);
        assert!(config["a"]["b"].is_null());
        assert_eq!(config["a"]["c"], "keep");
    }
}
