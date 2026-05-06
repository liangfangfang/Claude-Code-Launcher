/// 主窗口视图 - 工具栏 + 可滚动项目列表 + 空状态提示。
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::App;
use crate::gui::app::Message;
use crate::gui::{project_card, theme};

/// 根据窗口宽度决定列数
fn column_count(width: f32) -> usize {
    if width < 700.0 {
        1
    } else if width < 1100.0 {
        2
    } else {
        3
    }
}

/// 构建主窗口视图
pub fn view(app: &App) -> Element<'_, Message> {
    let toolbar = toolbar_view();
    let content = if app.project_list.is_empty() {
        empty_state_view()
    } else {
        project_list_view(app)
    };

    let mut layout = column![toolbar, content]
        .width(Length::Fill)
        .height(Length::Fill);

    // 底部状态提示
    if let Some(msg) = &app.status_message {
        let status_bar = container(text(msg).size(12).color(theme::TEXT_GRAY))
            .width(Length::Fill)
            .padding([4, 20])
            .style(theme::toolbar_container());
        layout = layout.push(status_bar);
    }

    container(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::background_container())
        .into()
}

fn toolbar_view() -> Element<'static, Message> {
    let title = text("Claude Code 启动器").size(18);
    let spacer = Space::with_width(Length::Fill);

    let about_btn = button(text("关于").size(13))
        .on_press(Message::About)
        .style(theme::toolbar_btn_style());
    let add_btn = button(text("+ 添加项目").size(13))
        .on_press(Message::AddProject)
        .style(theme::accent_btn_style());
    let temp_btn = button(text("+ 临时").size(13))
        .on_press(Message::TempProject)
        .style(theme::warning_btn_style());
    let settings_btn = button(text("设置").size(13))
        .on_press(Message::OpenSettings)
        .style(theme::toolbar_btn_style());

    let toolbar_row = row![title, spacer, about_btn, add_btn, temp_btn, settings_btn]
        .spacing(10)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fixed(60.0));

    container(toolbar_row)
        .width(Length::Fill)
        .padding([0, 20])
        .style(theme::toolbar_container())
        .into()
}

fn empty_state_view() -> Element<'static, Message> {
    let msg = text("暂无项目\n点击「+ 添加项目」开始使用")
        .color(theme::TEXT_GRAY)
        .size(14);

    container(msg)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn project_list_view(app: &App) -> Element<'_, Message> {
    let cols = column_count(app.window_width);

    // 将项目卡片按列数分组，每组为一行
    let mut rows = Vec::new();
    for chunk in app.project_list.chunks(cols) {
        let mut card_row = row![].spacing(10).width(Length::Fill);
        for project in chunk {
            let skip = app
                .skip_permissions
                .get(&project.id)
                .copied()
                .unwrap_or(false);
            let cont = app
                .continue_session
                .get(&project.id)
                .copied()
                .unwrap_or(false);
            card_row = card_row.push(project_card::view(project, skip, cont));
        }
        // 如果最后一行不足 cols 个，填充空白占位
        if chunk.len() < cols {
            for _ in 0..(cols - chunk.len()) {
                card_row = card_row.push(Space::with_width(Length::Fill));
            }
        }
        rows.push(card_row);
    }

    let cards = rows
        .into_iter()
        .fold(column![].spacing(10), |col, r| col.push(r));

    let scroll = scrollable(cards.padding([20, 20]))
        .width(Length::Fill)
        .height(Length::Fill);

    container(scroll)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
