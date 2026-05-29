# Claude Code Launcher - 需求基线

> 版本: 2.2.1 | 技术栈: Rust + iced | 日期: 2026-05-29

## 1. 项目概述

Claude Code 启动器是一个 Windows 桌面 GUI 应用，用于管理多个 Claude Code 项目，
提供一键启动、配置模板管理、全局设置管理、项目分组等功能。

## 2. 功能需求

### F-01 项目管理 (P0)

| ID | 功能 | 描述 |
|----|------|------|
| F-01-01 | 添加项目 | 输入名称+选择目录+选择分组，自动生成 UUID，路径去重，可选配置模板 |
| F-01-02 | 编辑项目 | 修改名称/路径/分组，路径去重校验，可选重新应用配置模板 |
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
| F-05-01 | 添加项目对话框 | 名称输入、路径浏览、分组选择、模板选择 |
| F-05-02 | 编辑项目对话框 | 修改名称/路径、分组选择、模板选择 |
| F-05-03 | 设置管理对话框 | 2 Tab: 全局配置管理、模板管理 |
| F-05-04 | 项目配置对话框 | JSON 编辑器、配置项注入、模板应用、保存为模板 |
| F-05-05 | 模态遮罩 | 对话框打开时半透明遮罩阻止背景交互，点击遮罩关闭对话框 |

### F-06 主界面 (P0)

| ID | 功能 | 描述 |
|----|------|------|
| F-06-01 | 工具栏 | 标题 + 管理员状态 + 设置/临时/添加/关于 按钮 |
| F-06-02 | 可滚动列表 | CTkScrollableFrame 等效 |
| F-06-03 | 暗色主题 | 背景 #1e293b，工具栏 #334155，强调色 #3b82f6 |
| F-06-04 | 关于弹窗 | 版本 2.2.1、作者akchth、功能列表、更新日志 |
| F-06-05 | 搜索过滤 | 按项目名称或路径过滤 |

### F-07 项目分组 (P1)

| ID | 功能 | 描述 |
|----|------|------|
| F-07-01 | 分组 CRUD | 创建/编辑/删除分组，上限 10 个 |
| F-07-02 | 分组页签 | 主界面显示分组页签，支持"全部"视图 |
| F-07-03 | 分组筛选 | 点击页签筛选该分组下的项目 |
| F-07-04 | 分组分隔 | "全部"视图中，不同分组用分隔线分开显示 |
| F-07-05 | 添加时选分组 | 添加项目对话框可选择分组 |
| F-07-06 | 编辑时改分组 | 编辑项目对话框可修改项目所属分组 |
| F-07-07 | 分组操作按钮 | 选中分组时，分组视图底部显示编辑/删除按钮 |

### F-08 全局配置管理 (P1)

| ID | 功能 | 描述 |
|----|------|------|
| F-08-01 | 读取配置 | 读取 `~/.claude/settings.json` |
| F-08-02 | 修改配置 | JSON 编辑器修改配置 |
| F-08-03 | 保存配置 | 保存配置到文件 |
| F-08-04 | 应用模板 | 将模板内容应用到全局配置 |
| F-08-05 | 删除配置 | 删除配置文件（带备份） |
| F-08-06 | 失效配置 | 将配置文件重命名为 `.disabled` |
| F-08-07 | 恢复配置 | 从备份恢复配置文件 |
| F-08-08 | 连续失效 | 失效时自动覆盖备份 |

### F-09 管理员权限 (P1)

| ID | 功能 | 描述 |
|----|------|------|
| F-09-01 | 权限检测 | 启动时检测是否管理员身份 |
| F-09-02 | UAC 提升 | 非管理员时申请管理员权限 |
| F-09-03 | 状态显示 | 工具栏显示管理员/普通用户状态 |
| F-09-04 | 拒绝处理 | 用户拒绝提升时显示提示并退出 |

## 3. 数据模型

### Project
```rust
struct Project {
    id: String,          // UUID v4
    name: String,        // 项目名称
    path: String,        // 文件系统路径
    group_id: Option<String>, // 所属分组 ID
    created_at: DateTime, // ISO 8601
}
```

### ProjectGroup
```rust
struct ProjectGroup {
    id: String,          // UUID v4
    name: String,        // 分组名称
    order: i32,          // 排序顺序
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
- 分组数据: `~/.claude-launcher/groups.json` — `{id: group_dict}`
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

## 6. 中文/国际化支持

| ID | 功能 | 描述 |
|----|------|------|
| F-10-01 | 中文输入法 | 修补 iced_winit 启用 IME 支持，支持中文/日文/韩文输入法 |
| F-10-02 | 中文路径 | PowerShell 文件对话框输出 UTF-8 编码，正确处理中文路径 |
| F-10-03 | 中文字体 | 启动时加载 Windows 系统中文字体（Microsoft YaHei UI） |

## 7. 配置兼容性与迁移 (P0)

| ID | 功能 | 描述 |
|----|------|------|
| F-11-01 | 启动校验 | 启动时检查配置目录是否为空，非空则逐个校验配置文件格式 |
| F-11-02 | 格式校验 | 校验 projects.json、groups.json、templates.json 是否符合当前版本格式 |
| F-11-03 | 归档确认 | 格式不兼容时弹窗提醒用户，确认后按时间戳归档旧配置目录 |
| F-11-04 | 重新初始化 | 归档后在干净的配置目录上初始化新版本配置 |
| F-11-05 | 跨版本迁移 | **TODO**: 支持从旧版本自动迁移配置数据到新版本格式（暂不实现） |

### 配置格式校验规则

| 文件 | 当前版本格式 | 旧版本格式（需归档） |
|------|-------------|---------------------|
| projects.json | `{id: {id, name, path, created_at}}` | `{"projects": [...]}` |
| groups.json | `{id: {id, name, order, created_at}}` | 任意非标准结构 |
| templates.json | `{default_template_id, templates: {id: {...}}}` | `{name, content}` 或缺少必要字段 |
