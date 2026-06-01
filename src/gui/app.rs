/// Claude Code 启动器主应用。
///
/// 管理 GUI 状态、消息路由、对话框系统。
use std::collections::HashMap;
use std::path::Path;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, Task};

use crate::core::models::{BackupStatus, Project, ProjectGroup};
use crate::gui::dialogs::{add_project, edit_project, project_config, settings};
use crate::gui::{main_view, theme};
use crate::launcher::terminal_launcher;

// ── 消息枚举 ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    /// 字体加载完成（仅用于内部初始化，无用户可见效果）
    FontLoaded,
    AddProject,
    EditProject(String),
    DeleteProject(String),
    ConfirmDelete(String),
    CancelDelete,
    LaunchProject(String),
    TempProject,
    OpenDirectory(String),
    OpenSettings,
    OpenProjectConfig(String),
    About,
    ToggleSkipPermissions(String, bool),
    ToggleContinueSession(String, bool),
    DialogDismissed,
    AddProjectMsg(add_project::Message),
    EditProjectMsg(edit_project::Message),
    SettingsMsg(settings::Message),
    ProjectConfigMsg(project_config::Message),
    BrowseAddProject(Option<String>),
    BrowseEditProject(Option<String>),
    ConfirmDisable,
    ConfirmRestore,
    ConfirmBackup,
    ConfirmDeleteTemplate(String),
    Error(String),
    WindowResized(f32),
    SearchQueryChanged(String),
    // 分组相关消息
    GroupSelected(Option<String>), // None 表示全部
    AddGroup,
    EditGroup(String),
    DeleteGroup(String),
    ConfirmDeleteGroup(String),
    GroupNameChanged(String),
    SaveGroup,
    CancelGroup,
}

// ── 对话框状态 ────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum Dialog {
    AddProject(add_project::State),
    EditProject(edit_project::State),
    Settings(settings::State),
    ProjectConfig(project_config::State),
    About,
    ConfirmDelete {
        project_id: String,
        project_name: String,
    },
    ConfirmDisable,
    ConfirmRestore,
    ConfirmBackup,
    ConfirmDeleteTemplate {
        template_name: String,
    },
    // 分组相关对话框
    AddGroup {
        name: String,
    },
    EditGroup {
        group_id: String,
        name: String,
    },
    ConfirmDeleteGroup {
        group_id: String,
        group_name: String,
    },
}

// ── 应用结构体 ────────────────────────────────────────────────────────

pub struct App {
    pub project_list: Vec<Project>,
    pub group_list: Vec<ProjectGroup>,
    pub selected_group_id: Option<String>, // None 表示全部
    pub skip_permissions: HashMap<String, bool>,
    pub continue_session: HashMap<String, bool>,
    pub dialog: Option<Dialog>,
    /// 操作提示信息（显示在主界面底部，下次操作自动清除）
    pub status_message: Option<String>,
    /// 窗口宽度（用于自适应多列布局）
    pub window_width: f32,
    /// 搜索过滤关键词
    pub search_query: String,
    /// 新建/编辑分组时的名称
    pub group_name_input: String,
    /// 是否以管理员身份运行
    pub is_admin: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let mut app = App {
            project_list: Vec::new(),
            group_list: Vec::new(),
            selected_group_id: None,
            skip_permissions: HashMap::new(),
            continue_session: HashMap::new(),
            dialog: None,
            status_message: None,
            window_width: 1000.0,
            search_query: String::new(),
            group_name_input: String::new(),
            is_admin: crate::admin::is_admin(),
        };
        if let Ok(mgr) = crate::core::project_manager::ProjectManager::default_manager()
            && let Ok(projects) = mgr.list_projects()
        {
            app.project_list = projects;
        }
        if let Ok(mgr) = crate::core::group_manager::GroupManager::default_manager()
            && let Ok(groups) = mgr.list_groups()
        {
            app.group_list = groups;
        }
        app
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        // 任何新操作自动清除状态提示
        self.status_message = None;
        match message {
            Message::FontLoaded => Task::none(),
            Message::AddProject => {
                let templates = self.get_template_names_with_default("（不应用模板）");
                let groups = self.get_group_names_with_default("（不分组）");
                self.dialog = Some(Dialog::AddProject(add_project::State::new(templates, groups)));
                Task::none()
            }
            Message::EditProject(id) => {
                if let Some(p) = self.project_list.iter().find(|p| p.id == id) {
                    let templates = self.get_template_names_with_default("（不修改）");
                    let groups = self.get_group_names_with_default("（不分组）");
                    let detected_template = self.detect_project_template(&p.path);
                    let selected_template = detected_template
                        .as_ref()
                        .and_then(|name| templates.iter().find(|t| *t == name).cloned());
                    let selected_group = if let Some(ref gid) = p.group_id {
                        self.group_list
                            .iter()
                            .find(|g| &g.id == gid)
                            .map(|g| g.name.clone())
                    } else {
                        Some("（不分组）".to_string())
                    };
                    let mut state = edit_project::State::new(
                        p.id.clone(),
                        p.name.clone(),
                        p.path.clone(),
                        templates,
                        groups,
                        selected_group,
                    );
                    state.selected_template = selected_template.or_else(|| Some("（不修改）".to_string()));
                    self.dialog = Some(Dialog::EditProject(state));
                }
                Task::none()
            }
            Message::DeleteProject(id) => {
                if let Some(p) = self.project_list.iter().find(|p| p.id == id) {
                    self.dialog = Some(Dialog::ConfirmDelete {
                        project_id: id,
                        project_name: p.name.clone(),
                    });
                }
                Task::none()
            }
            Message::ConfirmDelete(id) => {
                self.skip_permissions.remove(&id);
                self.continue_session.remove(&id);
                self.project_list.retain(|p| p.id != id);
                if let Ok(mgr) = crate::core::project_manager::ProjectManager::default_manager() {
                    let _ = mgr.delete_project(&id);
                }
                self.dialog = None;
                Task::none()
            }
            Message::CancelDelete => {
                self.dialog = None;
                Task::none()
            }
            Message::LaunchProject(id) => {
                if let Some(p) = self.project_list.iter().find(|p| p.id == id) {
                    let skip = self.skip_permissions.get(&id).copied().unwrap_or(false);
                    let cont = self.continue_session.get(&id).copied().unwrap_or(false);
                    if let Err(e) =
                        terminal_launcher::launch_claude_code(&p.path, &p.name, skip, cont, "auto")
                    {
                        tracing::error!("启动失败: {}", e);
                    }
                }
                Task::none()
            }
            Message::TempProject => {
                let now = chrono::Local::now();
                let name = format!("temp_{}", now.format("%Y%m%d_%H%M%S"));
                let temp_base = std::env::temp_dir().join("claude-projects");
                if let Err(e) = std::fs::create_dir_all(&temp_base) {
                    self.status_message = Some(format!("创建临时目录失败：{e}"));
                    tracing::error!("创建临时目录失败：{e}");
                    return Task::none();
                }
                let temp_path = temp_base.join(&name);
                if let Err(e) = std::fs::create_dir_all(&temp_path) {
                    self.status_message = Some(format!("创建项目目录失败：{e}"));
                    tracing::error!("创建项目目录失败：{e}");
                    return Task::none();
                }
                let path = temp_path.to_string_lossy().to_string();
                match crate::core::project_manager::ProjectManager::default_manager() {
                    Ok(mgr) => match mgr.add_project(name.clone(), path.clone()) {
                        Ok(project) => {
                            if let Ok(tm) =
                                crate::core::templates_manager::TemplatesManager::default_manager()
                            {
                                let _ = tm.apply_to_project(&path, None);
                            }
                            self.project_list.insert(0, project);
                            self.status_message =
                                Some(format!("临时项目「{name}」已创建：{path}"));
                            tracing::info!("临时项目已创建：{name} -> {path}");
                        }
                        Err(e) => {
                            self.status_message = Some(format!("添加项目失败：{e}"));
                            tracing::error!("添加项目失败：{e}");
                        }
                    },
                    Err(e) => {
                        self.status_message = Some(format!("初始化项目管理器失败：{e}"));
                        tracing::error!("初始化项目管理器失败：{e}");
                    }
                }
                Task::none()
            }
            Message::OpenDirectory(id) => {
                if let Some(p) = self.project_list.iter().find(|p| p.id == id) {
                    let path = std::path::Path::new(&p.path);
                    if path.exists() {
                        #[cfg(target_os = "windows")]
                        {
                            let _ = std::process::Command::new("explorer.exe")
                                .arg(&p.path)
                                .spawn();
                        }
                        #[cfg(target_os = "macos")]
                        {
                            let _ = std::process::Command::new("open")
                                .arg(&p.path)
                                .spawn();
                        }
                        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
                        {
                            let _ = std::process::Command::new("xdg-open")
                                .arg(&p.path)
                                .spawn();
                        }
                    } else {
                        self.status_message =
                            Some(format!("项目目录不存在：{}", p.path));
                        tracing::error!("项目目录不存在: {}", p.path);
                    }
                }
                Task::none()
            }
            Message::OpenSettings => {
                self.open_settings_dialog();
                Task::none()
            }
            Message::OpenProjectConfig(id) => {
                if let Some(p) = self.project_list.iter().find(|p| p.id == id) {
                    let sp = Path::new(&p.path)
                        .join(".claude")
                        .join("settings.local.json");
                    let cj = if sp.exists() {
                        std::fs::read_to_string(&sp).unwrap_or_else(|_| "{}".to_string())
                    } else {
                        "{}".to_string()
                    };
                    let ia = serde_json::from_str::<serde_json::Value>(&cj)
                        .ok()
                        .map(|v| crate::core::config_injector::detect_active_items(&v))
                        .unwrap_or_default();
                    let mut to = vec!["（不应用模板）".to_string()];
                    to.extend(self.get_template_names_list());
                    self.dialog = Some(Dialog::ProjectConfig(project_config::State {
                        project_id: p.id.clone(),
                        project_name: p.name.clone(),
                        project_path: p.path.clone(),
                        config_content: iced::widget::text_editor::Content::with_text(&cj),
                        injector_active: ia,
                        template_options: to,
                        selected_template: None,
                        show_save_as_template: false,
                        save_as_template_name: String::new(),
                        error: None,
                    }));
                }
                Task::none()
            }
            Message::About => {
                self.dialog = Some(Dialog::About);
                Task::none()
            }
            Message::ToggleSkipPermissions(id, v) => {
                self.skip_permissions.insert(id, v);
                Task::none()
            }
            Message::ToggleContinueSession(id, v) => {
                self.continue_session.insert(id, v);
                Task::none()
            }
            Message::DialogDismissed => {
                self.dialog = None;
                Task::none()
            }
            Message::AddProjectMsg(msg) => self.handle_add_project(msg),
            Message::EditProjectMsg(msg) => self.handle_edit_project(msg),
            Message::SettingsMsg(msg) => self.handle_settings(msg),
            Message::ProjectConfigMsg(msg) => self.handle_project_config(msg),
            Message::BrowseAddProject(path) => {
                if let Some(Dialog::AddProject(s)) = &mut self.dialog
                    && let Some(p) = path
                {
                    if s.name.is_empty() {
                        s.name = Path::new(&p)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                    }
                    s.path = p;
                }
                Task::none()
            }
            Message::BrowseEditProject(path) => {
                if let Some(Dialog::EditProject(s)) = &mut self.dialog
                    && let Some(p) = path
                {
                    s.path = p;
                }
                Task::none()
            }
            Message::Error(msg) => {
                tracing::error!("{}", msg);
                Task::none()
            }
            Message::WindowResized(width) => {
                self.window_width = width;
                Task::none()
            }
            Message::SearchQueryChanged(query) => {
                self.search_query = query;
                Task::none()
            }
            // ── 分组相关消息 ──────────────────────────────────────────
            Message::GroupSelected(group_id) => {
                self.selected_group_id = group_id;
                Task::none()
            }
            Message::AddGroup => {
                self.group_name_input = String::new();
                self.dialog = Some(Dialog::AddGroup { name: String::new() });
                Task::none()
            }
            Message::EditGroup(group_id) => {
                if let Some(group) = self.group_list.iter().find(|g| g.id == group_id) {
                    self.group_name_input = group.name.clone();
                    self.dialog = Some(Dialog::EditGroup {
                        group_id: group_id.clone(),
                        name: group.name.clone(),
                    });
                }
                Task::none()
            }
            Message::DeleteGroup(group_id) => {
                if let Some(group) = self.group_list.iter().find(|g| g.id == group_id) {
                    self.dialog = Some(Dialog::ConfirmDeleteGroup {
                        group_id: group_id.clone(),
                        group_name: group.name.clone(),
                    });
                }
                Task::none()
            }
            Message::ConfirmDeleteGroup(group_id) => {
                if let Ok(mgr) = crate::core::group_manager::GroupManager::default_manager() {
                    let _ = mgr.delete_group(&group_id);
                    self.group_list.retain(|g| g.id != group_id);
                    // 将该分组下的项目移到默认分组
                    for project in &mut self.project_list {
                        if project.group_id.as_ref() == Some(&group_id) {
                            project.group_id = None;
                        }
                    }
                    if self.selected_group_id.as_ref() == Some(&group_id) {
                        self.selected_group_id = None;
                    }
                }
                self.dialog = None;
                Task::none()
            }
            Message::GroupNameChanged(name) => {
                self.group_name_input = name.clone();
                if let Some(Dialog::AddGroup { name: n }) = &mut self.dialog {
                    *n = name;
                } else if let Some(Dialog::EditGroup { name: n, .. }) = &mut self.dialog {
                    *n = name;
                }
                Task::none()
            }
            Message::SaveGroup => {
                let name = self.group_name_input.trim().to_string();
                if name.is_empty() {
                    return Task::none();
                }

                if let Some(Dialog::AddGroup { .. }) = &self.dialog {
                    if let Ok(mgr) = crate::core::group_manager::GroupManager::default_manager() {
                        match mgr.create_group(name) {
                            Ok(group) => {
                                self.group_list.push(group);
                                self.dialog = None;
                            }
                            Err(e) => {
                                self.status_message = Some(format!("创建分组失败：{e}"));
                            }
                        }
                    }
                } else if let Some(Dialog::EditGroup { group_id, .. }) = &self.dialog {
                    let group_id = group_id.clone();
                    if let Ok(mgr) = crate::core::group_manager::GroupManager::default_manager() {
                        match mgr.update_group(&group_id, Some(name), None) {
                            Ok(updated) => {
                                if let Some(g) = self.group_list.iter_mut().find(|g| g.id == group_id) {
                                    *g = updated;
                                }
                                self.dialog = None;
                            }
                            Err(e) => {
                                self.status_message = Some(format!("更新分组失败：{e}"));
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::CancelGroup => {
                self.dialog = None;
                Task::none()
            }
            Message::ConfirmDisable => {
                self.execute_disable();
                Task::none()
            }
            Message::ConfirmRestore => {
                self.execute_restore();
                Task::none()
            }
            Message::ConfirmBackup => {
                // 执行覆盖备份
                if let Ok(mgr) = crate::core::backup_manager::BackupManager::default_manager() {
                    match mgr.backup() {
                        Ok(()) => tracing::info!("备份已覆盖"),
                        Err(e) => tracing::error!("备份失败：{}", e),
                    }
                }
                self.dialog = None;
                Task::none()
            }
            Message::ConfirmDeleteTemplate(name) => {
                self.execute_delete_template(&name);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let main = main_view::view(self);
        match &self.dialog {
            None => main,
            Some(Dialog::AddProject(s)) => {
                overlay(main, add_project::view(s).map(Message::AddProjectMsg))
            }
            Some(Dialog::EditProject(s)) => {
                overlay(main, edit_project::view(s).map(Message::EditProjectMsg))
            }
            Some(Dialog::Settings(s)) => overlay(main, settings::view(s).map(Message::SettingsMsg)),
            Some(Dialog::ProjectConfig(s)) => {
                overlay(main, project_config::view(s).map(Message::ProjectConfigMsg))
            }
            Some(Dialog::About) => overlay(main, about_dialog()),
            Some(Dialog::ConfirmDelete {
                project_id,
                project_name,
            }) => overlay(main, confirm_delete_dialog(project_id, project_name)),
            Some(Dialog::ConfirmDisable) => overlay(main, confirm_disable_dialog()),
            Some(Dialog::ConfirmRestore) => overlay(main, confirm_restore_dialog()),
            Some(Dialog::ConfirmBackup) => overlay(main, confirm_backup_dialog()),
            Some(Dialog::ConfirmDeleteTemplate { template_name }) => {
                overlay(main, confirm_delete_template_dialog(template_name))
            }
            Some(Dialog::AddGroup { name }) => {
                overlay(main, group_dialog("新建分组", name, true))
            }
            Some(Dialog::EditGroup { name, .. }) => {
                overlay(main, group_dialog("编辑分组", name, false))
            }
            Some(Dialog::ConfirmDeleteGroup {
                group_id,
                group_name,
            }) => overlay(main, confirm_delete_group_dialog(group_id, group_name)),
        }
    }
}

// ── 对话框消息处理 ────────────────────────────────────────────────────

impl App {
    fn handle_add_project(&mut self, msg: add_project::Message) -> Task<Message> {
        match msg {
            add_project::Message::Save => {
                let (name, path, sel_group, sel_template) = if let Some(Dialog::AddProject(s)) = &self.dialog {
                    (
                        s.name.trim().to_string(),
                        s.path.trim().to_string(),
                        s.selected_group.clone(),
                        s.selected_template.clone(),
                    )
                } else {
                    return Task::none();
                };
                if name.is_empty() || path.is_empty() {
                    if let Some(Dialog::AddProject(s)) = &mut self.dialog {
                        s.error = Some("请输入项目名称和目录".to_string());
                    }
                    return Task::none();
                }
                if !Path::new(&path).exists() {
                    if let Some(Dialog::AddProject(s)) = &mut self.dialog {
                        s.error = Some(format!("目录不存在：{path}"));
                    }
                    return Task::none();
                }

                // 获取分组 ID
                let group_id = if let Some(ref gn) = sel_group {
                    if gn == "（不分组）" {
                        None
                    } else {
                        self.group_list.iter().find(|g| g.name == *gn).map(|g| g.id.clone())
                    }
                } else {
                    None
                };

                if let Ok(mgr) = crate::core::project_manager::ProjectManager::default_manager() {
                    match mgr.add_project(name, path.clone()) {
                        Ok(project) => {
                            // 设置分组
                            let project = if let Some(gid) = group_id {
                                match mgr.update_project_group(&project.id, Some(gid)) {
                                    Ok(p) => p,
                                    Err(e) => {
                                        tracing::warn!("更新项目分组失败: {e}");
                                        project
                                    }
                                }
                            } else {
                                project
                            };
                            if let Some(ref tn) = sel_template
                                && tn != "（不应用模板）"
                            {
                                self.apply_template_by_name(tn, &path);
                            }
                            self.project_list.insert(0, project);
                            self.dialog = None;
                        }
                        Err(e) => {
                            if let Some(Dialog::AddProject(s)) = &mut self.dialog {
                                s.error = Some(e.to_string());
                            }
                        }
                    }
                }
                Task::none()
            }
            add_project::Message::Cancel => {
                self.dialog = None;
                Task::none()
            }
            add_project::Message::BrowseClicked => {
                Task::perform(browse_folder(), Message::BrowseAddProject)
            }
            add_project::Message::NameChanged(s) => {
                if let Some(Dialog::AddProject(st)) = &mut self.dialog {
                    st.name = s;
                }
                Task::none()
            }
            add_project::Message::GroupSelected(s) => {
                if let Some(Dialog::AddProject(st)) = &mut self.dialog {
                    st.selected_group = Some(s);
                }
                Task::none()
            }
            add_project::Message::PathChanged(s) => {
                if let Some(Dialog::AddProject(st)) = &mut self.dialog {
                    st.path = s;
                }
                Task::none()
            }
            add_project::Message::TemplateSelected(s) => {
                if let Some(Dialog::AddProject(st)) = &mut self.dialog {
                    st.selected_template = Some(s);
                }
                Task::none()
            }
        }
    }

    fn handle_edit_project(&mut self, msg: edit_project::Message) -> Task<Message> {
        match msg {
            edit_project::Message::Save => {
                let (id, name, path, sel_group, sel_template) = if let Some(Dialog::EditProject(s)) = &self.dialog {
                    (
                        s.project_id.clone(),
                        s.name.trim().to_string(),
                        s.path.trim().to_string(),
                        s.selected_group.clone(),
                        s.selected_template.clone(),
                    )
                } else {
                    return Task::none();
                };
                if !Path::new(&path).exists() {
                    if let Some(Dialog::EditProject(s)) = &mut self.dialog {
                        s.error = Some(format!("目录不存在：{path}"));
                    }
                    return Task::none();
                }
                if let Ok(mgr) = crate::core::project_manager::ProjectManager::default_manager() {
                    match mgr.update_project(&id, Some(name), Some(path.clone())) {
                        Ok(mut updated) => {
                            // 更新分组
                            let group_id = if let Some(ref gn) = sel_group {
                                if gn == "（不分组）" {
                                    None
                                } else {
                                    self.group_list.iter().find(|g| g.name == *gn).map(|g| g.id.clone())
                                }
                            } else {
                                None
                            };
                            if let Ok(p) = mgr.update_project_group(&id, group_id) {
                                updated = p;
                            }
                            if let Some(ref tn) = sel_template
                                && tn != "（不修改）"
                            {
                                self.apply_template_by_name(tn, &updated.path);
                            }
                            if let Some(pos) = self.project_list.iter().position(|p| p.id == id) {
                                self.project_list[pos] = updated;
                            }
                            self.dialog = None;
                        }
                        Err(e) => {
                            if let Some(Dialog::EditProject(s)) = &mut self.dialog {
                                s.error = Some(e.to_string());
                            }
                        }
                    }
                }
                Task::none()
            }
            edit_project::Message::Cancel => {
                self.dialog = None;
                Task::none()
            }
            edit_project::Message::BrowseClicked => {
                Task::perform(browse_folder(), Message::BrowseEditProject)
            }
            edit_project::Message::NameChanged(s) => {
                if let Some(Dialog::EditProject(st)) = &mut self.dialog {
                    st.name = s;
                }
                Task::none()
            }
            edit_project::Message::GroupSelected(s) => {
                if let Some(Dialog::EditProject(st)) = &mut self.dialog {
                    st.selected_group = Some(s);
                }
                Task::none()
            }
            edit_project::Message::PathChanged(s) => {
                if let Some(Dialog::EditProject(st)) = &mut self.dialog {
                    st.path = s;
                }
                Task::none()
            }
            edit_project::Message::TemplateSelected(s) => {
                if let Some(Dialog::EditProject(st)) = &mut self.dialog {
                    st.selected_template = Some(s);
                }
                Task::none()
            }
        }
    }

    fn handle_settings(&mut self, msg: settings::Message) -> Task<Message> {
        match msg {
            settings::Message::Close => {
                self.dialog = None;
                Task::none()
            }
            settings::Message::TabChanged(tab) => {
                if let Some(Dialog::Settings(s)) = &mut self.dialog {
                    s.active_tab = tab;
                }
                Task::none()
            }
            settings::Message::BackupClicked => {
                // 如果已有备份，先确认再覆盖（#2 备份安全）
                if let Ok(mgr) = crate::core::backup_manager::BackupManager::default_manager()
                    && mgr.get_status().backup_exists
                {
                    self.dialog = Some(Dialog::ConfirmBackup);
                    return Task::none();
                }
                // 没有备份，直接创建
                if let Ok(mgr) = crate::core::backup_manager::BackupManager::default_manager() {
                    let m = match mgr.backup() {
                        Ok(()) => "备份已创建".to_string(),
                        Err(e) => format!("备份失败：{e}"),
                    };
                    if let Some(Dialog::Settings(s)) = &mut self.dialog {
                        s.backup_message = Some(m);
                        s.backup_status = mgr.get_status();
                    }
                }
                Task::none()
            }
            settings::Message::DisableClicked => {
                // 检查是否已有备份，如果有则显示覆盖提示
                if let Ok(mgr) = crate::core::backup_manager::BackupManager::default_manager() {
                    let status = mgr.get_status();
                    if status.backup_exists {
                        // 已有备份，显示覆盖确认
                        self.dialog = Some(Dialog::ConfirmDisable);
                    } else {
                        // 没有备份，直接失效
                        self.execute_disable();
                    }
                }
                Task::none()
            }
            settings::Message::RestoreClicked => {
                self.dialog = Some(Dialog::ConfirmRestore);
                Task::none()
            }
            settings::Message::AddTemplateClicked => {
                if let Some(Dialog::Settings(s)) = &mut self.dialog {
                    s.show_new_template_input = true;
                    s.new_template_name.clear();
                }
                Task::none()
            }
            settings::Message::NewTemplateNameChanged(t) => {
                if let Some(Dialog::Settings(s)) = &mut self.dialog {
                    s.new_template_name = t;
                }
                Task::none()
            }
            settings::Message::NewTemplateConfirm => {
                let name = if let Some(Dialog::Settings(s)) = &self.dialog {
                    s.new_template_name.trim().to_string()
                } else {
                    return Task::none();
                };
                if name.is_empty() {
                    if let Some(Dialog::Settings(s)) = &mut self.dialog {
                        s.backup_message = Some("模板名称不能为空".to_string());
                    }
                    return Task::none();
                }
                match crate::core::templates_manager::TemplatesManager::default_manager() {
                    Ok(mgr) => match mgr.create_template(name.clone(), serde_json::json!({})) {
                        Ok(_) => {
                            self.dialog = None;
                            self.open_settings_dialog();
                            if let Some(Dialog::Settings(s)) = &mut self.dialog {
                                s.active_tab = settings::Tab::Templates;
                                s.backup_message = Some(format!("模板「{name}」创建成功"));
                            }
                        }
                        Err(e) => {
                            tracing::error!("模板创建失败: {}", e);
                            if let Some(Dialog::Settings(s)) = &mut self.dialog {
                                s.backup_message = Some(format!("创建失败：{e}"));
                            }
                        }
                    },
                    Err(e) => {
                        tracing::error!("TemplatesManager 初始化失败: {}", e);
                        if let Some(Dialog::Settings(s)) = &mut self.dialog {
                            s.backup_message = Some(format!("初始化失败：{e}"));
                        }
                    }
                }
                Task::none()
            }
            settings::Message::NewTemplateCancel => {
                if let Some(Dialog::Settings(s)) = &mut self.dialog {
                    s.show_new_template_input = false;
                }
                Task::none()
            }
            settings::Message::EditTemplateClicked(name) => {
                if let Ok(mgr) = crate::core::templates_manager::TemplatesManager::default_manager()
                    && let Ok(ts) = mgr.list_templates()
                    && let Some(t) = ts.iter().find(|t| t.name == name)
                {
                    let json = serde_json::to_string_pretty(&t.content).unwrap_or_default();
                    if let Some(Dialog::Settings(s)) = &mut self.dialog {
                        s.editing_template_id = Some(t.id.clone());
                        s.editing_template_name = t.name.clone();
                        s.editing_template_content =
                            iced::widget::text_editor::Content::with_text(&json);
                    }
                }
                Task::none()
            }
            settings::Message::DeleteTemplateClicked(name) => {
                self.dialog = Some(Dialog::ConfirmDeleteTemplate {
                    template_name: name,
                });
                Task::none()
            }
            settings::Message::SetDefaultClicked(name) => {
                if let Ok(mgr) = crate::core::templates_manager::TemplatesManager::default_manager()
                    && let Ok(ts) = mgr.list_templates()
                    && let Some(t) = ts.iter().find(|t| t.name == name)
                {
                    let _ = mgr.set_default_template(&t.id);
                    self.refresh_settings_template_lists();
                    if let Some(Dialog::Settings(s)) = &mut self.dialog {
                        s.backup_message = Some(format!("已将「{name}」设为默认模板"));
                    }
                }
                Task::none()
            }
            settings::Message::TemplateEditAction(action) => {
                if let Some(Dialog::Settings(s)) = &mut self.dialog {
                    s.editing_template_content.perform(action);
                }
                Task::none()
            }
            settings::Message::SaveTemplateClicked => {
                let (id_opt, json_str) = if let Some(Dialog::Settings(s)) = &self.dialog {
                    (
                        s.editing_template_id.clone(),
                        s.editing_template_content.text().clone(),
                    )
                } else {
                    return Task::none();
                };
                if let Some(id) = id_opt {
                    match serde_json::from_str::<serde_json::Value>(&json_str) {
                        Ok(content) => {
                            if let Ok(mgr) =
                                crate::core::templates_manager::TemplatesManager::default_manager()
                            {
                                let _ = mgr.update_template(&id, None, Some(content));
                            }
                            // 关闭并重新打开以确保列表刷新
                            self.dialog = None;
                            self.open_settings_dialog();
                            if let Some(Dialog::Settings(s)) = &mut self.dialog {
                                s.active_tab = settings::Tab::Templates;
                                s.backup_message = Some("模板已保存".to_string());
                            }
                        }
                        Err(_) => {
                            if let Some(Dialog::Settings(s)) = &mut self.dialog {
                                s.backup_message =
                                    Some("模板 JSON 格式错误，请检查内容".to_string());
                            }
                        }
                    }
                }
                Task::none()
            }
            settings::Message::GlobalTemplateSelected(name) => {
                if name == "（不应用模板）" {
                    return Task::none();
                }
                if let Ok(mgr) = crate::core::templates_manager::TemplatesManager::default_manager()
                    && let Ok(ts) = mgr.list_templates()
                    && let Some(t) = ts.iter().find(|t| t.name == name)
                {
                    let json = serde_json::to_string_pretty(&t.content).unwrap_or_default();
                    if let Some(Dialog::Settings(s)) = &mut self.dialog {
                        s.global_config_content =
                            iced::widget::text_editor::Content::with_text(&json);
                        s.selected_global_template = Some(name);
                    }
                }
                Task::none()
            }
            settings::Message::GlobalConfigAction(action) => {
                if let Some(Dialog::Settings(s)) = &mut self.dialog {
                    s.global_config_content.perform(action);
                }
                Task::none()
            }
            settings::Message::ToggleInjector(key) => {
                if let Some(Dialog::Settings(s)) = &mut self.dialog {
                    // Toggle: checked → insert true (active), unchecked → insert false (to remove)
                    if s.injector_active.get(&key) == Some(&true) {
                        s.injector_active.insert(key.clone(), false);
                    } else {
                        s.injector_active.insert(key.clone(), true);
                    }
                    if let Ok(config) =
                        serde_json::from_str::<serde_json::Value>(&s.global_config_content.text())
                    {
                        let new_config =
                            crate::core::config_injector::inject_items(&config, &s.injector_active);
                        s.global_config_content = iced::widget::text_editor::Content::with_text(
                            &serde_json::to_string_pretty(&new_config).unwrap_or_default(),
                        );
                        // Sync injector_active with what's actually in the config now
                        s.injector_active =
                            crate::core::config_injector::detect_active_items(&new_config);
                    }
                }
                Task::none()
            }
            settings::Message::SaveGlobalClicked => {
                let content = if let Some(Dialog::Settings(s)) = &self.dialog {
                    s.global_config_content.text().trim().to_string()
                } else {
                    return Task::none();
                };
                if content.is_empty() {
                    return Task::none();
                }
                if serde_json::from_str::<serde_json::Value>(&content).is_err() {
                    if let Some(Dialog::Settings(s)) = &mut self.dialog {
                        s.backup_message = Some("全局配置 JSON 格式错误，请检查内容".to_string());
                    }
                    return Task::none();
                }
                if let Ok(mgr) = crate::core::backup_manager::BackupManager::default_manager()
                    && mgr.write_settings(&content).is_ok()
                    && let Some(Dialog::Settings(s)) = &mut self.dialog
                {
                    s.backup_message = Some("全局配置已保存".to_string());
                    s.backup_status = mgr.get_status();
                }
                Task::none()
            }
            settings::Message::DeleteGlobalConfig => {
                if let Ok(mgr) = crate::core::backup_manager::BackupManager::default_manager() {
                    // 先备份再删除
                    if mgr.get_status().settings_exists {
                        let _ = mgr.backup();
                        // 清空配置文件
                        let _ = mgr.write_settings("{}");
                        if let Some(Dialog::Settings(s)) = &mut self.dialog {
                            s.backup_message = Some("全局配置已删除（已备份）".to_string());
                            s.backup_status = mgr.get_status();
                            s.global_config_content = iced::widget::text_editor::Content::with_text("{}");
                        }
                    }
                }
                Task::none()
            }
        }
    }

    fn handle_project_config(&mut self, msg: project_config::Message) -> Task<Message> {
        match msg {
            project_config::Message::Close => {
                self.dialog = None;
                Task::none()
            }
            project_config::Message::ConfigAction(action) => {
                if let Some(Dialog::ProjectConfig(s)) = &mut self.dialog {
                    s.config_content.perform(action);
                }
                Task::none()
            }
            project_config::Message::ToggleInjector(key) => {
                if let Some(Dialog::ProjectConfig(s)) = &mut self.dialog {
                    // Toggle: checked → insert true (active), unchecked → insert false (to remove)
                    if s.injector_active.get(&key) == Some(&true) {
                        s.injector_active.insert(key.clone(), false);
                    } else {
                        s.injector_active.insert(key.clone(), true);
                    }
                    if let Ok(config) =
                        serde_json::from_str::<serde_json::Value>(&s.config_content.text())
                    {
                        let new_config =
                            crate::core::config_injector::inject_items(&config, &s.injector_active);
                        s.config_content = iced::widget::text_editor::Content::with_text(
                            &serde_json::to_string_pretty(&new_config).unwrap_or_default(),
                        );
                        // Sync injector_active with what's actually in the config now
                        s.injector_active =
                            crate::core::config_injector::detect_active_items(&new_config);
                    }
                }
                Task::none()
            }
            project_config::Message::TemplateSelected(name) => {
                if name == "（不应用模板）" {
                    return Task::none();
                }
                if let Ok(mgr) = crate::core::templates_manager::TemplatesManager::default_manager()
                    && let Ok(ts) = mgr.list_templates()
                    && let Some(t) = ts.iter().find(|t| t.name == name)
                {
                    let json = serde_json::to_string_pretty(&t.content).unwrap_or_default();
                    let active = crate::core::config_injector::detect_active_items(&t.content);
                    if let Some(Dialog::ProjectConfig(s)) = &mut self.dialog {
                        s.config_content = iced::widget::text_editor::Content::with_text(&json);
                        s.injector_active = active;
                        s.selected_template = Some(name);
                    }
                }
                Task::none()
            }
            project_config::Message::Save => {
                if let Some(Dialog::ProjectConfig(s)) = &self.dialog {
                    let content = s.config_content.text().trim().to_string();
                    if serde_json::from_str::<serde_json::Value>(&content).is_ok() {
                        let sp = Path::new(&s.project_path)
                            .join(".claude")
                            .join("settings.local.json");
                        if let Some(parent) = sp.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        match std::fs::write(&sp, &content) {
                            Ok(()) => self.dialog = None,
                            Err(e) => {
                                if let Some(Dialog::ProjectConfig(s)) = &mut self.dialog {
                                    s.error = Some(format!("保存失败：{e}"));
                                }
                            }
                        }
                    } else if let Some(Dialog::ProjectConfig(s)) = &mut self.dialog {
                        s.error = Some("JSON 格式错误".to_string());
                    }
                }
                Task::none()
            }
            project_config::Message::SaveAsTemplateClicked => {
                if let Some(Dialog::ProjectConfig(s)) = &mut self.dialog {
                    s.show_save_as_template = true;
                    s.save_as_template_name.clear();
                }
                Task::none()
            }
            project_config::Message::SaveAsTemplateNameChanged(n) => {
                if let Some(Dialog::ProjectConfig(s)) = &mut self.dialog {
                    s.save_as_template_name = n;
                }
                Task::none()
            }
            project_config::Message::SaveAsTemplateConfirm => {
                if let Some(Dialog::ProjectConfig(s)) = &self.dialog {
                    let name = s.save_as_template_name.trim().to_string();
                    let js = s.config_content.text().clone();
                    if !name.is_empty()
                        && let Ok(content) = serde_json::from_str::<serde_json::Value>(&js)
                        && let Ok(mgr) =
                            crate::core::templates_manager::TemplatesManager::default_manager()
                        && mgr.create_template(name, content).is_ok()
                        && let Some(Dialog::ProjectConfig(s)) = &mut self.dialog
                    {
                        s.show_save_as_template = false;
                    }
                }
                Task::none()
            }
            project_config::Message::SaveAsTemplateCancel => {
                if let Some(Dialog::ProjectConfig(s)) = &mut self.dialog {
                    s.show_save_as_template = false;
                }
                Task::none()
            }
            project_config::Message::Reset => {
                if let Ok(mgr) = crate::core::templates_manager::TemplatesManager::default_manager()
                {
                    let dc = mgr
                        .get_default_template()
                        .map(|t| t.content)
                        .unwrap_or(serde_json::json!({}));
                    let active = crate::core::config_injector::detect_active_items(&dc);
                    let json = serde_json::to_string_pretty(&dc).unwrap_or_default();
                    if let Some(Dialog::ProjectConfig(s)) = &mut self.dialog {
                        s.config_content = iced::widget::text_editor::Content::with_text(&json);
                        s.injector_active = active;
                    }
                }
                Task::none()
            }
            project_config::Message::Cancel => {
                self.dialog = None;
                Task::none()
            }
        }
    }

    // ── 辅助方法 ──────────────────────────────────────────────────────

    fn get_template_names_list(&self) -> Vec<String> {
        crate::core::templates_manager::TemplatesManager::default_manager()
            .ok()
            .and_then(|m| m.list_templates().ok())
            .map(|ts| ts.into_iter().map(|t| t.name).collect())
            .unwrap_or_default()
    }

    fn get_template_names_with_default(&self, default: &str) -> Vec<String> {
        let mut opts = vec![default.to_string()];
        opts.extend(self.get_template_names_list());
        opts
    }

    /// 刷新设置对话框中的模板列表和全局模板选项
    fn refresh_settings_template_lists(&mut self) {
        let names = self.get_template_names_list();
        let default_tn = crate::core::templates_manager::TemplatesManager::default_manager()
            .ok()
            .and_then(|m| m.get_default_template().ok())
            .map(|t| t.name);
        let mut gto = vec!["（不应用模板）".to_string()];
        gto.extend(names.clone());
        if let Some(Dialog::Settings(s)) = &mut self.dialog {
            s.template_names = names;
            s.default_template_name = default_tn;
            s.global_template_options = gto;
        }
    }

    fn get_group_names_list(&self) -> Vec<String> {
        self.group_list.iter().map(|g| g.name.clone()).collect()
    }

    fn get_group_names_with_default(&self, default: &str) -> Vec<String> {
        let mut opts = vec![default.to_string()];
        opts.extend(self.get_group_names_list());
        opts
    }

    fn apply_template_by_name(&self, name: &str, project_path: &str) {
        if let Ok(mgr) = crate::core::templates_manager::TemplatesManager::default_manager()
            && let Ok(ts) = mgr.list_templates()
            && let Some(t) = ts.iter().find(|t| t.name == name)
        {
            let _ = mgr.apply_to_project(project_path, Some(&t.id));
        }
    }

    fn detect_project_template(&self, project_path: &str) -> Option<String> {
        let settings_path = Path::new(project_path)
            .join(".claude")
            .join("settings.local.json");
        let content = std::fs::read_to_string(&settings_path).ok()?;
        let config: serde_json::Value = serde_json::from_str(&content).ok()?;
        if let Ok(mgr) = crate::core::templates_manager::TemplatesManager::default_manager()
            && let Ok(ts) = mgr.list_templates()
        {
            for t in &ts {
                if t.content == config {
                    return Some(t.name.clone());
                }
            }
        }
        None
    }

    fn execute_disable(&mut self) {
        if let Ok(mgr) = crate::core::backup_manager::BackupManager::default_manager() {
            // 检查是否已有备份
            let status = mgr.get_status();
            if status.backup_exists {
                // 已有备份，先覆盖备份再失效
                if let Err(e) = mgr.backup() {
                    tracing::warn!("覆盖备份失败: {e}");
                }
            }

            let m = match mgr.disable() {
                Ok(_) => "settings.json 已失效".to_string(),
                Err(e) => format!("失效失败：{e}"),
            };
            let status = mgr.get_status();
            if let Some(Dialog::Settings(s)) = &mut self.dialog {
                s.backup_message = Some(m);
                s.backup_status = status;
            }
        }
        self.dialog = None;
    }

    fn execute_restore(&mut self) {
        if let Ok(mgr) = crate::core::backup_manager::BackupManager::default_manager() {
            let m = match mgr.restore() {
                Ok(_) => "settings.json 已恢复".to_string(),
                Err(e) => format!("恢复失败：{e}"),
            };
            let status = mgr.get_status();
            if let Some(Dialog::Settings(s)) = &mut self.dialog {
                s.backup_message = Some(m);
                s.backup_status = status;
            }
        }
        self.dialog = None;
    }

    fn execute_delete_template(&mut self, name: &str) {
        let deleted = {
            let mut found = false;
            if let Ok(mgr) = crate::core::templates_manager::TemplatesManager::default_manager()
                && let Ok(ts) = mgr.list_templates()
                && let Some(t) = ts.iter().find(|t| t.name == name)
            {
                let _ = mgr.delete_template(&t.id);
                found = true;
            }
            found
        };
        // 关闭确认对话框
        self.dialog = None;
        if deleted {
            // 重新打开设置对话框（模板管理 Tab）
            self.open_settings_dialog();
            if let Some(Dialog::Settings(s)) = &mut self.dialog {
                s.active_tab = settings::Tab::Templates;
                s.backup_message = Some("模板已删除".to_string());
            }
        }
    }

    /// 打开设置对话框并初始化所有状态
    fn open_settings_dialog(&mut self) {
        let bs = crate::core::backup_manager::BackupManager::default_manager()
            .map_or(BackupStatus {
                settings_exists: false,
                settings_disabled: false,
                backup_exists: false,
            }, |m| m.get_status());
        let tn = self.get_template_names_list();
        let default_tn =
            crate::core::templates_manager::TemplatesManager::default_manager()
                .ok()
                .and_then(|m| m.get_default_template().ok())
                .map(|t| t.name);
        let gc = crate::core::backup_manager::BackupManager::default_manager()
            .ok()
            .and_then(|m| m.read_settings())
            .unwrap_or_else(|| "{}".to_string());
        let ia = serde_json::from_str::<serde_json::Value>(&gc)
            .ok()
            .map(|v| crate::core::config_injector::detect_active_items(&v))
            .unwrap_or_default();
        let mut gto = vec!["（不应用模板）".to_string()];
        gto.extend(tn.clone());
        let global_content = iced::widget::text_editor::Content::with_text(&gc);
        self.dialog = Some(Dialog::Settings(settings::State {
            active_tab: settings::Tab::Global,
            backup_status: bs,
            backup_message: None,
            global_config_content: global_content,
            global_template_options: gto,
            selected_global_template: None,
            injector_active: ia,
            template_names: tn,
            default_template_name: default_tn,
            editing_template_id: None,
            editing_template_name: String::new(),
            editing_template_content: iced::widget::text_editor::Content::new(),
            show_new_template_input: false,
            new_template_name: String::new(),
        }));
    }
}

/// 返回订阅（窗口大小变化事件 + 键盘快捷键）
impl App {
    pub fn subscription(&self) -> iced::Subscription<Message> {
        let resize = iced::window::resize_events().map(|(_id, size)| {
            Message::WindowResized(size.width)
        });

        let keyboard = iced::keyboard::on_key_press(|key, _modifiers| {
            match key {
                iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                    Some(Message::DialogDismissed)
                }
                _ => None,
            }
        });

        iced::Subscription::batch(vec![resize, keyboard])
    }
}

// ── 对话框视图 ────────────────────────────────────────────────────────

fn about_dialog() -> Element<'static, Message> {
    let version = env!("CARGO_PKG_VERSION");
    let content = column![
        text("关于 Claude Code 启动器").size(20),
        text(format!("版本：{version}")).size(14),
        text("作者：akchth").size(14),
        text("版权声明：MIT License").size(14),
        text("").size(8),
        text("功能特性：").size(14),
        text("• 管理多个 Claude Code 项目").size(13),
        text("• 一键启动 Claude Code").size(13),
        text("• 自定义项目配置").size(13),
        text("• 配置模板管理").size(13),
        text("• 可选配置一键注入").size(13),
        text("• 全局配置管理（读取/修改/保存/应用模板/删除/失效/恢复）").size(13),
        text("• 支持临时项目创建").size(13),
        text("• 项目分组管理（最多10个分组）").size(13),
        text("• 管理员权限自动提升").size(13),
        text("").size(8),
        text("更新日志：").size(14),
        text("v2.2.1 - 编辑项目支持分组修改、模板创建修复").size(13),
        text("v2.2.0 - 项目分组、全局配置管理、管理员权限检测").size(13),
        text("v2.1.0 - 日期格式兼容、模板迁移修复").size(13),
        text("v2.0.0 - Rust 重写版本").size(13),
        button(text("关闭").size(13)).on_press(Message::DialogDismissed).style(theme::accent_btn_style()),
    ].spacing(5).padding(20);
    container(content)
        .width(Length::Fixed(450.0))
        .style(theme::overlay_container())
        .into()
}

fn confirm_delete_dialog<'a>(id: &'a str, name: &'a str) -> Element<'a, Message> {
    let content = column![
        text("确认删除").size(18),
        text(format!(
            "确定要删除项目「{name}」吗？\n这只会从启动器中移除项目，\n不会删除实际文件。"
        ))
        .size(14),
        row![
            button(text("取消").size(13))
                .on_press(Message::CancelDelete)
                .style(theme::toolbar_btn_style()),
            button(text("确认删除").size(13))
                .on_press(Message::ConfirmDelete(id.to_string()))
                .style(theme::danger_btn_style()),
        ]
        .spacing(10),
    ]
    .spacing(15)
    .padding(20);
    container(content)
        .width(Length::Fixed(420.0))
        .style(theme::overlay_container())
        .into()
}

fn confirm_disable_dialog() -> Element<'static, Message> {
    let content = column![
        text("确认失效").size(18),
        text("已有备份文件存在，失效将覆盖上一个备份。\n确定要使 settings.json 失效吗？").size(14),
        row![
            button(text("取消").size(13))
                .on_press(Message::DialogDismissed)
                .style(theme::toolbar_btn_style()),
            button(text("确认失效并覆盖备份").size(13))
                .on_press(Message::ConfirmDisable)
                .style(theme::danger_btn_style()),
        ]
        .spacing(10),
    ]
    .spacing(15)
    .padding(20);
    container(content)
        .width(Length::Fixed(420.0))
        .style(theme::overlay_container())
        .into()
}

fn confirm_restore_dialog() -> Element<'static, Message> {
    let content = column![
        text("确认恢复").size(18),
        text("确定要从备份恢复 settings.json 吗？\n这将覆盖当前配置。").size(14),
        row![
            button(text("取消").size(13))
                .on_press(Message::DialogDismissed)
                .style(theme::toolbar_btn_style()),
            button(text("确认恢复").size(13))
                .on_press(Message::ConfirmRestore)
                .style(theme::accent_btn_style()),
        ]
        .spacing(10),
    ]
    .spacing(15)
    .padding(20);
    container(content)
        .width(Length::Fixed(420.0))
        .style(theme::overlay_container())
        .into()
}

fn confirm_backup_dialog() -> Element<'static, Message> {
    let content = column![
        text("覆盖备份").size(18),
        text("已有备份文件存在，确定要覆盖吗？\n当前备份将被替换。").size(14),
        row![
            button(text("取消").size(13))
                .on_press(Message::DialogDismissed)
                .style(theme::toolbar_btn_style()),
            button(text("覆盖备份").size(13))
                .on_press(Message::ConfirmBackup)
                .style(theme::danger_btn_style()),
        ]
        .spacing(10),
    ]
    .spacing(15)
    .padding(20);
    container(content)
        .width(Length::Fixed(420.0))
        .style(theme::overlay_container())
        .into()
}

fn confirm_delete_template_dialog(template_name: &str) -> Element<'_, Message> {
    let content = column![
        text("确认删除模板").size(18),
        text(format!("确定要删除模板「{template_name}」吗？")).size(14),
        row![
            button(text("取消").size(13))
                .on_press(Message::DialogDismissed)
                .style(theme::toolbar_btn_style()),
            button(text("确认删除").size(13))
                .on_press(Message::ConfirmDeleteTemplate(template_name.to_string()))
                .style(theme::danger_btn_style()),
        ]
        .spacing(10),
    ]
    .spacing(15)
    .padding(20);
    container(content)
        .width(Length::Fixed(420.0))
        .style(theme::overlay_container())
        .into()
}

/// 分组对话框（新建/编辑）
fn group_dialog<'a>(title: &'a str, name: &'a str, is_new: bool) -> Element<'a, Message> {
    let name_input = iced::widget::text_input("分组名称", name)
        .on_input(Message::GroupNameChanged)
        .padding(8);

    let save_btn_text = if is_new { "创建" } else { "保存" };
    let save_btn = button(text(save_btn_text).size(13))
        .on_press(Message::SaveGroup)
        .style(theme::accent_btn_style());
    let cancel_btn = button(text("取消").size(13))
        .on_press(Message::CancelGroup)
        .style(theme::toolbar_btn_style());

    let content = column![
        text(title).size(18),
        name_input,
        row![cancel_btn, save_btn].spacing(10),
    ]
    .spacing(15)
    .padding(20);

    container(content)
        .width(Length::Fixed(400.0))
        .style(theme::overlay_container())
        .into()
}

/// 确认删除分组对话框
fn confirm_delete_group_dialog<'a>(group_id: &'a str, group_name: &'a str) -> Element<'a, Message> {
    let content = column![
        text("确认删除分组").size(18),
        text(format!("确定要删除分组「{group_name}」吗？\n该分组下的项目将变为未分组状态。")).size(14),
        row![
            button(text("取消").size(13))
                .on_press(Message::DialogDismissed)
                .style(theme::toolbar_btn_style()),
            button(text("确认删除").size(13))
                .on_press(Message::ConfirmDeleteGroup(group_id.to_string()))
                .style(theme::danger_btn_style()),
        ]
        .spacing(10),
    ]
    .spacing(15)
    .padding(20);
    container(content)
        .width(Length::Fixed(420.0))
        .style(theme::overlay_container())
        .into()
}

fn overlay<'a>(main: Element<'a, Message>, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let dialog_box = container(dialog)
        .width(Length::Shrink)
        .height(Length::Shrink);
    let overlay_layer = container(dialog_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(theme::overlay_container());
    // 半透明遮罩层，阻止背景交互，点击遮罩关闭对话框
    let mask = container(
        button(text(" ").size(1))
            .on_press(Message::DialogDismissed)
            .style(|_, _| iced::widget::button::Style {
                background: Some(iced::Color::new(0.0, 0.0, 0.0, 0.6).into()),
                text_color: iced::Color::TRANSPARENT,
                border: iced::Border::default(),
                shadow: iced::Shadow::default(),
            })
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::background_container());
    iced::widget::stack![main, mask, overlay_layer].into()
}

// ── 异步辅助 ──────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
async fn browse_folder() -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = (|| -> Option<String> {
            // 设置 PowerShell 输出编码为 UTF-8，解决中文路径乱码问题
            let ps_script = "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
                Add-Type -AssemblyName System.Windows.Forms; \
                $f = New-Object System.Windows.Forms.FolderBrowserDialog; \
                $f.Description = '选择项目目录'; \
                if ($f.ShowDialog() -eq 'OK') { $f.SelectedPath } else { '' }";
            let output = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", ps_script])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output().ok()?;
            if !output.status.success() {
                return None;
            }
            // 尝试 UTF-8 解码，失败则使用系统默认编码
            let path = String::from_utf8(output.stdout.clone())
                .unwrap_or_else(|_| String::from_utf8_lossy(&output.stdout).to_string())
                .trim()
                .to_string();
            if path.is_empty() { None } else { Some(path) }
        })();
        let _ = tx.send(result);
    });
    let result = rx.recv().ok().flatten();
    if result.is_none() {
        tracing::warn!("浏览文件夹对话框失败或用户取消");
    }
    result
}

#[cfg(target_os = "macos")]
async fn browse_folder() -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = (|| -> Option<String> {
            let output = std::process::Command::new("osascript")
                .args([
                    "-e",
                    "set folderPath to POSIX path of (choose folder with prompt \"选择项目目录\")",
                ])
                .output().ok()?;
            if !output.status.success() {
                return None;
            }
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if path.is_empty() { None } else { Some(path) }
        })();
        let _ = tx.send(result);
    });
    let result = rx.recv().ok().flatten();
    if result.is_none() {
        tracing::warn!("浏览文件夹对话框失败或用户取消");
    }
    result
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
async fn browse_folder() -> Option<String> {
    tracing::warn!("当前平台不支持文件夹浏览对话框");
    None
}
