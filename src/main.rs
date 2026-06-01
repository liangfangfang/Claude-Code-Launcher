// 隐藏控制台窗口，仅显示 GUI（仅 Windows 需要）
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() -> iced::Result {
    // 初始化日志
    tracing_subscriber::fmt::init();

    tracing::info!("Claude Code Launcher v2.2.1 启动");

    // Windows 专有：管理员权限检测 + 配置兼容性检查
    #[cfg(target_os = "windows")]
    {
        if !claude_launcher::admin::is_admin() {
            tracing::warn!("未以管理员身份运行，尝试提升权限...");
            match claude_launcher::admin::request_elevation() {
                Ok(true) => {
                    std::process::exit(0);
                }
                Ok(false) => {
                    show_admin_required_dialog();
                    std::process::exit(1);
                }
                Err(e) => {
                    tracing::error!("权限提升失败: {e}");
                    show_admin_required_dialog();
                    std::process::exit(1);
                }
            }
        }

        tracing::info!("已以管理员身份运行");

        check_config_compatibility();
    }

    // 首次启动自动备份
    if let Ok(mgr) = claude_launcher::core::backup_manager::BackupManager::default_manager() {
        mgr.auto_backup_on_first_run();
    }

    // 清理残留的启动脚本文件
    claude_launcher::launcher::terminal_launcher::cleanup_old_launch_scripts();

    // macOS：使用华文黑体（固定路径，始终存在）作为默认中文字体
    // Windows：使用微软雅黑
    #[cfg(target_os = "macos")]
    let default_font = iced::Font {
        family: iced::font::Family::Name("STHeiti"),
        weight: iced::font::Weight::Normal,
        stretch: iced::font::Stretch::Normal,
        style: iced::font::Style::Normal,
    };
    #[cfg(target_os = "windows")]
    let default_font = iced::Font {
        family: iced::font::Family::Name("Microsoft YaHei"),
        weight: iced::font::Weight::Normal,
        stretch: iced::font::Stretch::Normal,
        style: iced::font::Style::Normal,
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let default_font = iced::Font::default();

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

// ── Windows 专有函数 ──────────────────────────────────────────────────

/// 显示需要管理员权限的对话框
#[cfg(target_os = "windows")]
fn show_admin_required_dialog() {
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let title: Vec<u16> = "Claude Code 启动器\0".encode_utf16().collect();
    let message: Vec<u16> = "此应用程序需要管理员权限才能正常运行。\n\n请右键点击程序，选择「以管理员身份运行」。\0"
        .encode_utf16()
        .collect();

    unsafe {
        MessageBoxW(
            None,
            windows::core::PCWSTR(message.as_ptr()),
            windows::core::PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

/// 检查配置目录兼容性，发现旧版本配置时弹窗确认归档。
#[cfg(target_os = "windows")]
fn check_config_compatibility() {
    let report = claude_launcher::core::config_migrator::validate_config_dir();

    if !report.has_files {
        tracing::info!("配置目录为空，将正常初始化");
        return;
    }

    if report.is_compatible() {
        tracing::info!("配置文件格式兼容，正常启动");
        return;
    }

    tracing::warn!(
        "发现不兼容的配置文件: {:?}",
        report.incompatible_files
    );

    let msg = claude_launcher::core::config_migrator::build_archive_message(&report);
    let confirmed = show_archive_confirm_dialog(&msg);

    if confirmed {
        match claude_launcher::core::config_migrator::archive_and_reinitialize() {
            Ok(archive_dir) => {
                tracing::info!("旧配置已归档至: {:?}", archive_dir);
                show_info_dialog(&format!(
                    "旧配置已归档至：\n{}\n\n程序将使用全新配置启动。",
                    archive_dir.display()
                ));
            }
            Err(e) => {
                tracing::error!("归档失败: {}", e);
                show_error_dialog(&format!("归档旧配置失败：\n{e}\n\n程序将退出。"));
                std::process::exit(1);
            }
        }
    } else {
        tracing::info!("用户取消归档，程序退出");
        std::process::exit(0);
    }
}

/// 显示归档确认对话框，返回用户是否确认。
#[cfg(target_os = "windows")]
fn show_archive_confirm_dialog(message: &str) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONWARNING, MB_YESNO, IDYES,
    };

    let title: Vec<u16> = "配置兼容性检查\0".encode_utf16().collect();
    let msg: Vec<u16> = format!("{message}\0").encode_utf16().collect();

    let result = unsafe {
        MessageBoxW(
            None,
            windows::core::PCWSTR(msg.as_ptr()),
            windows::core::PCWSTR(title.as_ptr()),
            MB_YESNO | MB_ICONWARNING,
        )
    };

    result == IDYES
}

/// 显示信息对话框
#[cfg(target_os = "windows")]
fn show_info_dialog(message: &str) {
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};

    let title: Vec<u16> = "Claude Code 启动器\0".encode_utf16().collect();
    let msg: Vec<u16> = format!("{message}\0").encode_utf16().collect();

    unsafe {
        MessageBoxW(
            None,
            windows::core::PCWSTR(msg.as_ptr()),
            windows::core::PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

/// 显示错误对话框
#[cfg(target_os = "windows")]
fn show_error_dialog(message: &str) {
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let title: Vec<u16> = "Claude Code 启动器\0".encode_utf16().collect();
    let msg: Vec<u16> = format!("{message}\0").encode_utf16().collect();

    unsafe {
        MessageBoxW(
            None,
            windows::core::PCWSTR(msg.as_ptr()),
            windows::core::PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

// ── 字体加载 ──────────────────────────────────────────────────────────

/// 尝试从系统字体目录加载中文字体。
///
/// iced 0.13 的 `Family::Name` 需要字体先通过 `font::load()` 注册到
/// iced 的字体系统中，否则只会使用内置的 Latin 字体，导致中文显示为方框。
fn load_chinese_font() -> iced::Task<claude_launcher::gui::app::Message> {
    // macOS：扫描系统字体目录中有 CJK 支持的字体
    #[cfg(target_os = "macos")]
    let font_paths = get_macos_cjk_font_paths();

    // Windows 系统字体
    #[cfg(target_os = "windows")]
    let font_paths = [
        "C:\\Windows\\Fonts\\msyh.ttc".to_string(),   // 微软雅黑
        "C:\\Windows\\Fonts\\msyh.ttf".to_string(),   // 微软雅黑 (单个文件)
        "C:\\Windows\\Fonts\\simhei.ttf".to_string(), // 黑体
        "C:\\Windows\\Fonts\\simsun.ttc".to_string(), // 宋体
    ];

    // Linux / 其他平台回退
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let font_paths: [String; 0] = [];

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

/// 在 macOS 上扫描已知位置的中文字体文件。
#[cfg(target_os = "macos")]
fn get_macos_cjk_font_paths() -> Vec<String> {
    let mut paths = Vec::new();

    // 固定路径的字体（macOS 13+ 仍保留这些）
    for p in &[
        "/System/Library/Fonts/STHeiti Medium.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/Supplemental/Songti.ttc",
    ] {
        if std::path::Path::new(p).exists() {
            paths.push(p.to_string());
        }
    }

    // PingFang 在 macOS Ventura+ 移到了 AssetsV2 动态路径
    if let Ok(entries) = std::fs::read_dir("/System/Library/AssetsV2") {
        for entry in entries.flatten() {
            let asset_dir = entry.path();
            let font_file = asset_dir.join("AssetData").join("PingFang.ttc");
            if font_file.exists() {
                paths.push(font_file.to_string_lossy().to_string());
                break;
            }
        }
    }

    paths
}
