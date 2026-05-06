/// 暗色主题常量与自定义样式。
///
/// 定义 Claude Code 启动器的配色方案和各组件样式。
use iced::widget::{button, container};
use iced::{Border, Color, Shadow, Vector};

// ── 配色常量 ──────────────────────────────────────────────────────────

/// 主背景色 #1e293b
pub const BG: Color = Color::from_rgb(30.0 / 255.0, 41.0 / 255.0, 59.0 / 255.0);
/// 工具栏背景色 #334155
pub const TOOLBAR_BG: Color = Color::from_rgb(51.0 / 255.0, 65.0 / 255.0, 85.0 / 255.0);
/// 强调色 #3b82f6
pub const ACCENT: Color = Color::from_rgb(59.0 / 255.0, 130.0 / 255.0, 246.0 / 255.0);
/// 强调色悬停态 #2563eb
pub const ACCENT_HOVER: Color = Color::from_rgb(37.0 / 255.0, 99.0 / 255.0, 235.0 / 255.0);
/// 卡片背景色 #334155
pub const CARD_BG: Color = Color::from_rgb(51.0 / 255.0, 65.0 / 255.0, 85.0 / 255.0);
/// 危险色 #dc2626
pub const DANGER: Color = Color::from_rgb(220.0 / 255.0, 38.0 / 255.0, 38.0 / 255.0);
/// 成功色 #38a169
pub const SUCCESS: Color = Color::from_rgb(56.0 / 255.0, 161.0 / 255.0, 105.0 / 255.0);
/// 警告色 #f59e0b
pub const WARNING: Color = Color::from_rgb(245.0 / 255.0, 158.0 / 255.0, 11.0 / 255.0);
/// 灰色文本
pub const TEXT_GRAY: Color = Color::from_rgb(160.0 / 255.0, 160.0 / 255.0, 160.0 / 255.0);
/// 次要按钮色 #475569
pub const SECONDARY: Color = Color::from_rgb(71.0 / 255.0, 85.0 / 255.0, 105.0 / 255.0);
/// 次要按钮悬停态 #64748b
pub const SECONDARY_HOVER: Color = Color::from_rgb(100.0 / 255.0, 116.0 / 255.0, 139.0 / 255.0);
/// 白色
pub const WHITE: Color = Color::WHITE;

// ── 按钮样式函数 ──────────────────────────────────────────────────────

/// 强调色按钮样式（启动、添加项目）
pub fn accent_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => ACCENT_HOVER,
        button::Status::Disabled => Color::from_rgb8(40, 60, 100),
        _ => ACCENT,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: WHITE,
        border: Border::default().rounded(4),
        shadow: Shadow::default(),
    }
}

/// 工具栏按钮样式
pub fn toolbar_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => SECONDARY_HOVER,
        button::Status::Disabled => Color::from_rgb8(50, 60, 75),
        _ => SECONDARY,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: WHITE,
        border: Border::default().rounded(4),
        shadow: Shadow::default(),
    }
}

/// 危险按钮样式（删除）
pub fn danger_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => Color::from_rgb8(185, 28, 28),
        button::Status::Disabled => Color::from_rgb8(120, 30, 30),
        _ => DANGER,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: WHITE,
        border: Border::default().rounded(4),
        shadow: Shadow::default(),
    }
}

/// 警告按钮样式（临时项目）
pub fn warning_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => Color::from_rgb8(217, 119, 6),
        button::Status::Disabled => Color::from_rgb8(120, 80, 10),
        _ => WARNING,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: WHITE,
        border: Border::default().rounded(4),
        shadow: Shadow::default(),
    }
}

/// 成功按钮样式（保存）
pub fn success_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => Color::from_rgb8(72, 187, 120),
        button::Status::Disabled => Color::from_rgb8(40, 80, 50),
        _ => SUCCESS,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: WHITE,
        border: Border::default().rounded(4),
        shadow: Shadow::default(),
    }
}

// ── 容器样式函数 ──────────────────────────────────────────────────────

/// 工具栏容器样式
pub fn toolbar_container_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(TOOLBAR_BG.into()),
        text_color: Some(WHITE),
        border: Border::default(),
        shadow: Shadow::default(),
    }
}

/// 卡片容器样式
pub fn card_container_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(CARD_BG.into()),
        text_color: Some(WHITE),
        border: Border::default().rounded(8),
        shadow: Shadow::default(),
    }
}

/// 主背景容器样式
pub fn background_container_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(BG.into()),
        text_color: Some(WHITE),
        border: Border::default(),
        shadow: Shadow::default(),
    }
}

/// 对话框遮罩容器样式
pub fn overlay_container_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgb8(15, 23, 42).into()),
        text_color: Some(WHITE),
        border: Border {
            color: ACCENT,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.5),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
    }
}

/// 注入面板容器样式
pub fn panel_container_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgb8(26, 32, 44).into()),
        text_color: Some(WHITE),
        border: Border::default().rounded(4),
        shadow: Shadow::default(),
    }
}

// ── 样式辅助函数（用于 style() 调用） ──────────────────────────────

pub fn accent_btn_style() -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    accent_button_style
}

pub fn toolbar_btn_style() -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    toolbar_button_style
}

pub fn danger_btn_style() -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    danger_button_style
}

pub fn warning_btn_style() -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    warning_button_style
}

pub fn success_btn_style() -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    success_button_style
}

pub fn background_container() -> impl Fn(&iced::Theme) -> container::Style {
    background_container_style
}

pub fn toolbar_container() -> impl Fn(&iced::Theme) -> container::Style {
    toolbar_container_style
}

pub fn card_container() -> impl Fn(&iced::Theme) -> container::Style {
    card_container_style
}

pub fn overlay_container() -> impl Fn(&iced::Theme) -> container::Style {
    overlay_container_style
}

pub fn panel_container() -> impl Fn(&iced::Theme) -> container::Style {
    panel_container_style
}
