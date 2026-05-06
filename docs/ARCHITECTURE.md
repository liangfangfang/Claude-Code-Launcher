# Claude Code Launcher - 架构设计文档 (Rust)

## 1. 技术栈

| 组件 | 选型 | 理由 |
|------|------|------|
| 语言 | Rust 1.94+ | 性能、内存安全、单二进制分发 |
| GUI | iced | 成熟的原生 GUI 框架，Elm 架构，跨平台 |
| JSON | serde + serde_json | Rust 生态标准 |
| UUID | uuid | v4 生成 |
| 时间 | chrono | DateTime 处理 |
| 路径 | dirs | 跨平台 home/temp 目录 |
| 日志 | tracing + tracing-subscriber | 结构化日志 |
| 测试 | 内置 #[test] + assert_cmd | 单元+集成测试 |

## 2. 模块架构

```
src/
├── main.rs              # 入口：初始化 App，启动 GUI
├── app.rs               # 顶层 iced Application，路由消息
├── core/
│   ├── mod.rs
│   ├── models.rs        # Project, SettingsTemplate, BackupStatus
│   ├── project_manager.rs  # 项目 CRUD + JSON 持久化
│   ├── templates_manager.rs # 模板 CRUD + JSON 持久化 + 应用到项目
│   ├── backup_manager.rs    # 备份/失效/恢复 settings.json
│   └── config_injector.rs   # 配置项快捷注入
├── gui/
│   ├── mod.rs
│   ├── theme.rs         # 颜色常量、主题配置
│   ├── main_view.rs     # 主窗口：工具栏 + 项目列表
│   ├── project_card.rs  # 项目卡片组件
│   ├── dialogs/
│   │   ├── mod.rs
│   │   ├── add_project.rs
│   │   ├── edit_project.rs
│   │   ├── settings.rs      # 3-Tab 设置对话框
│   │   └── project_config.rs # 项目配置对话框
│   └── widgets.rs       # 通用组件：按钮、标签、复选框封装
└── launcher/
    ├── mod.rs
    └── terminal_launcher.rs # 终端检测 + 启动逻辑
```

## 3. 消息架构 (iced Elm)

```rust
enum Message {
    // 项目操作
    AddProject,
    EditProject(String),       // project_id
    DeleteProject(String),     // project_id
    TempProject,
    LaunchProject(String),     // project_id
    OpenDirectory(String),     // project_id

    // 配置操作
    OpenSettings,
    OpenProjectConfig(String), // project_id
    About,

    // 内部事件
    ProjectListLoaded(Vec<Project>),
    ProjectAdded(Project),
    ProjectUpdated(Project),
    ProjectDeleted(String),
    Error(String),

    // 对话框
    DialogDismissed,
}
```

## 4. 开发约定

### 4.1 错误处理
- 核心模块使用 `thiserror` 定义错误类型
- GUI 层将错误转为用户友好消息显示
- 文件 I/O 使用 `?` 操作符，不 unwrap

### 4.2 编码规范
- 模块级文档注释 (///)
- 公共 API 必须有文档
- 函数不超过 50 行
- 错误消息使用中文

### 4.3 测试约定
- 每个核心模块对应 `tests/test_*.rs` 或 `#[cfg(test)] mod tests`
- 项目管理器测试使用 tempdir 隔离
- GUI 测试使用 iced 的 test utilities
- CI: `cargo test && cargo clippy && cargo fmt --check`

### 4.4 构建约定
- `cargo build --release` 生成优化二进制
- 目标：Windows x86_64
- 依赖最小化，避免动态链接
