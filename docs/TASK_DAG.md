# 任务 DAG - Rust 重写执行计划

## 阶段 0: 项目初始化 (已完成)
- [x] cargo init
- [x] docs/REQUIREMENTS.md
- [x] docs/ARCHITECTURE.md

## 阶段 1: 基础设施 (Agent 1 - 串行前置)

### T1.1 Cargo.toml + 目录结构
- 依赖: iced, serde, serde_json, uuid, chrono, dirs, thiserror, tracing

### T1.2 Core Models (src/core/models.rs)
- Project, SettingsTemplate, BackupStatus 数据结构
- Serialize/Deserialize, to_dict/from_dict

### T1.3 Project Manager (src/core/project_manager.rs)
- CRUD + JSON 持久化 + 路径去重 + 旧格式迁移

### T1.4 Templates Manager (src/core/templates_manager.rs)
- CRUD + 默认模板 + 应用到项目 + 全局合并 + 旧格式迁移

### T1.5 Backup Manager (src/core/backup_manager.rs)
- 备份/失效/恢复/状态查询/读写 settings.json

### T1.6 Config Injector (src/core/config_injector.rs)
- 7 个预设配置项 + 注入/移除/检测

### T1.7 单元测试 (tests/)
- test_models, test_project_manager, test_templates_manager
- test_backup_manager, test_config_injector

## 阶段 2: GUI 框架 + 主界面 (Agent 2 - 依赖 T1)

### T2.1 主题系统 (src/gui/theme.rs)
- 暗色主题常量

### T2.2 主窗口 (src/gui/main_view.rs)
- 工具栏 + 可滚动项目列表 + 空状态

### T2.3 项目卡片 (src/gui/project_card.rs)
- 信息展示 + 启动/编辑/配置/打开目录/删除按钮 + 复选框

## 阶段 3: 对话框 + 启动器 + 集成 (Agent 3 - 依赖 T1)

### T3.1 终端启动器 (src/launcher/terminal_launcher.rs)
- WT/PS/CMD 检测 + 启动 + 临时脚本

### T3.2 添加项目对话框 (src/gui/dialogs/add_project.rs)
### T3.3 编辑项目对话框 (src/gui/dialogs/edit_project.rs)
### T3.4 设置管理对话框 (src/gui/dialogs/settings.rs)
### T3.5 项目配置对话框 (src/gui/dialogs/project_config.rs)

## 阶段 4: 集成 + E2E 测试 (串行收尾)
- main.rs 集成
- 端到端测试
- cargo clippy + cargo fmt
- Webhook 通知
