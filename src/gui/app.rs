/// Claude Code 启动器主应用。
///
/// 管理 GUI 状态、消息路由、对话框系统。
use std::collections::HashMap;
use std::path::Path;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, Task};

use crate::core::models::{BackupStatus, Project};
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
}

// ── 应用结构体 ────────────────────────────────────────────────────────

pub struct App {
    pub project_list: Vec<Project>,
    pub skip_permissions: HashMap<String, bool>,
    pub continue_session: HashMap<String, bool>,
    pub dialog: Option<Dialog>,
    /// 操作提示信息（显示在主界面底部，下次操作自动清除）
    pub status_message: Option<String>,
    /// 窗口宽度（用于自适应多列布局）
    pub window_width: f32,
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
            skip_permissions: HashMap::new(),
            continue_session: HashMap::new(),
            dialog: None,
            status_message: None,
            window_width: 1000.0,
        };
        if let Ok(mgr) = crate::core::project_manager::ProjectManager::default_manager()
            && let Ok(projects) = mgr.list_projects()
        {
            app.project_list = projects;
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
                self.dialog = Some(Dialog::AddProject(add_project::State::new(templates)));
                Task::none()
            }
            Message::EditProject(id) => {
                if let Some(p) = self.project_list.iter().find(|p| p.id == id) {
                    let templates = self.get_template_names_with_default("（不修改）");
                    let detected_template = self.detect_project_template(&p.path);
                    let selected = detected_template
                        .as_ref()
                        .and_then(|name| templates.iter().find(|t| *t == name).cloned());
                    let mut state = edit_project::State::new(
                        p.id.clone(),
                        p.name.clone(),
                        p.path.clone(),
                        templates,
                    );
                    state.selected_template = selected.or_else(|| Some("（不修改）".to_string()));
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
                let _ = std::fs::create_dir_all(&temp_base);
                let temp_path = temp_base.join(&name);
                let _ = std::fs::create_dir_all(&temp_path);
                let path = temp_path.to_string_lossy().to_string();
                if let Ok(mgr) = crate::core::project_manager::ProjectManager::default_manager()
                    && let Ok(project) = mgr.add_project(name.clone(), path.clone())
                {
                    if let Ok(tm) =
                        crate::core::templates_manager::TemplatesManager::default_manager()
                    {
                        let _ = tm.apply_to_project(&path, None);
                    }
                    self.project_list.insert(0, project);
                    self.status_message = Some(format!("临时项目「{}」已创建：{}", name, path));
                    tracing::info!("临时项目已创建：{} -> {}", name, path);
                } else {
                    self.status_message = Some("临时项目创建失败，请检查日志".to_string());
                    tracing::error!("临时项目创建失败");
                }
                Task::none()
            }
            Message::OpenDirectory(id) => {
                if let Some(p) = self.project_list.iter().find(|p| p.id == id) {
                    let path = std::path::Path::new(&p.path);
                    if path.exists() {
                        let _ = std::process::Command::new("explorer.exe")
                            .arg(&p.path)
                            .spawn();
                    } else {
                        tracing::error!("项目目录不存在: {}", p.path);
                    }
                }
                Task::none()
            }
            Message::OpenSettings => {
                let bs = crate::core::backup_manager::BackupManager::default_manager()
                    .map(|m| m.get_status())
                    .unwrap_or(BackupStatus {
                        settings_exists: false,
                        settings_disabled: false,
                        backup_exists: false,
                    });
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
                    active_tab: settings::Tab::Backup,
                    backup_status: bs,
                    backup_message: None,
                    template_names: tn,
                    default_template_name: default_tn,
                    editing_template_id: None,
                    editing_template_name: String::new(),
                    editing_template_content: iced::widget::text_editor::Content::new(),
                    show_new_template_input: false,
                    new_template_name: String::new(),
                    global_config_content: global_content,
                    global_template_options: gto,
                    selected_global_template: None,
                    injector_active: ia,
                }));
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
        }
    }
}

// ── 对话框消息处理 ────────────────────────────────────────────────────

impl App {
    fn handle_add_project(&mut self, msg: add_project::Message) -> Task<Message> {
        match msg {
            add_project::Message::Save => {
                let (name, path, sel_template) = if let Some(Dialog::AddProject(s)) = &self.dialog {
                    (
                        s.name.trim().to_string(),
                        s.path.trim().to_string(),
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
                        s.error = Some(format!("目录不存在：{}", path));
                    }
                    return Task::none();
                }
                if let Ok(mgr) = crate::core::project_manager::ProjectManager::default_manager() {
                    match mgr.add_project(name, path.clone()) {
                        Ok(project) => {
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
                let (id, name, path, sel) = if let Some(Dialog::EditProject(s)) = &self.dialog {
                    (
                        s.project_id.clone(),
                        s.name.trim().to_string(),
                        s.path.trim().to_string(),
                        s.selected_template.clone(),
                    )
                } else {
                    return Task::none();
                };
                if !Path::new(&path).exists() {
                    if let Some(Dialog::EditProject(s)) = &mut self.dialog {
                        s.error = Some(format!("目录不存在：{}", path));
                    }
                    return Task::none();
                }
                if let Ok(mgr) = crate::core::project_manager::ProjectManager::default_manager() {
                    match mgr.update_project(&id, Some(name), Some(path.clone())) {
                        Ok(updated) => {
                            if let Some(ref tn) = sel
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
                        Err(e) => format!("备份失败：{}", e),
                    };
                    if let Some(Dialog::Settings(s)) = &mut self.dialog {
                        s.backup_message = Some(m);
                        s.backup_status = mgr.get_status();
                    }
                }
                Task::none()
            }
            settings::Message::DisableClicked => {
                self.dialog = Some(Dialog::ConfirmDisable);
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
                if !name.is_empty()
                    && let Ok(mgr) =
                        crate::core::templates_manager::TemplatesManager::default_manager()
                    && mgr.create_template(name, serde_json::json!({})).is_ok()
                {
                    let names = self.get_template_names_list();
                    if let Some(Dialog::Settings(s)) = &mut self.dialog {
                        s.template_names = names;
                        s.show_new_template_input = false;
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
                        s.editing_template_content.text().to_string(),
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
                            if let Some(Dialog::Settings(s)) = &mut self.dialog {
                                s.editing_template_id = None;
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
                                    s.error = Some(format!("保存失败：{}", e));
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
                    let js = s.config_content.text().to_string();
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
            let m = match mgr.disable() {
                Ok(_) => "settings.json 已失效".to_string(),
                Err(e) => format!("失效失败：{}", e),
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
                Err(e) => format!("恢复失败：{}", e),
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
        if deleted {
            let names = self.get_template_names_list();
            let default_tn = crate::core::templates_manager::TemplatesManager::default_manager()
                .ok()
                .and_then(|m| m.get_default_template().ok())
                .map(|t| t.name);
            if let Some(Dialog::Settings(s)) = &mut self.dialog {
                s.template_names = names;
                s.default_template_name = default_tn;
            }
        }
    }
}

/// 返回订阅（窗口大小变化事件）
impl App {
    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::window::resize_events().map(|(_id, size)| {
            Message::WindowResized(size.width)
        })
    }
}

// ── 对话框视图 ────────────────────────────────────────────────────────

fn about_dialog() -> Element<'static, Message> {
    let version = env!("CARGO_PKG_VERSION");
    let content = column![
        text("关于").size(20),
        text(format!(
            "Claude Code 启动器\n\n版本：{}\n\n作者：akchth\n\n版权声明：\nMIT License\n\n功能特性：\n• 管理多个 Claude Code 项目\n• 一键启动 Claude Code\n• 自定义项目配置\n• 配置模板管理\n• 可选配置一键注入\n• 全局设置备份与恢复\n• 支持临时项目创建",
            version
        ))
        .size(14),
        button(text("关闭").size(13)).on_press(Message::DialogDismissed).style(theme::accent_btn_style()),
    ].spacing(15).padding(20);
    container(content)
        .width(Length::Fixed(420.0))
        .style(theme::overlay_container())
        .into()
}

fn confirm_delete_dialog<'a>(id: &'a str, name: &'a str) -> Element<'a, Message> {
    let content = column![
        text("确认删除").size(18),
        text(format!(
            "确定要删除项目「{}」吗？\n这只会从启动器中移除项目，\n不会删除实际文件。",
            name
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
        text("确定要使 settings.json 失效吗？\n这将备份当前设置并清空配置文件。").size(14),
        row![
            button(text("取消").size(13))
                .on_press(Message::DialogDismissed)
                .style(theme::toolbar_btn_style()),
            button(text("确认失效").size(13))
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
        text(format!("确定要删除模板「{}」吗？", template_name)).size(14),
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
        button(text(""))
            .on_press(Message::DialogDismissed)
            .style(|_, _| iced::widget::button::Style {
                background: Some(iced::Color::new(0.0, 0.0, 0.0, 0.6).into()),
                text_color: iced::Color::TRANSPARENT,
                border: iced::Border::default(),
                shadow: iced::Shadow::default(),
            }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::background_container());
    iced::widget::stack![main, mask, overlay_layer].into()
}

// ── 异步辅助 ──────────────────────────────────────────────────────────

async fn browse_folder() -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = (|| -> Option<String> {
            let output = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command",
                    "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.FolderBrowserDialog; $f.Description = '选择项目目录'; if ($f.ShowDialog() -eq 'OK') { $f.SelectedPath } else { '' }"])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW — 防止闪现控制台窗口
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
