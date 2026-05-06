# Claude Code Launcher - 需求规格书 (Rust 重写)

> 版本: 2.0.0 | 技术栈: Rust + iced | 日期: 2026-04-14

## 1. 项目概述

Claude Code 启动器是一个 Windows 桌面 GUI 应用，用于管理多个 Claude Code 项目，
提供一键启动、配置模板管理、全局设置备份等功能。

## 2. 功能需求

### F-01 项目管理 (P0)

| ID | 功能 | 描述 |
|----|------|------|
| F-01-01 | 添加项目 | 输入名称+选择目录，自动生成 UUID，路径去重，可选配置模板 |
| F-01-02 | 编辑项目 | 修改名称/路径，路径去重校验，可选重新应用配置模板 |
| F-01-03 | 删除项目 | 从启动器移除（带确认），不删除磁盘文件 |
| F-01-04 | 项目列表 | 卡片式展示，可滚动，空列表引导提示 |
| F-01-05 | 数据持久化 | JSON 存储 `~/.claude-launcher/projects.json`，UTF-8，支持旧格式迁移 |
| F-01-06 | 临时项目 | 一键创建 `temp_YYYYMMDD_HHMMSS` 项目于 `%TEMP%/claude-projects/` |

### F-02 项目卡片 (P0)

| ID | 功能 | 描述 |
|----|------|------|
| F-02-01 | 信息展示 | 蓝色圆点图标 + 项目名称(粗体) + 路径(灰色) |
| F-02-02 | 启动按钮 | 在终端中启动 Claude Code，支持 --dangerously-skip-permissions 和 --continue |
| F-02-03 | 配置按钮 | 打开项目级 settings.local.json 编辑器 |
| F-02-04 | 删除按钮 | 带确认的项目删除 |
| F-02-05 | 跳过权限复选框 | 附加 --dangerously-skip-permissions |
| F-02-06 | 继续会话复选框 | 附加 --continue |
| F-02-07 | 打开目录按钮 | 在 Windows 资源管理器中打开项目目录 |

### F-03 终端启动 (P0)

| ID | 功能 | 描述 |
|----|------|------|
| F-03-01 | WT 检测 | 检测 PATH 和 `%LOCALAPPDATA%\Microsoft\WindowsApps\wt.exe` |
| F-03-02 | WT 启动 | `wt.exe -d <path> --title <name> -- powershell.exe -NoExit -ExecutionPolicy Bypass -File <ps1>` |
| F-03-03 | CMD 回退 | `cmd.exe /k <bat>` |
| F-03-04 | PowerShell 回退 | `powershell.exe -NoExit -ExecutionPolicy Bypass -File <ps1>` |
| F-03-05 | 工作目录切换 | 各终端类型均正确切换到项目路径 |

### F-04 配置模板管理 (P1)

| ID | 功能 | 描述 |
|----|------|------|
| F-04-01 | 模板 CRUD | 创建/编辑/删除/设为默认，上限 10 个，默认模板不可删除 |
| F-04-02 | 备份/失效/恢复 | 管理 `~/.claude/settings.json`，首次启动自动备份 |
| F-04-03 | 配置项注入 | 快捷注入 Agent Teams、Thinking Mode、权限等预设配置 |
| F-04-04 | 模板应用 | 写入项目 `.claude/settings.local.json`，与全局设置合并 |
| F-04-05 | 添加时选模板 | 添加项目对话框可选择配置模板 |
| F-04-06 | 编辑时选模板 | 编辑项目对话框可选择重新应用模板 |

### F-05 对话框 (P0-P1)

| ID | 功能 | 描述 |
|----|------|------|
| F-05-01 | 添加项目对话框 | 名称输入、路径浏览、模板选择 |
| F-05-02 | 编辑项目对话框 | 修改名称/路径、模板选择 |
| F-05-03 | 设置管理对话框 | 3 Tab: 备份管理、模板管理、全局配置 |
| F-05-04 | 项目配置对话框 | JSON 编辑器、配置项注入、模板应用、保存为模板 |

### F-06 主界面 (P0)

| ID | 功能 | 描述 |
|----|------|------|
| F-06-01 | 工具栏 | 标题 + 设置/临时/添加/关于 按钮 |
| F-06-02 | 可滚动列表 | CTkScrollableFrame 等效 |
| F-06-03 | 暗色主题 | 背景 #1e293b，工具栏 #334155，强调色 #3b82f6 |
| F-06-04 | 关于弹窗 | 版本 2.0.0、作者akchth、功能列表 |

## 3. 数据模型

### Project
```rust
struct Project {
    id: String,          // UUID v4
    name: String,        // 项目名称
    path: String,        // 文件系统路径
    created_at: DateTime, // ISO 8601
}
```

### SettingsTemplate
```rust
struct SettingsTemplate {
    id: String,          // UUID v4
    name: String,        // 模板名称
    content: Value,      // serde_json::Value
    created_at: DateTime,
}
```

### BackupStatus
```rust
struct BackupStatus {
    settings_exists: bool,
    settings_disabled: bool,
    backup_exists: bool,
}
```

## 4. 存储规范

- 项目数据: `~/.claude-launcher/projects.json` — `{id: project_dict}`
- 模板数据: `~/.claude-launcher/templates.json` — `{default_template_id, templates: {id: template_dict}}`
- 全局配置: `~/.claude/settings.json`
- 备份: `~/.claude/settings.json.bak`
- 失效: `~/.claude/settings.json.disabled`
- 项目配置: `<project>/.claude/settings.local.json`

## 5. 可注入配置项

| Key | Label | Path | Value/Mode |
|-----|-------|------|------------|
| agent_teams | 启用 Agent Teams | env.CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS | "1" / set |
| thinking_mode | 启用思考模式 | alwaysThinkingEnabled | true / set |
| skip_dangerous | 跳过危险操作确认 | skipDangerousModePermissionPrompt | true / set |
| mcp_web_search | MCP Web 搜索权限 | permissions.allow[] | append_list |
| mcp_web_reader | MCP Web 读取器权限 | permissions.allow[] | append_list |
| npm_permissions | NPM 权限 | permissions.allow[] | append_list_multi |
| pip_permissions | PIP 权限 | permissions.allow[] | append_list_multi |
