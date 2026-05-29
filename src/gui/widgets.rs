/// Common widget helpers for Claude Code Launcher.
///
/// Provides reusable button and container constructors to reduce
/// boilerplate in dialog and view code.
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

use crate::gui::app::Message;
use crate::gui::theme;

/// Creates a standard action button with consistent sizing.
pub fn action_button<'a>(
    label: &'a str,
    msg: Message,
    style: impl Fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style + 'a,
) -> iced::widget::Button<'a, Message> {
    button(text(label).size(13).color(theme::WHITE))
        .on_press(msg)
        .padding([6, 16])
        .style(style)
}

/// Creates a small button for card-level actions.
pub fn small_button<'a>(
    label: &'a str,
    msg: Message,
    style: impl Fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style + 'a,
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

/// Creates a labeled section with a title and content.
pub fn section<'a>(title: &'a str, content: Element<'a, Message>) -> Element<'a, Message> {
    column![text(title).size(14).color(theme::WHITE), content]
        .spacing(6)
        .into()
}

/// Creates an error message display.
pub fn error_message(msg: &str) -> Element<'_, Message> {
    container(text(msg).size(12).color(theme::DANGER))
        .padding([4, 0])
        .into()
}

/// Creates a status/info message display.
pub fn info_message(msg: &str) -> Element<'_, Message> {
    container(text(msg).size(12).color(theme::TEXT_GRAY))
        .padding([4, 0])
        .into()
}

/// Creates a labeled form row (label + input).
pub fn form_row<'a>(label: &'a str, input: Element<'a, Message>) -> Element<'a, Message> {
    row![
        text(label).size(13).width(Length::Fixed(100.0)),
        input
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center)
    .into()
}

/// Creates a dialog title bar with close button.
pub fn dialog_title_bar(title: &str) -> Element<'_, Message> {
    row![
        text(title).size(18),
        row![].width(Length::Fill),
        button(text("X").size(16))
            .on_press(Message::DialogDismissed)
            .style(theme::toolbar_btn_style()),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}
