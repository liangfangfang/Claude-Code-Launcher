/// 设置管理对话框 - 2 Tab: 全局配置管理、模板管理。
use std::collections::HashMap;

use iced::widget::{button, column, container, pick_list, row, text, text_editor, text_input};
use iced::{Element, Length};

use crate::core::models::BackupStatus;
use crate::gui::theme;

/// 设置对话框状态
#[derive(Debug)]
pub struct State {
    pub active_tab: Tab,

    // 全局配置 Tab（合并了备份管理）
    pub backup_status: BackupStatus,
    pub backup_message: Option<String>,
    pub global_config_content: text_editor::Content,
    pub global_template_options: Vec<String>,
    pub selected_global_template: Option<String>,
    pub injector_active: HashMap<String, bool>,

    // 模板 Tab
    pub template_names: Vec<String>,
    pub default_template_name: Option<String>,
    pub editing_template_id: Option<String>,
    pub editing_template_name: String,
    pub editing_template_content: text_editor::Content,
    pub show_new_template_input: bool,
    pub new_template_name: String,
}

impl State {
    /// 从全局配置 JSON 字符串创建 State
    pub fn with_global_config(global_config_json: &str, other: impl FnOnce(&mut Self)) -> Self {
        let mut state = Self {
            active_tab: Tab::Global,
            backup_status: BackupStatus {
                settings_exists: false,
                settings_disabled: false,
                backup_exists: false,
            },
            backup_message: None,
            global_config_content: text_editor::Content::with_text(global_config_json),
            global_template_options: Vec::new(),
            selected_global_template: None,
            injector_active: HashMap::new(),
            template_names: Vec::new(),
            default_template_name: None,
            editing_template_id: None,
            editing_template_name: String::new(),
            editing_template_content: text_editor::Content::new(),
            show_new_template_input: false,
            new_template_name: String::new(),
        };
        other(&mut state);
        state
    }
}

/// 设置对话框标签页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Global,
    Templates,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Tab 切换
    TabChanged(Tab),
    Close,

    // 全局配置管理（包含备份管理）
    BackupClicked,
    DisableClicked,
    RestoreClicked,
    GlobalTemplateSelected(String),
    GlobalConfigAction(text_editor::Action),
    ToggleInjector(String),
    SaveGlobalClicked,
    DeleteGlobalConfig,

    // 模板管理
    AddTemplateClicked,
    NewTemplateNameChanged(String),
    NewTemplateConfirm,
    NewTemplateCancel,
    EditTemplateClicked(String),
    DeleteTemplateClicked(String),
    SetDefaultClicked(String),
    TemplateEditAction(text_editor::Action),
    SaveTemplateClicked,
}

pub fn view(state: &State) -> Element<'_, Message> {
    // 标题行 + 关闭按钮
    let title_row = row![
        text("设置管理").size(18),
        row![].width(Length::Fill),
        button(text("X").size(16))
            .on_press(Message::Close)
            .style(theme::toolbar_btn_style()),
    ];

    let tab_btns = row![
        tab_button("全局配置管理", Tab::Global, state.active_tab),
        tab_button("模板管理", Tab::Templates, state.active_tab),
    ]
    .spacing(4);

    let tab_content = match state.active_tab {
        Tab::Global => global_config_tab(state),
        Tab::Templates => templates_tab(state),
    };

    let layout = column![title_row, tab_btns, tab_content]
        .spacing(10)
        .padding(15)
        .width(Length::Fill);

    container(layout)
        .width(Length::Fixed(700.0))
        .height(Length::Fixed(650.0))
        .style(theme::overlay_container())
        .into()
}

fn tab_button(label: &str, tab: Tab, active: Tab) -> iced::widget::Button<'_, Message> {
    button(text(label).size(13))
        .on_press(Message::TabChanged(tab))
        .style(move |t, status| {
            if tab == active {
                theme::accent_button_style(t, status)
            } else {
                theme::toolbar_button_style(t, status)
            }
        })
}

// ── 模板管理 Tab ─────────────────────────────────────────────────────

fn templates_tab(state: &State) -> Element<'_, Message> {
    let header = text("模板列表").size(14);
    let add_btn = button(text("新增模板").size(13))
        .on_press(Message::AddTemplateClicked)
        .style(theme::accent_btn_style());

    let mut template_rows = column![header].spacing(6);
    for name in &state.template_names {
        let display_name = if state.default_template_name.as_deref() == Some(name) {
            format!("{name} [默认]")
        } else {
            name.clone()
        };
        let name_text = text(display_name).size(13);
        let edit_btn = button(text("编辑").size(11))
            .on_press(Message::EditTemplateClicked(name.clone()))
            .style(theme::toolbar_btn_style());
        let del_btn = button(text("删除").size(11))
            .on_press(Message::DeleteTemplateClicked(name.clone()))
            .style(theme::danger_btn_style());
        let set_btn = button(text("设为默认").size(11))
            .on_press(Message::SetDefaultClicked(name.clone()))
            .style(theme::toolbar_btn_style());

        let r = row![name_text, edit_btn, set_btn, del_btn]
            .spacing(6)
            .width(Length::Fill);
        template_rows = template_rows.push(r);
    }

    let mut col = column![template_rows, add_btn].spacing(10);

    // 模板操作提示/错误信息
    if let Some(msg) = &state.backup_message {
        col = col.push(text(msg).size(12).color(theme::TEXT_GRAY));
    }

    // 新增模板输入
    if state.show_new_template_input {
        let input = text_input("模板名称", &state.new_template_name)
            .on_input(Message::NewTemplateNameChanged)
            .padding(6);
        let confirm = button(text("确定").size(12))
            .on_press(Message::NewTemplateConfirm)
            .style(theme::accent_btn_style());
        let cancel = button(text("取消").size(12))
            .on_press(Message::NewTemplateCancel)
            .style(theme::toolbar_btn_style());
        col = col.push(row![input, confirm, cancel].spacing(8));
    }

    // 编辑区域 - 使用 text_editor 支持多行 JSON
    if state.editing_template_id.is_some() {
        let edit_label = text(format!("编辑模板：{}", state.editing_template_name)).size(13);
        let json_editor = text_editor(&state.editing_template_content)
            .on_action(Message::TemplateEditAction)
            .height(Length::Fixed(200.0))
            .padding(8);
        let save_btn = button(text("保存模板").size(12))
            .on_press(Message::SaveTemplateClicked)
            .style(theme::accent_btn_style());
        col = col.push(column![edit_label, json_editor, save_btn].spacing(8));
    }

    col.into()
}

// ── 全局配置管理 Tab（合并了备份管理）─────────────────────────────────────

fn global_config_tab(state: &State) -> Element<'_, Message> {
    // 状态信息
    let status_text = format!(
        "配置文件状态：{} | 备份：{}",
        state.backup_status.status_text(),
        if state.backup_status.backup_exists { "有" } else { "无" }
    );
    let status = text(status_text).size(13);

    // 备份管理按钮行
    let backup_btn = button(
        text(if state.backup_status.backup_exists {
            "覆盖备份"
        } else {
            "备份"
        })
        .size(13),
    )
    .on_press(Message::BackupClicked)
    .style(theme::accent_btn_style());

    let disable_btn = if state.backup_status.settings_exists {
        button(text("失效").size(13))
            .on_press(Message::DisableClicked)
            .style(theme::danger_btn_style())
    } else {
        button(text("失效（无配置文件）").size(13)).style(theme::toolbar_btn_style())
    };

    let restore_btn = if state.backup_status.backup_exists {
        button(text("恢复").size(13))
            .on_press(Message::RestoreClicked)
            .style(theme::success_btn_style())
    } else {
        button(text("恢复（无备份）").size(13)).style(theme::toolbar_btn_style())
    };

    let delete_btn = if state.backup_status.settings_exists {
        button(text("删除配置").size(13))
            .on_press(Message::DeleteGlobalConfig)
            .style(theme::danger_btn_style())
    } else {
        button(text("删除配置").size(13)).style(theme::toolbar_btn_style())
    };

    let backup_row = row![backup_btn, disable_btn, restore_btn, delete_btn].spacing(8);

    // 备份消息
    let backup_msg = state
        .backup_message
        .as_ref()
        .map(|m| text(m).size(12).color(theme::TEXT_GRAY));

    // 注入面板
    let injector_label = text("配置项注入").size(13);
    let injectable_items = crate::core::config_injector::get_all_items();
    let mut injector_checks = column![injector_label].spacing(4);

    let sorted_keys: Vec<_> = crate::core::config_injector::ordered_keys();

    for key in sorted_keys {
        let item = &injectable_items[key];
        let is_active = state.injector_active.get(key) == Some(&true);
        let key_clone = key.to_string();
        let cb = iced::widget::checkbox(&item.label, is_active)
            .on_toggle(move |_| Message::ToggleInjector(key_clone.clone()))
            .size(iced::Pixels(13.0));
        injector_checks = injector_checks.push(cb);
    }

    let injector_panel = container(injector_checks)
        .padding(10)
        .style(theme::panel_container());

    // 模板应用下拉
    let template_label = text("应用模板：").size(13);
    let template_pick = pick_list(
        state.global_template_options.as_slice(),
        state.selected_global_template.clone(),
        Message::GlobalTemplateSelected,
    );

    // JSON 多行编辑器
    let config_label = text("~/.claude/settings.json")
        .size(12)
        .color(theme::TEXT_GRAY);
    let json_editor = text_editor(&state.global_config_content)
        .on_action(Message::GlobalConfigAction)
        .height(Length::Fixed(200.0))
        .padding(8);

    // 保存按钮
    let save_btn = button(text("保存配置").size(13))
        .on_press(Message::SaveGlobalClicked)
        .style(theme::accent_btn_style());

    let mut col = column![
        status,
        backup_row,
    ]
    .spacing(10);

    if let Some(m) = backup_msg {
        col = col.push(m);
    }

    col = col.push(injector_panel);
    col = col.push(template_label);
    col = col.push(template_pick);
    col = col.push(config_label);
    col = col.push(json_editor);
    col = col.push(save_btn);

    col.spacing(10).into()
}
