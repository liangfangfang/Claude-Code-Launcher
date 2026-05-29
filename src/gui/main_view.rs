/// 主窗口视图 - 工具栏 + 分组页签 + 可滚动项目列表 + 空状态提示。
use iced::widget::{Space, button, column, container, horizontal_rule, row, scrollable, text, text_input};
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
    let toolbar = toolbar_view(app.project_list.len(), app.is_admin);
    let search_bar = search_bar_view(&app.search_query);
    let group_tabs = group_tabs_view(app);
    let content = if app.project_list.is_empty() && app.selected_group_id.is_none() {
        empty_state_view()
    } else {
        project_list_view(app)
    };

    let mut layout = column![toolbar, search_bar, group_tabs, content]
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

fn toolbar_view(project_count: usize, is_admin: bool) -> Element<'static, Message> {
    let admin_status = if is_admin {
        text("管理员").size(11).color(theme::SUCCESS)
    } else {
        text("普通用户").size(11).color(theme::WARNING)
    };
    let title = row![
        text(format!("Claude Code 启动器 ({project_count})")).size(18),
        admin_status,
    ]
    .spacing(8)
    .align_y(Alignment::Center);
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

fn search_bar_view(query: &str) -> Element<'_, Message> {
    let search_input = text_input("搜索项目名称或路径...", query)
        .on_input(Message::SearchQueryChanged)
        .padding(8)
        .width(Length::Fill);

    container(search_input)
        .width(Length::Fill)
        .padding([8, 20])
        .style(theme::toolbar_container())
        .into()
}

/// 分组页签视图
fn group_tabs_view(app: &App) -> Element<'_, Message> {
    let mut tabs = row![].spacing(4);

    // "全部" 页签
    let all_selected = app.selected_group_id.is_none();
    let all_btn = button(text("全部").size(13))
        .on_press(Message::GroupSelected(None))
        .style(move |t, status| {
            if all_selected {
                theme::accent_button_style(t, status)
            } else {
                theme::toolbar_button_style(t, status)
            }
        });
    tabs = tabs.push(all_btn);

    // 各个分组页签
    for group in &app.group_list {
        let group_id = group.id.clone();
        let is_selected = app.selected_group_id.as_ref() == Some(&group_id);

        let group_btn = button(text(&group.name).size(13))
            .on_press(Message::GroupSelected(Some(group_id.clone())))
            .style(move |t, status| {
                if is_selected {
                    theme::accent_button_style(t, status)
                } else {
                    theme::toolbar_button_style(t, status)
                }
            });

        tabs = tabs.push(group_btn);
    }

    // 添加分组按钮（最多10个）
    if app.group_list.len() < 10 {
        let add_group_btn = button(text("+").size(13))
            .on_press(Message::AddGroup)
            .style(theme::toolbar_btn_style());
        tabs = tabs.push(add_group_btn);
    }

    container(tabs)
        .width(Length::Fill)
        .padding([8, 20])
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

    // Filter projects by search query
    let query = app.search_query.to_lowercase();
    let filtered: Vec<_> = if query.is_empty() {
        app.project_list.iter().collect()
    } else {
        app.project_list
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query)
                    || p.path.to_lowercase().contains(&query)
            })
            .collect()
    };

    // 按分组筛选
    let grouped: Vec<_> = if let Some(group_id) = &app.selected_group_id {
        filtered
            .into_iter()
            .filter(|p| p.group_id.as_ref() == Some(group_id))
            .collect()
    } else {
        filtered
    };

    if grouped.is_empty() {
        let msg = if app.search_query.is_empty() {
            "该分组下暂无项目"
        } else {
            "没有匹配的项目"
        };
        return container(
            text(msg)
                .color(theme::TEXT_GRAY)
                .size(14),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into();
    }

    // 如果是"全部"视图，按分组分隔显示
    if app.selected_group_id.is_none() && !app.group_list.is_empty() {
        return project_list_grouped_view(app, cols);
    }

    // 普通列表视图（带分组操作按钮）
    let base = project_list_flat_view(app, &grouped, cols);

    // 如果选中了某个分组，在底部显示编辑/删除按钮
    if let Some(group_id) = &app.selected_group_id {
        if let Some(group) = app.group_list.iter().find(|g| &g.id == group_id) {
            let group_name = &group.name;
            let gid = group_id.clone();
            let edit_btn = button(text(format!("编辑分组「{group_name}」")).size(13))
                .on_press(Message::EditGroup(gid.clone()))
                .style(theme::toolbar_btn_style());
            let del_btn = button(text(format!("删除分组「{group_name}」")).size(13))
                .on_press(Message::DeleteGroup(gid))
                .style(theme::danger_btn_style());
            let actions = container(row![edit_btn, del_btn].spacing(10))
                .padding([8, 20])
                .width(Length::Fill)
                .style(theme::toolbar_container());
            return column![base, actions]
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }
    }

    base
}

/// 普通列表视图（不分组）
fn project_list_flat_view<'a>(app: &'a App, projects: &[&'a crate::core::models::Project], cols: usize) -> Element<'a, Message> {
    let mut rows = Vec::new();
    for chunk in projects.chunks(cols) {
        let mut card_row = row![].spacing(10).width(Length::Fill);
        for project in chunk {
            let skip = app.skip_permissions.get(&project.id).copied().unwrap_or(false);
            let cont = app.continue_session.get(&project.id).copied().unwrap_or(false);
            card_row = card_row.push(project_card::view(project, skip, cont));
        }
        if chunk.len() < cols {
            for _ in 0..(cols - chunk.len()) {
                card_row = card_row.push(Space::with_width(Length::Fill));
            }
        }
        rows.push(card_row);
    }

    let cards = rows
        .into_iter()
        .fold(column![].spacing(10), iced::widget::Column::push);

    let scroll = scrollable(cards.padding([20, 20]))
        .width(Length::Fill)
        .height(Length::Fill);

    container(scroll)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// 分组视图（全部显示时，不同分组用分隔线分开）
fn project_list_grouped_view<'a>(app: &'a App, cols: usize) -> Element<'a, Message> {
    let mut layout = column![].spacing(10);

    // 按分组分组
    let mut groups: Vec<(&str, Vec<&crate::core::models::Project>)> = Vec::new();

    // 先添加有分组的项目
    for group in &app.group_list {
        let group_projects: Vec<_> = app.project_list
            .iter()
            .filter(|p| p.group_id.as_ref() == Some(&group.id))
            .collect();
        if !group_projects.is_empty() {
            groups.push((&group.name, group_projects));
        }
    }

    // 添加未分组的项目
    let ungrouped: Vec<_> = app.project_list
        .iter()
        .filter(|p| p.group_id.is_none())
        .collect();
    if !ungrouped.is_empty() {
        groups.push(("未分组", ungrouped));
    }

    // 渲染每个分组
    for (i, (group_name, group_projects)) in groups.iter().enumerate() {
        // 分组标题
        let header = container(
            row![
                text(*group_name).size(14).color(theme::ACCENT),
                text(format!(" ({} 项)", group_projects.len())).size(12).color(theme::TEXT_GRAY),
            ]
            .align_y(Alignment::Center),
        )
        .padding([4, 0]);

        layout = layout.push(header);

        // 分组内项目
        let mut rows = Vec::new();
        for chunk in group_projects.chunks(cols) {
            let mut card_row = row![].spacing(10).width(Length::Fill);
            for project in chunk {
                let skip = app.skip_permissions.get(&project.id).copied().unwrap_or(false);
                let cont = app.continue_session.get(&project.id).copied().unwrap_or(false);
                card_row = card_row.push(project_card::view(project, skip, cont));
            }
            if chunk.len() < cols {
                for _ in 0..(cols - chunk.len()) {
                    card_row = card_row.push(Space::with_width(Length::Fill));
                }
            }
            rows.push(card_row);
        }

        let cards = rows
            .into_iter()
            .fold(column![].spacing(10), iced::widget::Column::push);
        layout = layout.push(cards);

        // 分隔线（除了最后一个分组）
        if i < groups.len() - 1 {
            layout = layout.push(horizontal_rule(1));
        }
    }

    let scroll = scrollable(layout.padding([20, 20]))
        .width(Length::Fill)
        .height(Length::Fill);

    container(scroll)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
