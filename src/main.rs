// 隐藏控制台窗口，仅显示 GUI
#![windows_subsystem = "windows"]

fn main() -> iced::Result {
    // 初始化日志
    tracing_subscriber::fmt::init();

    tracing::info!("Claude Code Launcher v2.1.0 启动");

    // 首次启动自动备份
    if let Ok(mgr) = claude_launcher::core::backup_manager::BackupManager::default_manager() {
        mgr.auto_backup_on_first_run();
    }

    // 清理残留的启动脚本文件
    claude_launcher::launcher::terminal_launcher::cleanup_old_launch_scripts();

    // 默认字体设为微软雅黑，确保中文正常显示
    let default_font = iced::Font {
        family: iced::font::Family::Name("Microsoft YaHei"),
        weight: iced::font::Weight::Normal,
        stretch: iced::font::Stretch::Normal,
        style: iced::font::Style::Normal,
    };

    // 加载中文字体到 iced 的字体系统
    let font_task = load_chinese_font();

    // 加载窗口图标
    let window_icon = iced::window::icon::from_file_data(
        include_bytes!("../assets/icon.png"),
        None,
    )
    .ok();

    // 窗口设置：大小、居中、图标
    let window_settings = iced::window::Settings {
        size: iced::Size::new(1000.0, 700.0),
        position: iced::window::Position::Centered,
        icon: window_icon,
        ..Default::default()
    };

    // 使用 run_with 传入初始状态（App 不实现 Default）
    iced::application(
        "Claude Code 启动器",
        claude_launcher::App::update,
        claude_launcher::App::view,
    )
    .default_font(default_font)
    .theme(|_| iced::Theme::Dark)
    .window(window_settings)
    .subscription(claude_launcher::App::subscription)
    .run_with(|| {
        let app = claude_launcher::App::new();
        (app, font_task)
    })
}

/// 尝试从 Windows 系统字体目录加载中文字体。
///
/// iced 0.13 的 `Family::Name` 需要字体先通过 `font::load()` 注册到
/// iced 的字体系统中，否则只会使用内置的 Latin 字体，导致中文显示为方框。
fn load_chinese_font() -> iced::Task<claude_launcher::gui::app::Message> {
    let font_paths = [
        "C:\\Windows\\Fonts\\msyh.ttc",   // 微软雅黑 (TrueType Collection)
        "C:\\Windows\\Fonts\\msyh.ttf",   // 微软雅黑 (单个文件)
        "C:\\Windows\\Fonts\\simhei.ttf", // 黑体
        "C:\\Windows\\Fonts\\simsun.ttc", // 宋体
    ];

    for path in &font_paths {
        if let Ok(bytes) = std::fs::read(path) {
            tracing::info!("正在加载中文字体: {}", path);
            return iced::font::load(bytes).map(|result| {
                match result {
                    Ok(()) => tracing::info!("中文字体加载成功"),
                    Err(e) => tracing::warn!("中文字体加载失败: {:?}", e),
                }
                claude_launcher::gui::app::Message::FontLoaded
            });
        }
    }

    tracing::warn!("未找到中文字体文件，中文可能无法正常显示");
    iced::Task::none()
}
