# Claude Code Launcher

> Windows 桌面应用，用于管理和一键启动 Claude Code 项目。

![Rust](https://img.shields.io/badge/Rust-2024_edition-orange)
![iced](https://img.shields.io/badge/iced-0.13-blue)
![Platform](https://img.shields.io/badge/Platform-Windows-blueviolet)
![License](https://img.shields.io/badge/License-MIT-green)

## 功能特性

- **项目管理** — 添加、编辑、删除 Claude Code 项目，卡片式展示
- **一键启动** — 自动检测 Windows Terminal，支持 CMD/PowerShell 回退
- **终端选项** — 跳过权限确认 (`--dangerously-skip-permissions`)、继续会话 (`--continue`)
- **配置模板** — 创建/编辑/管理配置模板，快速应用到项目
- **全局配置管理** — 备份、失效、恢复 `~/.claude/settings.json`
- **配置项注入** — 快捷注入 Agent Teams、思考模式、MCP 权限等预设
- **临时项目** — 一键创建临时项目于 `%TEMP%/claude-projects/`
- **自适应布局** — 根据窗口宽度自动切换 1-3 列卡片布局
- **暗色主题** — 精心设计的深色界面

## 截图

<!-- 在此添加截图
![主界面](screenshots/main.png)
![设置对话框](screenshots/settings.png)
-->

## 编译安装

### 前置要求

- [Rust](https://rustup.rs/) 1.94+ (2024 edition)
- Windows 10/11

### 从源码编译

```bash
git clone https://github.com/Akchth/claude-launcher.git
cd claude-launcher
cargo build --release
```

编译产物位于 `target/release/claude-launcher.exe`。

### 运行测试

```bash
cargo test
```

## 使用方法

1. 运行 `claude-launcher.exe`
2. 点击「+ 添加项目」添加 Claude Code 项目目录
3. 在项目卡片上点击「启动」即可在终端中打开 Claude Code

## 技术栈

| 组件 | 选型 |
|------|------|
| 语言 | Rust (2024 edition) |
| GUI 框架 | [iced](https://iced.rs/) 0.13 |
| 序列化 | serde + serde_json |
| UUID | uuid v4 |
| 时间处理 | chrono |
| 路径定位 | dirs |
| 错误处理 | thiserror |
| 日志 | tracing + tracing-subscriber |

## 项目结构

```
src/
├── main.rs                    # 入口：窗口初始化、字体加载
├── lib.rs                     # 库入口
├── core/                      # 业务逻辑（无 GUI 依赖）
│   ├── models.rs              # 数据模型
│   ├── project_manager.rs     # 项目 CRUD + 持久化
│   ├── templates_manager.rs   # 配置模板管理
│   ├── backup_manager.rs      # 全局配置备份/恢复
│   └── config_injector.rs     # 配置项快捷注入
├── gui/                       # GUI 层
│   ├── app.rs                 # 应用主逻辑 + 消息处理
│   ├── theme.rs               # 暗色主题定义
│   ├── main_view.rs           # 主界面布局（自适应多列）
│   ├── project_card.rs        # 项目卡片组件
│   └── dialogs/               # 对话框
│       ├── add_project.rs     # 添加项目
│       ├── edit_project.rs    # 编辑项目
│       ├── settings.rs        # 设置管理（3 Tab）
│       └── project_config.rs  # 项目配置编辑器
└── launcher/
    └── terminal_launcher.rs   # 终端检测 + 启动逻辑
```

## 数据存储

所有数据存储在 `~/.claude-launcher/` 目录下：

- `projects.json` — 项目列表
- `templates.json` — 配置模板
- 全局配置操作针对 `~/.claude/settings.json`

## 许可证

[MIT License](LICENSE)

## 作者

[Akchth](https://github.com/Akchth)
