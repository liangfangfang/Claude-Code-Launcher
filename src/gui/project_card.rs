/// 项目卡片组件 - 展示单个项目信息和操作按钮。
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::core::models::Project;
use crate::gui::app::Message;
use crate::gui::theme;

/// 构建单个项目卡片视图
pub fn view(
    project: &Project,
    skip_permissions: bool,
    continue_session: bool,
) -> Element<'_, Message> {
    let icon = text("●").size(24).color(theme::ACCENT);
    let name = text(&project.name).size(16);
    let path_text = text(&project.path).size(12).color(theme::TEXT_GRAY);

    let skip_check = iced::widget::checkbox("跳过权限", skip_permissions)
        .on_toggle(move |val| Message::ToggleSkipPermissions(project.id.clone(), val))
        .size(iced::Pixels(14.0));

    let continue_check = iced::widget::checkbox("继续会话", continue_session)
        .on_toggle(move |val| Message::ToggleContinueSession(project.id.clone(), val))
        .size(iced::Pixels(14.0));

    let options = row![skip_check, continue_check].spacing(15);

    let info = column![name, path_text, options]
        .spacing(4)
        .width(Length::Fill);

    let launch_btn = make_button(
        "启动",
        Message::LaunchProject(project.id.clone()),
        theme::accent_btn_style(),
    );
    let edit_btn = make_button(
        "编辑",
        Message::EditProject(project.id.clone()),
        theme::toolbar_btn_style(),
    );
    let config_btn = make_button(
        "配置",
        Message::OpenProjectConfig(project.id.clone()),
        theme::toolbar_btn_style(),
    );
    let open_dir_btn = make_button(
        "打开目录",
        Message::OpenDirectory(project.id.clone()),
        theme::toolbar_btn_style(),
    );
    let delete_btn = make_button(
        "删除",
        Message::DeleteProject(project.id.clone()),
        theme::danger_btn_style(),
    );

    let buttons = column![launch_btn, edit_btn, config_btn, open_dir_btn, delete_btn]
        .spacing(4)
        .width(Length::Fixed(80.0));

    let card_row = row![icon, info, buttons]
        .spacing(10)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    container(card_row)
        .width(Length::FillPortion(1))
        .padding(15)
        .style(theme::card_container())
        .into()
}

fn make_button<'a>(
    label: &'a str,
    msg: Message,
    style: impl Fn(&iced::Theme, button::Status) -> button::Style + 'a,
) -> iced::widget::Button<'a, Message> {
    button(
        text(label)
            .size(12)
            .color(theme::WHITE)
            .width(Length::Shrink),
    )
    .on_press(msg)
    .width(Length::Fill)
    .height(Length::Fixed(28.0))
    .padding([3, 8])
    .style(style)
}
