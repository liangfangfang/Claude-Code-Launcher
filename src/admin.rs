/// 管理员权限检测和提升模块

/// 检测当前进程是否以管理员身份运行
#[cfg(target_os = "windows")]
pub fn is_admin() -> bool {
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let process = GetCurrentProcess();
        let mut token = windows::Win32::Foundation::HANDLE::default();

        if OpenProcessToken(process, TOKEN_QUERY, &mut token).is_ok() {
            let mut elevation = TOKEN_ELEVATION::default();
            let mut return_length = 0u32;

            let result = GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut return_length,
            );

            let _ = windows::Win32::Foundation::CloseHandle(token);

            result.is_ok() && elevation.TokenIsElevated != 0
        } else {
            false
        }
    }
}

/// 非 Windows 平台总是返回 true
#[cfg(not(target_os = "windows"))]
pub fn is_admin() -> bool {
    true
}

/// 请求管理员权限提升（UAC）
/// 返回 Ok(true) 如果成功提升，Ok(false) 如果用户拒绝，Err 如果失败
#[cfg(target_os = "windows")]
pub fn request_elevation() -> Result<bool, String> {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

    let exe_path = std::env::current_exe()
        .map_err(|e| format!("无法获取可执行文件路径: {e}"))?;

    let exe_path_wide: Vec<u16> = exe_path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let operation: Vec<u16> = "runas\0".encode_utf16().collect();

    unsafe {
        let result = ShellExecuteW(
            None,
            windows::core::PCWSTR(operation.as_ptr()),
            windows::core::PCWSTR(exe_path_wide.as_ptr()),
            None,
            None,
            SW_SHOW,
        );

        // ShellExecuteW 返回值大于 32 表示成功
        if result.0 as i32 > 32 {
            // 退出当前非管理员进程
            std::process::exit(0);
        } else {
            Ok(false)
        }
    }
}

/// 非 Windows 平台的占位实现
#[cfg(not(target_os = "windows"))]
pub fn request_elevation() -> Result<bool, String> {
    Ok(true)
}
