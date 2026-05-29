/// 添加项目对话框 - 输入名称、选择路径、选择分组、选择模板。
use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::{Element, Length};

use crate::gui::theme;

/// 添加项目对话框状态
#[derive(Debug, Clone)]
pub struct State {
    pub name: String,
    pub path: String,
    pub group_options: Vec<String>,
    pub selected_group: Option<String>,
    pub template_options: Vec<String>,
    pub selected_template: Option<String>,
    pub error: Option<String>,
}

impl State {
    pub fn new(template_options: Vec<String>, group_options: Vec<String>) -> Self {
        let default_template = template_options.first().cloned();
        let default_group = group_options.first().cloned();
        Self {
            name: String::new(),
            path: String::new(),
            group_options,
            selected_group: default_group,
            template_options,
            selected_template: default_template,
            error: None,
        }
    }
}

/// 添加项目对话框消息
#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    PathChanged(String),
    BrowseClicked,
    GroupSelected(String),
    TemplateSelected(String),
    Save,
    Cancel,
}

pub fn view(state: &State) -> Element<'_, Message> {
    let title = text("添加项目").size(18);

    let name_label = text("项目名称：").size(13);
    let name_input = text_input("我的项目", &state.name)
        .on_input(Message::NameChanged)
        .padding(8);

    let path_label = text("项目目录：").size(13);
    let path_input = text_input("选择目录...", &state.path)
        .on_input(Message::PathChanged)
        .padding(8);
    let browse_btn = button(text("浏览").size(12))
        .on_press(Message::BrowseClicked)
        .style(theme::toolbar_btn_style());

    let path_row = row![path_input, browse_btn].spacing(8);

    // 分组选择
    let group_label = text("项目分组：").size(13);
    let group_pick = pick_list(
        state.group_options.as_slice(),
        state.selected_group.clone(),
        Message::GroupSelected,
    );

    // 模板选择
    let template_label = text("配置模板：").size(13);
    let template_pick = pick_list(
        state.template_options.as_slice(),
        state.selected_template.clone(),
        Message::TemplateSelected,
    );

    let error_text = state
        .error
        .as_ref()
        .map(|e| text(e).color(theme::DANGER).size(12));

    let cancel_btn = button(text("取消").size(13))
        .on_press(Message::Cancel)
        .style(theme::toolbar_btn_style());
    let save_btn = button(text("添加").size(13))
        .on_press(Message::Save)
        .style(theme::accent_btn_style());

    let buttons = row![cancel_btn, save_btn].spacing(10);

    let mut content = column![
        title,
        name_label,
        name_input,
        path_label,
        path_row,
        group_label,
        group_pick,
        template_label,
        template_pick
    ]
    .spacing(10)
    .padding(20);

    if let Some(err) = error_text {
        content = content.push(err);
    }

    content = content.push(buttons);

    container(content)
        .width(Length::Fixed(500.0))
        .style(theme::overlay_container())
        .into()
}
