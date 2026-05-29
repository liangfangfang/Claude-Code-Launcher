/// 项目配置对话框 - JSON 编辑器、配置注入、模板应用、保存为模板。
use std::collections::HashMap;

use iced::widget::{button, column, container, pick_list, row, text, text_editor, text_input};
use iced::{Element, Length};

use crate::gui::theme;

/// 项目配置对话框状态
#[derive(Debug)]
pub struct State {
    pub project_id: String,
    pub project_name: String,
    pub project_path: String,
    pub config_content: text_editor::Content,
    pub injector_active: HashMap<String, bool>,
    pub template_options: Vec<String>,
    pub selected_template: Option<String>,
    pub show_save_as_template: bool,
    pub save_as_template_name: String,
    pub error: Option<String>,
}

impl State {
    /// 从 JSON 字符串创建 State
    pub fn with_config_json(config_json: &str) -> Self {
        Self {
            project_id: String::new(),
            project_name: String::new(),
            project_path: String::new(),
            config_content: text_editor::Content::with_text(config_json),
            injector_active: HashMap::new(),
            template_options: Vec::new(),
            selected_template: None,
            show_save_as_template: false,
            save_as_template_name: String::new(),
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ConfigAction(text_editor::Action),
    ToggleInjector(String),
    TemplateSelected(String),
    Save,
    SaveAsTemplateClicked,
    SaveAsTemplateNameChanged(String),
    SaveAsTemplateConfirm,
    SaveAsTemplateCancel,
    Reset,
    Cancel,
    Close,
}

pub fn view(state: &State) -> Element<'_, Message> {
    // 标题行 + 关闭按钮
    let title_row = row![
        text(format!("编辑项目配置：{}", state.project_name)).size(16),
        row![].width(Length::Fill),
        button(text("X").size(16))
            .on_press(Message::Close)
            .style(theme::toolbar_btn_style()),
    ];

    let path_label = text(format!(
        "{}/.claude/settings.local.json",
        state.project_path
    ))
    .size(11)
    .color(theme::TEXT_GRAY);

    // 注入面板
    let injectable_items = crate::core::config_injector::get_all_items();
    let injector_title = text("配置项快捷注入").size(13);
    let mut injector_checks = column![injector_title].spacing(4);

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

    // 模板下拉
    let template_pick = pick_list(
        state.template_options.as_slice(),
        state.selected_template.clone(),
        Message::TemplateSelected,
    );

    // JSON 多行编辑器
    let json_editor = text_editor(&state.config_content)
        .on_action(Message::ConfigAction)
        .height(Length::Fixed(200.0))
        .padding(8);

    // 底部按钮
    let reset_btn = button(text("恢复默认").size(12))
        .on_press(Message::Reset)
        .style(theme::toolbar_btn_style());
    let save_template_btn = button(text("保存为模板").size(12))
        .on_press(Message::SaveAsTemplateClicked)
        .style(theme::toolbar_btn_style());
    let cancel_btn = button(text("取消").size(12))
        .on_press(Message::Cancel)
        .style(theme::toolbar_btn_style());
    let save_btn = button(text("保存").size(12))
        .on_press(Message::Save)
        .style(theme::accent_btn_style());

    let buttons = row![reset_btn, save_template_btn, cancel_btn, save_btn].spacing(8);

    let mut content = column![
        title_row,
        path_label,
        injector_panel,
        template_pick,
        json_editor,
    ]
    .spacing(10)
    .padding(15)
    .width(Length::Fill);

    // 保存为模板输入
    if state.show_save_as_template {
        let name_input = text_input("模板名称", &state.save_as_template_name)
            .on_input(Message::SaveAsTemplateNameChanged)
            .padding(6);
        let confirm = button(text("确定").size(12))
            .on_press(Message::SaveAsTemplateConfirm)
            .style(theme::accent_btn_style());
        let cancel2 = button(text("取消").size(12))
            .on_press(Message::SaveAsTemplateCancel)
            .style(theme::toolbar_btn_style());
        content = content.push(row![name_input, confirm, cancel2].spacing(8));
    }

    if let Some(err) = &state.error {
        content = content.push(text(err).color(theme::DANGER).size(12));
    }

    content = content.push(buttons);

    container(content)
        .width(Length::Fixed(700.0))
        .height(Length::Fixed(650.0))
        .style(theme::overlay_container())
        .into()
}
