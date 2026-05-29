# Claude Code Launcher - 设计基线

> 版本: 2.2.1 | 日期: 2026-05-29

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
| 测试 | 内置 #[test] + tempfile | 单元+集成测试 |
| Windows | windows | Win32 API 访问（UAC 等） |
| 资源 | winres | Windows 可执行文件资源编译 |

## 2. 模块架构

```
src/
├── main.rs              # 入口：初始化 App，启动 GUI，管理员权限检测
├── lib.rs               # 库入口
├── admin.rs             # 管理员权限检测和 UAC 提升
├── core/
│   ├── mod.rs
│   ├── models.rs        # Project, ProjectGroup, SettingsTemplate, BackupStatus
│   ├── project_manager.rs  # 项目 CRUD + JSON 持久化
│   ├── group_manager.rs    # 分组 CRUD + JSON 持久化
│   ├── templates_manager.rs # 模板 CRUD + JSON 持久化 + 应用到项目
│   ├── backup_manager.rs    # 备份/失效/恢复 settings.json
│   └── config_injector.rs   # 配置项快捷注入
├── gui/
│   ├── mod.rs
│   ├── theme.rs         # 颜色常量、主题配置
│   ├── main_view.rs     # 主窗口：工具栏 + 分组页签 + 项目列表
│   ├── project_card.rs  # 项目卡片组件
│   ├── widgets.rs       # 通用组件封装
│   └── dialogs/
│       ├── mod.rs
│       ├── add_project.rs    # 添加项目（含分组选择）
│       ├── edit_project.rs   # 编辑项目
│       ├── settings.rs       # 全局配置管理 + 模板管理
│       └── project_config.rs # 项目配置编辑器
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

    // 分组操作
    GroupSelected(Option<String>), // group_id, None=全部
    AddGroup,
    EditGroup(String),
    DeleteGroup(String),
    ConfirmDeleteGroup(String),
    GroupNameChanged(String),
    SaveGroup,
    CancelGroup,

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

    // 搜索
    SearchQueryChanged(String),

    // 窗口
    WindowResized(f32),

    // 字体
    FontLoaded,
}
```

## 4. 数据流

### 4.1 项目加载流程
```
App::new()
  → ProjectManager::default_manager()
  → mgr.list_projects()
  → app.project_list = projects
```

### 4.2 分组加载流程
```
App::new()
  → GroupManager::default_manager()
  → mgr.list_groups()
  → app.group_list = groups
```

### 4.3 项目分组筛选流程
```
main_view::view(app)
  → project_list_view(app)
  → 按 selected_group_id 筛选项目
  → 渲染分组视图或普通视图
```

### 4.4 管理员权限检测流程
```
main()
  → admin::is_admin()
  → 如果非管理员:
    → admin::request_elevation()
    → 成功: 退出当前进程，新进程以管理员启动
    → 失败: 显示对话框提示，退出
```

## 5. UI 布局

### 5.1 主界面布局
```
┌─────────────────────────────────────────────────────────────┐
│ [管理员] Claude Code 启动器 (N)          [关于] [+添加] [+临时] [设置] │  ← 工具栏
├─────────────────────────────────────────────────────────────┤
│ [搜索项目名称或路径...]                                        │  ← 搜索栏
├─────────────────────────────────────────────────────────────┤
│ [全部] [分组1] [分组2] [+]                                    │  ← 分组页签
├─────────────────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ ● 项目名称                                    [启动]    │ │
│ │   /path/to/project                          [编辑]    │ │
│ │   □ 跳过权限  □ 继续会话                    [配置]    │ │
│ │                                           [打开目录] │ │
│ │                                           [删除]    │ │
│ └─────────────────────────────────────────────────────────┘ │
│ ─────────────────────────────────────────────────────────── │  ← 分隔线
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ ● 另一个项目                                  [启动]    │ │
│ │   ...                                                   │ │
│ └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 添加项目对话框
```
┌─────────────────────────────────────┐
│ 添加项目                            │
│                                     │
│ 项目名称：                          │
│ [我的项目                         ] │
│                                     │
│ 项目目录：                          │
│ [选择目录...                      ] [浏览] │
│                                     │
│ 项目分组：                          │
│ [（不分组）              ▼]         │
│                                     │
│ 配置模板：                          │
│ [（不应用模板）          ▼]         │
│                                     │
│            [取消]  [添加]           │
└─────────────────────────────────────┘
```

### 5.3 全局配置管理对话框
```
┌─────────────────────────────────────────────────────────────┐
│ 设置管理                                              [X]   │
├─────────────────────────────────────────────────────────────┤
│ [全局配置管理] [模板管理]                                     │
├─────────────────────────────────────────────────────────────┤
│ 配置文件状态：正常 | 备份：有                                 │
│ [覆盖备份] [失效] [恢复] [删除配置]                           │
│                                                             │
│ 配置项注入                                                   │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ □ 启用 Agent Teams                                      │ │
│ │ □ 启用思考模式                                           │ │
│ │ □ 跳过危险操作确认                                       │ │
│ │ □ MCP Web 搜索权限                                      │ │
│ │ □ MCP Web 读取器权限                                    │ │
│ │ □ NPM 权限                                              │ │
│ │ □ PIP 权限                                              │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                                             │
│ 应用模板：                                                   │
│ [（不应用模板）          ▼]                                  │
│                                                             │
│ ~/.claude/settings.json                                     │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ {                                                       │ │
│ │   "env": {...},                                         │ │
│ │   "permissions": {...}                                  │ │
│ │ }                                                       │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                                             │
│                                            [保存配置]        │
└─────────────────────────────────────────────────────────────┘
```

## 6. 错误处理

### 6.1 错误类型
- `ProjectError` - 项目管理错误
- `GroupError` - 分组管理错误
- `TemplateError` - 模板管理错误
- `BackupError` - 备份管理错误
- `LauncherError` - 终端启动错误

### 6.2 错误展示
- 对话框内错误：显示在对话框底部
- 操作结果：显示在主界面底部状态栏
- 日志：使用 tracing 记录详细错误

## 7. 测试策略

### 7.1 单元测试
- 每个核心模块对应 `#[cfg(test)] mod tests`
- 使用 tempfile 隔离测试数据
- 测试 CRUD 操作、边界条件、错误处理

### 7.2 集成测试
- `tests/integration_test.rs` - 核心功能测试
- `tests/real_world_test.rs` - 真实数据测试
- `tests/debug_projects.rs` - 问题诊断测试

### 7.3 测试覆盖
- 项目管理：100%
- 分组管理：100%
- 模板管理：100%
- 备份管理：100%
- 配置注入：100%

## 8. 构建和发布

### 8.1 构建命令
```bash
cargo build --release
```

### 8.2 输出
- `target/release/claude-launcher.exe` (约 15MB)
- 包含 Windows 图标和版本信息

### 8.3 依赖
- Windows 10/11
- Rust 1.94+
- Windows SDK (用于链接)
- Visual Studio Build Tools (用于 MSVC 链接器)

## 9. 变更记录

### v2.2.1 (2026-05-29)
- 修复模板创建后列表不刷新的问题（关闭并重新打开设置对话框）
- 修复模板保存后列表不刷新的问题（同上）
- 分组编辑/删除按钮从页签栏移至分组视图底部（更宽敞、更清晰）
- 修复对话框遮罩层点击穿透问题（遮罩按钮填充全屏并设为不透明）
- 新增启动配置校验：检查配置目录兼容性，不兼容时弹窗归档旧配置
- 重构设置对话框初始化为 `open_settings_dialog()` 方法
- 添加 `refresh_settings_template_lists()` 统一刷新模板列表
- 修复中文输入法：修补 iced_winit 启用 IME 支持
- 修复中文路径乱码：PowerShell 输出编码改为 UTF-8

### v2.2.0 (2026-05-28)
- 项目分组功能（最多10个分组）
- 全局配置管理（合并备份管理）
- 管理员权限检测和 UAC 提升
- Windows 可执行文件图标和版本信息
