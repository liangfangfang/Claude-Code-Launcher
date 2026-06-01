//! Terminal launcher module for launching Claude Code in various terminals.
//!
//! **Windows:** Supports Windows Terminal (wt.exe), Command Prompt (cmd.exe),
//! and PowerShell.
//!
//! **macOS:** Supports Terminal.app and iTerm2.
//!
//! Provides auto-detection of available terminals and script generation.
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use thiserror::Error;

// ---------------------------------------------------------------------------
// Terminal type
// ---------------------------------------------------------------------------

/// Terminal types supported for launching Claude Code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalType {
    /// Automatically detect the best available terminal.
    Auto,
    /// Windows Terminal (wt.exe).
    WindowsTerminal,
    /// Command Prompt (cmd.exe).
    Cmd,
    /// PowerShell.
    PowerShell,
    /// macOS Terminal.app.
    TerminalApp,
    /// iTerm2.
    ITerm2,
}

impl TerminalType {
    /// Parse a terminal type string into a [`TerminalType`].
    ///
    /// Recognised values (case-insensitive):
    ///   - `"auto"` → [`TerminalType::Auto`]
    ///   - `"wt"`, `"windows-terminal"` → [`TerminalType::WindowsTerminal`]
    ///   - `"cmd"` → [`TerminalType::Cmd`]
    ///   - `"powershell"` → [`TerminalType::PowerShell`]
    ///   - `"terminal"` → [`TerminalType::TerminalApp`]
    ///   - `"iterm2"` → [`TerminalType::ITerm2`]
    ///
    /// Anything else defaults to [`TerminalType::Auto`].
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "wt" | "windows-terminal" => TerminalType::WindowsTerminal,
            "cmd" => TerminalType::Cmd,
            "powershell" => TerminalType::PowerShell,
            "terminal" => TerminalType::TerminalApp,
            "iterm2" => TerminalType::ITerm2,
            _ => TerminalType::Auto,
        }
    }

    /// Resolve [`TerminalType::Auto`] to an actual terminal type based on
    /// platform-specific availability.
    pub fn resolve(&self) -> Self {
        match self {
            TerminalType::Auto => {
                #[cfg(target_os = "macos")]
                {
                    if detect_iterm2() {
                        TerminalType::ITerm2
                    } else {
                        TerminalType::TerminalApp
                    }
                }
                #[cfg(target_os = "windows")]
                {
                    if detect_windows_terminal() {
                        TerminalType::WindowsTerminal
                    } else if detect_powershell() {
                        TerminalType::PowerShell
                    } else {
                        TerminalType::Cmd
                    }
                }
                #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                {
                    TerminalType::Cmd
                }
            }
            other => other.clone(),
        }
    }
}

impl std::fmt::Display for TerminalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TerminalType::Auto => write!(f, "auto"),
            TerminalType::WindowsTerminal => write!(f, "wt"),
            TerminalType::Cmd => write!(f, "cmd"),
            TerminalType::PowerShell => write!(f, "powershell"),
            TerminalType::TerminalApp => write!(f, "terminal"),
            TerminalType::ITerm2 => write!(f, "iterm2"),
        }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during terminal launching.
#[derive(Debug, Error)]
pub enum LauncherError {
    /// Failed to find a required executable.
    #[error("找不到终端可执行文件: {0}")]
    ExecutableNotFound(String),

    /// Failed to create a launch script.
    #[error("创建启动脚本失败: {0}")]
    ScriptCreationFailed(String),

    /// Failed to launch the terminal process.
    #[error("启动终端失败: {0}")]
    LaunchFailed(String),

    /// IO error during operation.
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Executable discovery (cross-platform)
// ---------------------------------------------------------------------------

/// Search for an executable by name in the system `PATH`.
#[cfg(any(target_os = "windows", test))]
fn find_in_path(name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Windows executable discovery
// ---------------------------------------------------------------------------

/// Find the Windows Terminal (`wt.exe`) executable.
///
/// Searches:
/// 1. System `PATH`
/// 2. `%LOCALAPPDATA%\Microsoft\WindowsApps\wt.exe`
#[cfg(target_os = "windows")]
pub fn find_wt_exe() -> Option<PathBuf> {
    // 1. Search PATH first
    if let Some(path) = find_in_path("wt.exe") {
        return Some(path);
    }

    // 2. Check %LOCALAPPDATA%\Microsoft\WindowsApps\wt.exe
    if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
        let wt_path = PathBuf::from(local_app_data)
            .join("Microsoft")
            .join("WindowsApps")
            .join("wt.exe");
        if wt_path.is_file() {
            return Some(wt_path);
        }
    }

    None
}

/// Non-Windows stub — wt.exe only exists on Windows.
#[cfg(not(target_os = "windows"))]
pub fn find_wt_exe() -> Option<PathBuf> {
    None
}

/// Detect whether Windows Terminal is available on the system.
pub fn detect_windows_terminal() -> bool {
    find_wt_exe().is_some()
}

/// Detect whether PowerShell is available on the system.
#[cfg(target_os = "windows")]
fn detect_powershell() -> bool {
    find_in_path("powershell.exe").is_some()
}

// ---------------------------------------------------------------------------
// macOS executable discovery
// ---------------------------------------------------------------------------

/// Detect whether iTerm2 is installed.
#[cfg(target_os = "macos")]
fn detect_iterm2() -> bool {
    Path::new("/Applications/iTerm.app").exists()
}

/// Non-macOS stub.
#[cfg(not(target_os = "macos"))]
fn detect_iterm2() -> bool {
    false
}

// ---------------------------------------------------------------------------
// Script generation
// ---------------------------------------------------------------------------

/// Generate a unique temp file path in the system temp directory.
fn temp_script_path(extension: &str) -> PathBuf {
    let temp_dir = env::temp_dir();
    let unique_name = format!("claude-launcher-{}.{}", uuid::Uuid::new_v4(), extension);
    temp_dir.join(unique_name)
}

// ---------------------------------------------------------------------------
// Windows script generation
// ---------------------------------------------------------------------------

/// Create PowerShell and batch launch scripts for Claude Code (Windows).
///
/// Returns the paths to the created `.ps1` and `.bat` files.  The caller is
/// responsible for cleaning them up (see [`cleanup_launch_scripts`]).
#[cfg(target_os = "windows")]
pub fn create_launch_scripts(
    project_path: &str,
    project_name: &str,
    claude_cmd_str: &str,
) -> Result<(PathBuf, PathBuf), LauncherError> {
    let window_title = format!("{project_name} - Claude Code");

    // --- PowerShell script ---
    let ps1_content = format!(
        "Set-Location -LiteralPath \"{project_path}\"\n\
         $host.UI.RawUI.WindowTitle = \"{window_title}\"\n\
         {claude_cmd_str}\n"
    );
    let ps1_path = temp_script_path("ps1");
    // 添加 UTF-8 BOM，确保 PowerShell 5.x（Windows 内置版）能正确识别编码
    let mut ps1_bytes: Vec<u8> = vec![0xEF, 0xBB, 0xBF];
    ps1_bytes.extend(ps1_content.as_bytes());
    fs::write(&ps1_path, &ps1_bytes).map_err(|e| {
        LauncherError::ScriptCreationFailed(format!(
            "无法写入PowerShell脚本 {}: {}",
            ps1_path.display(),
            e
        ))
    })?;

    // --- Batch script ---
    let bat_content = format!(
        "@echo off\n\
         title {window_title}\n\
         cd /d \"{project_path}\"\n\
         echo Starting Claude Code in: {project_name}\n\
         echo.\n\
         {claude_cmd_str}\n"
    );
    let bat_path = temp_script_path("bat");
    fs::write(&bat_path, &bat_content).map_err(|e| {
        LauncherError::ScriptCreationFailed(format!(
            "无法写入批处理脚本 {}: {}",
            bat_path.display(),
            e
        ))
    })?;

    Ok((ps1_path, bat_path))
}

// ---------------------------------------------------------------------------
// macOS script generation
// ---------------------------------------------------------------------------

/// Create a shell launch script for Claude Code (macOS).
///
/// Returns the path to the created `.sh` file. The caller is responsible for
/// cleaning it up.
#[cfg(target_os = "macos")]
fn create_sh_launch_script(
    project_path: &str,
    project_name: &str,
    claude_cmd_str: &str,
) -> Result<PathBuf, LauncherError> {
    let window_title = format!("{} - Claude Code", project_name);
    let sh_content = format!(
        "#!/bin/bash\n\
         cd \"{}\" || exit 1\n\
         printf '\\e]0;%s\\a' \"{}\"\n\
         echo \"Starting Claude Code in: {}\"\n\
         echo\n\
         {}\n",
        project_path, window_title, project_name, claude_cmd_str
    );
    let sh_path = temp_script_path("sh");
    fs::write(&sh_path, sh_content.as_bytes()).map_err(|e| {
        LauncherError::ScriptCreationFailed(format!(
            "无法写入Shell脚本 {}: {}",
            sh_path.display(),
            e
        ))
    })?;
    Ok(sh_path)
}

// ---------------------------------------------------------------------------
// Command building (cross-platform)
// ---------------------------------------------------------------------------

/// Build the Claude CLI command string based on launch options.
fn build_claude_command(skip_permissions: bool, continue_session: bool) -> String {
    let mut parts = vec!["claude".to_string()];

    if skip_permissions {
        parts.push("--dangerously-skip-permissions".to_string());
    }
    if continue_session {
        parts.push("--continue".to_string());
    }

    parts.join(" ")
}

// ---------------------------------------------------------------------------
// Windows terminal launchers
// ---------------------------------------------------------------------------

/// Spawn Claude Code inside Windows Terminal.
#[cfg(target_os = "windows")]
fn launch_in_wt(
    ps1_path: &Path,
    project_path: &str,
    project_name: &str,
) -> Result<Child, LauncherError> {
    let wt_exe = find_wt_exe().ok_or_else(|| {
        LauncherError::ExecutableNotFound("找不到 Windows Terminal (wt.exe)".to_string())
    })?;

    let title = format!("{project_name} - Claude Code");
    let ps1 = ps1_path.to_string_lossy().to_string();

    Command::new(&wt_exe)
        .args([
            "-d",
            project_path,
            "--title",
            &title,
            "--",
            "powershell.exe",
            "-NoExit",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &ps1,
        ])
        .spawn()
        .map_err(|e| LauncherError::LaunchFailed(format!("启动 Windows Terminal 失败: {e}")))
}

/// Spawn Claude Code inside Command Prompt.
#[cfg(target_os = "windows")]
fn launch_in_cmd(bat_path: &Path) -> Result<Child, LauncherError> {
    let bat = bat_path.to_string_lossy().to_string();

    Command::new("cmd.exe")
        .args(["/k", &bat])
        .spawn()
        .map_err(|e| LauncherError::LaunchFailed(format!("启动命令提示符失败: {e}")))
}

/// Spawn Claude Code inside PowerShell.
#[cfg(target_os = "windows")]
fn launch_in_powershell(ps1_path: &Path) -> Result<Child, LauncherError> {
    if find_in_path("powershell.exe").is_none() {
        return Err(LauncherError::ExecutableNotFound(
            "找不到 PowerShell (powershell.exe)".to_string(),
        ));
    }
    let ps1 = ps1_path.to_string_lossy().to_string();

    Command::new("powershell.exe")
        .args(["-NoExit", "-ExecutionPolicy", "Bypass", "-File", &ps1])
        .spawn()
        .map_err(|e| LauncherError::LaunchFailed(format!("启动 PowerShell 失败: {e}")))
}

// ---------------------------------------------------------------------------
// macOS terminal launchers
// ---------------------------------------------------------------------------

/// Spawn Claude Code inside macOS Terminal.app.
#[cfg(target_os = "macos")]
fn launch_in_terminal_app(sh_path: &Path) -> Result<Child, LauncherError> {
    let script = format!(
        "tell app \"Terminal\" to do script \"bash '{}'\"",
        sh_path.to_string_lossy()
    );
    Command::new("osascript")
        .args(["-e", &script])
        .spawn()
        .map_err(|e| LauncherError::LaunchFailed(format!("启动 Terminal 失败: {}", e)))
}

/// Spawn Claude Code inside iTerm2.
#[cfg(target_os = "macos")]
fn launch_in_iterm2(sh_path: &Path) -> Result<Child, LauncherError> {
    let script = format!(
        "tell app \"iTerm\" to create window with default profile command \"bash '{}'\"",
        sh_path.to_string_lossy()
    );
    Command::new("osascript")
        .args(["-e", &script])
        .spawn()
        .map_err(|e| LauncherError::LaunchFailed(format!("启动 iTerm2 失败: {}", e)))
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Launch Claude Code in the specified terminal.
///
/// # Arguments
/// * `project_path`  - Path to the project directory.
/// * `project_name`  - Display name used in the terminal window title.
/// * `skip_permissions` - Pass `--dangerously-skip-permissions` to Claude.
/// * `continue_session` - Pass `--continue` to Claude.
/// * `terminal`      - One of `"auto"`, `"wt"`, `"cmd"`, `"powershell"`,
///                      `"terminal"`, `"iterm2"`.
///
/// # Returns
/// A [`Child`] handle to the spawned terminal process.
///
/// # Errors
/// Returns [`LauncherError`] if script creation or process spawning fails.
pub fn launch_claude_code(
    project_path: &str,
    project_name: &str,
    skip_permissions: bool,
    continue_session: bool,
    terminal: &str,
) -> Result<Child, LauncherError> {
    let term = TerminalType::from_str(terminal);
    let resolved = term.resolve();
    let claude_cmd = build_claude_command(skip_permissions, continue_session);

    #[cfg(target_os = "windows")]
    {
        let (ps1_path, bat_path) =
            create_launch_scripts(project_path, project_name, &claude_cmd)?;

        let child = match resolved {
            TerminalType::WindowsTerminal => {
                launch_in_wt(&ps1_path, project_path, project_name)?
            }
            TerminalType::Cmd => launch_in_cmd(&bat_path)?,
            TerminalType::PowerShell => launch_in_powershell(&ps1_path)?,
            _ => unreachable!("非 Windows 终端类型不会在 Windows 上解析"),
        };

        Ok(child)
    }

    #[cfg(target_os = "macos")]
    {
        let sh_path = create_sh_launch_script(project_path, project_name, &claude_cmd)?;

        let child = match resolved {
            TerminalType::ITerm2 => launch_in_iterm2(&sh_path)?,
            TerminalType::TerminalApp => launch_in_terminal_app(&sh_path)?,
            _ => unreachable!("非 macOS 终端类型不会在 macOS 上解析"),
        };

        Ok(child)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Err(LauncherError::LaunchFailed(
            "不支持的平台".to_string(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Cleanup (cross-platform)
// ---------------------------------------------------------------------------

/// Remove launch scripts from the temp directory.
///
/// Silently ignores errors (e.g. if the files have already been deleted or the
/// terminal is still reading them).
#[cfg(target_os = "windows")]
pub fn cleanup_launch_scripts(ps1_path: &Path, bat_path: &Path) {
    let _ = fs::remove_file(ps1_path);
    let _ = fs::remove_file(bat_path);
}

/// Remove a launch script from the temp directory (macOS).
///
/// Silently ignores errors.
#[cfg(target_os = "macos")]
pub fn cleanup_launch_scripts(sh_path: &Path) {
    let _ = fs::remove_file(sh_path);
}

/// 清理临时目录中所有残留的启动脚本（应用启动时调用）。
pub fn cleanup_old_launch_scripts() {
    let temp_dir = std::env::temp_dir();
    if let Ok(entries) = fs::read_dir(&temp_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("claude-launcher-")
                && (name_str.ends_with(".ps1")
                    || name_str.ends_with(".bat")
                    || name_str.ends_with(".sh"))
            {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // -----------------------------------------------------------------------
    // TerminalType
    // -----------------------------------------------------------------------

    #[test]
    fn test_terminal_type_from_str() {
        assert_eq!(TerminalType::from_str("wt"), TerminalType::WindowsTerminal);
        assert_eq!(TerminalType::from_str("WT"), TerminalType::WindowsTerminal);
        assert_eq!(
            TerminalType::from_str("windows-terminal"),
            TerminalType::WindowsTerminal
        );
        assert_eq!(TerminalType::from_str("cmd"), TerminalType::Cmd);
        assert_eq!(TerminalType::from_str("CMD"), TerminalType::Cmd);
        assert_eq!(
            TerminalType::from_str("powershell"),
            TerminalType::PowerShell
        );
        assert_eq!(
            TerminalType::from_str("PowerShell"),
            TerminalType::PowerShell
        );
        assert_eq!(
            TerminalType::from_str("terminal"),
            TerminalType::TerminalApp
        );
        assert_eq!(
            TerminalType::from_str("Terminal"),
            TerminalType::TerminalApp
        );
        assert_eq!(TerminalType::from_str("iterm2"), TerminalType::ITerm2);
        assert_eq!(TerminalType::from_str("iTerm2"), TerminalType::ITerm2);
        assert_eq!(TerminalType::from_str("auto"), TerminalType::Auto);
        assert_eq!(TerminalType::from_str("invalid"), TerminalType::Auto);
        assert_eq!(TerminalType::from_str(""), TerminalType::Auto);
    }

    #[test]
    fn test_terminal_type_display() {
        assert_eq!(TerminalType::Auto.to_string(), "auto");
        assert_eq!(TerminalType::WindowsTerminal.to_string(), "wt");
        assert_eq!(TerminalType::Cmd.to_string(), "cmd");
        assert_eq!(TerminalType::PowerShell.to_string(), "powershell");
        assert_eq!(TerminalType::TerminalApp.to_string(), "terminal");
        assert_eq!(TerminalType::ITerm2.to_string(), "iterm2");
    }

    #[test]
    fn test_terminal_type_resolve_never_returns_auto() {
        let resolved = TerminalType::Auto.resolve();
        assert_ne!(resolved, TerminalType::Auto);
    }

    #[test]
    fn test_terminal_type_resolve_passthrough() {
        assert_eq!(TerminalType::Cmd.resolve(), TerminalType::Cmd);
        assert_eq!(TerminalType::PowerShell.resolve(), TerminalType::PowerShell);
        assert_eq!(
            TerminalType::WindowsTerminal.resolve(),
            TerminalType::WindowsTerminal
        );
        assert_eq!(TerminalType::TerminalApp.resolve(), TerminalType::TerminalApp);
        assert_eq!(TerminalType::ITerm2.resolve(), TerminalType::ITerm2);
    }

    // -----------------------------------------------------------------------
    // Executable discovery
    // -----------------------------------------------------------------------

    #[test]
    fn test_find_in_path_nonexistent() {
        let result = find_in_path("this_exe_absolutely_does_not_exist_12345.exe");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_wt_exe_returns_option() {
        // May or may not exist depending on the test environment.
        let result = find_wt_exe();
        if let Some(ref p) = result {
            assert!(p.to_string_lossy().contains("wt.exe"));
        }
    }

    #[test]
    fn test_detect_windows_terminal_no_panic() {
        let _ = detect_windows_terminal();
    }

    // -----------------------------------------------------------------------
    // Command building
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_claude_command_no_flags() {
        assert_eq!(build_claude_command(false, false), "claude");
    }

    #[test]
    fn test_build_claude_command_skip_permissions() {
        assert_eq!(
            build_claude_command(true, false),
            "claude --dangerously-skip-permissions"
        );
    }

    #[test]
    fn test_build_claude_command_continue() {
        assert_eq!(build_claude_command(false, true), "claude --continue");
    }

    #[test]
    fn test_build_claude_command_both_flags() {
        assert_eq!(
            build_claude_command(true, true),
            "claude --dangerously-skip-permissions --continue"
        );
    }

    // -----------------------------------------------------------------------
    // Script generation — Windows
    // -----------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    fn test_create_launch_scripts_basic() {
        let (ps1, bat) =
            create_launch_scripts("C:\\Projects\\MyProject", "MyProject", "claude --continue")
                .unwrap();

        // Verify extensions
        assert_eq!(ps1.extension().unwrap(), "ps1");
        assert_eq!(bat.extension().unwrap(), "bat");

        // Verify filenames contain our prefix
        assert!(
            ps1.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("claude-launcher-")
        );
        assert!(
            bat.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("claude-launcher-")
        );

        // --- ps1 content ---
        let ps1_content = fs::read_to_string(&ps1).unwrap();
        assert!(ps1_content.contains("Set-Location"));
        assert!(ps1_content.contains("C:\\Projects\\MyProject"));
        assert!(ps1_content.contains("MyProject - Claude Code"));
        assert!(ps1_content.contains("claude --continue"));

        // --- bat content ---
        let bat_content = fs::read_to_string(&bat).unwrap();
        assert!(bat_content.contains("@echo off"));
        assert!(bat_content.contains("cd /d"));
        assert!(bat_content.contains("C:\\Projects\\MyProject"));
        assert!(bat_content.contains("claude --continue"));

        cleanup_launch_scripts(&ps1, &bat);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_create_launch_scripts_special_chars_in_path() {
        let (ps1, bat) = create_launch_scripts(
            "C:\\Projects\\My Project (2024)",
            "My Project",
            "claude --dangerously-skip-permissions",
        )
        .unwrap();

        let ps1_content = fs::read_to_string(&ps1).unwrap();
        assert!(ps1_content.contains("C:\\Projects\\My Project (2024)"));

        let bat_content = fs::read_to_string(&bat).unwrap();
        assert!(bat_content.contains("C:\\Projects\\My Project (2024)"));

        cleanup_launch_scripts(&ps1, &bat);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_ps1_script_line_structure() {
        let (ps1, bat) = create_launch_scripts("D:\\Dev\\App", "App", "claude").unwrap();

        let binding = fs::read_to_string(&ps1).unwrap();
        let lines: Vec<&str> = binding.lines().collect();
        assert!(lines.len() >= 3);
        assert!(lines[0].contains("Set-Location"));
        assert!(lines[1].contains("WindowTitle"));
        assert!(lines[2].contains("claude"));

        cleanup_launch_scripts(&ps1, &bat);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_bat_script_line_structure() {
        let (ps1, bat) = create_launch_scripts("D:\\Dev\\App", "App", "claude").unwrap();

        let binding = fs::read_to_string(&bat).unwrap();
        let lines: Vec<&str> = binding.lines().collect();
        assert!(lines.len() >= 4);
        assert_eq!(lines[0].trim(), "@echo off");
        assert!(lines[1].starts_with("title"));
        assert!(lines[2].starts_with("cd /d"));
        assert!(lines[3].starts_with("echo Starting"));

        cleanup_launch_scripts(&ps1, &bat);
    }

    // -----------------------------------------------------------------------
    // Script generation — macOS
    // -----------------------------------------------------------------------

    #[cfg(target_os = "macos")]
    #[test]
    fn test_create_sh_launch_script_basic() {
        let sh =
            create_sh_launch_script("/Users/test/MyProject", "MyProject", "claude --continue")
                .unwrap();

        assert_eq!(sh.extension().unwrap(), "sh");
        assert!(
            sh.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("claude-launcher-")
        );

        let content = fs::read_to_string(&sh).unwrap();
        assert!(content.contains("#!/bin/bash"));
        assert!(content.contains("cd \"/Users/test/MyProject\""));
        assert!(content.contains("MyProject - Claude Code"));
        assert!(content.contains("claude --continue"));

        cleanup_launch_scripts(&sh);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_create_sh_launch_script_special_chars() {
        let sh = create_sh_launch_script(
            "/Users/test/My Project (2024)",
            "My Project",
            "claude --dangerously-skip-permissions",
        )
        .unwrap();

        let content = fs::read_to_string(&sh).unwrap();
        assert!(content.contains("/Users/test/My Project (2024)"));

        cleanup_launch_scripts(&sh);
    }

    // -----------------------------------------------------------------------
    // Cleanup
    // -----------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    fn test_cleanup_removes_files() {
        let (ps1, bat) = create_launch_scripts("/tmp/test", "T", "claude").unwrap();

        assert!(ps1.exists());
        assert!(bat.exists());

        cleanup_launch_scripts(&ps1, &bat);

        assert!(!ps1.exists());
        assert!(!bat.exists());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_cleanup_nonexistent_does_not_panic() {
        cleanup_launch_scripts(
            PathBuf::from("/tmp/no_such_file_abc.ps1").as_path(),
            PathBuf::from("/tmp/no_such_file_xyz.bat").as_path(),
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_cleanup_removes_file_macos() {
        let sh = create_sh_launch_script("/tmp/test", "T", "claude").unwrap();
        assert!(sh.exists());
        cleanup_launch_scripts(&sh);
        assert!(!sh.exists());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_cleanup_nonexistent_does_not_panic_macos() {
        cleanup_launch_scripts(PathBuf::from("/tmp/no_such_file_abc.sh").as_path());
    }

    // -----------------------------------------------------------------------
    // Error display
    // -----------------------------------------------------------------------

    #[test]
    fn test_error_messages_contain_context() {
        let err = LauncherError::ExecutableNotFound("wt.exe".into());
        assert!(err.to_string().contains("wt.exe"));

        let err = LauncherError::ScriptCreationFailed("disk full".into());
        assert!(err.to_string().contains("disk full"));

        let err = LauncherError::LaunchFailed("permission denied".into());
        assert!(err.to_string().contains("permission denied"));
    }

    // -----------------------------------------------------------------------
    // Temp path generation
    // -----------------------------------------------------------------------

    #[test]
    fn test_temp_script_path_has_correct_extension() {
        let p = temp_script_path("ps1");
        assert_eq!(p.extension().unwrap(), "ps1");
        assert!(
            p.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("claude-launcher-")
        );

        let p = temp_script_path("bat");
        assert_eq!(p.extension().unwrap(), "bat");

        let p = temp_script_path("sh");
        assert_eq!(p.extension().unwrap(), "sh");
    }

    // -----------------------------------------------------------------------
    // Integration-style: launch_claude_code error paths
    // -----------------------------------------------------------------------

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "spawns real terminal processes — run with: cargo test -- --ignored"]
    fn test_launch_claude_code_invalid_project_path_does_not_panic() {
        let result = launch_claude_code(
            "Z:\\nonexistent\\project",
            "TestProj",
            false,
            false,
            "powershell",
        );
        match result {
            Ok(_child) => {}
            Err(LauncherError::LaunchFailed(msg)) => {
                assert!(msg.len() > 0);
            }
            Err(LauncherError::ExecutableNotFound(_)) => {}
            Err(other) => {
                panic!("Unexpected error: {}", other);
            }
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "spawns real terminal processes — run with: cargo test -- --ignored"]
    fn test_launch_claude_code_auto_terminal() {
        let result = launch_claude_code("Z:\\nonexistent\\project", "AutoTest", true, true, "auto");
        match result {
            Ok(_)
            | Err(LauncherError::LaunchFailed(_))
            | Err(LauncherError::ExecutableNotFound(_)) => {}
            Err(other) => panic!("Unexpected error: {}", other),
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "spawns real terminal processes — run with: cargo test -- --ignored"]
    fn test_launch_claude_code_cmd_terminal() {
        let result = launch_claude_code("Z:\\nonexistent", "CmdTest", false, false, "cmd");
        match result {
            Ok(_) | Err(LauncherError::LaunchFailed(_)) => {}
            Err(other) => panic!("Unexpected error: {}", other),
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "spawns real terminal processes — run with: cargo test -- --ignored"]
    fn test_launch_claude_code_macos_terminal() {
        let result = launch_claude_code(
            "/nonexistent/project",
            "TestProj",
            false,
            false,
            "terminal",
        );
        match result {
            Ok(_) | Err(LauncherError::LaunchFailed(_)) => {}
            Err(other) => panic!("Unexpected error: {}", other),
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "spawns real terminal processes — run with: cargo test -- --ignored"]
    fn test_launch_claude_code_macos_auto() {
        let result = launch_claude_code("/nonexistent/project", "AutoTest", true, true, "auto");
        match result {
            Ok(_) | Err(LauncherError::LaunchFailed(_)) => {}
            Err(other) => panic!("Unexpected error: {}", other),
        }
    }
}
