/// Backup Manager - manages backup/restore of Claude Code settings.json.
///
/// Handles the Claude Code configuration directory (~/.claude),
/// providing backup, disable, and restore operations for settings.json.
/// Behavior matches the Python source exactly.
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::core::models::BackupStatus;

/// Errors for backup management operations.
#[derive(Debug, Error)]
pub enum BackupError {
    #[error("备份文件不存在: {0}")]
    BackupNotFound(String),

    #[error("设置文件不存在: {0}")]
    SettingsNotFound(String),

    #[error("无法失效设置文件：备份文件不存在 ({0})\n请先创建备份")]
    CannotDisableWithoutBackup(String),

    #[error("无法恢复设置文件：当前设置文件已存在 ({0})\n请先失效当前设置")]
    CannotRestoreWhileSettingsExist(String),

    #[error("设置文件已禁用")]
    SettingsDisabled,

    #[error("读取文件失败: {0}")]
    ReadError(String),

    #[error("写入文件失败: {0}")]
    WriteError(String),

    #[error("无法获取用户主目录")]
    HomeDirNotFound,
}

/// Settings file name.
const SETTINGS_FILE: &str = "settings.json";

/// Backup file name.
const BACKUP_FILE: &str = "settings.json.bak";

/// Disabled settings file name.
const DISABLED_FILE: &str = "settings.json.disabled";

/// Manages backup, disable, and restore operations for Claude Code settings.
pub struct BackupManager {
    claude_dir: PathBuf,
}

impl BackupManager {
    /// Creates a new BackupManager with the given Claude directory path.
    pub fn new(claude_dir: PathBuf) -> Self {
        Self { claude_dir }
    }

    /// Creates a BackupManager with the default Claude directory `~/.claude`.
    pub fn default_manager() -> Result<Self, BackupError> {
        let home = dirs::home_dir().ok_or(BackupError::HomeDirNotFound)?;
        Ok(Self {
            claude_dir: home.join(".claude"),
        })
    }

    /// Returns the Claude directory path.
    pub fn claude_dir(&self) -> &Path {
        &self.claude_dir
    }

    // ── File paths ────────────────────────────────────────────────

    fn settings_path(&self) -> PathBuf {
        self.claude_dir.join(SETTINGS_FILE)
    }

    fn backup_path(&self) -> PathBuf {
        self.claude_dir.join(BACKUP_FILE)
    }

    fn disabled_path(&self) -> PathBuf {
        self.claude_dir.join(DISABLED_FILE)
    }

    // ── Status ────────────────────────────────────────────────────

    /// Gets the current backup status.
    pub fn get_status(&self) -> BackupStatus {
        BackupStatus {
            settings_exists: self.settings_path().exists(),
            settings_disabled: self.disabled_path().exists(),
            backup_exists: self.backup_path().exists(),
        }
    }

    // ── Read / Write settings ─────────────────────────────────────

    /// Reads the current settings.json content, if it exists.
    pub fn read_settings(&self) -> Option<String> {
        let path = self.settings_path();
        if !path.exists() {
            return None;
        }
        std::fs::read_to_string(&path).ok()
    }

    /// Writes content to settings.json, creating the directory if needed.
    pub fn write_settings(&self, content: &str) -> Result<(), BackupError> {
        if !self.claude_dir.exists() {
            std::fs::create_dir_all(&self.claude_dir).map_err(|e| {
                BackupError::WriteError(format!("无法创建目录 {:?}: {}", self.claude_dir, e))
            })?;
        }
        std::fs::write(self.settings_path(), content).map_err(|e| {
            BackupError::WriteError(format!("无法写入 {:?}: {}", self.settings_path(), e))
        })?;
        Ok(())
    }

    // ── Backup operations ─────────────────────────────────────────

    /// Performs an automatic backup on first run.
    ///
    /// If settings.json exists and no backup file exists, copies
    /// settings.json to settings.json.bak. Returns `true` if a backup
    /// was created.
    pub fn auto_backup_on_first_run(&self) -> bool {
        let settings = self.settings_path();
        let backup = self.backup_path();

        if settings.exists() && !backup.exists() {
            match std::fs::copy(&settings, &backup) {
                Ok(_) => true,
                Err(e) => {
                    tracing::warn!("自动备份失败: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    /// Creates a backup of settings.json to settings.json.bak.
    ///
    /// Overwrites any existing backup. Errors if settings.json doesn't exist.
    pub fn backup(&self) -> Result<(), BackupError> {
        let settings = self.settings_path();
        let backup = self.backup_path();

        if !settings.exists() {
            return Err(BackupError::SettingsNotFound(format!(
                "设置文件不存在: {}",
                settings.display()
            )));
        }

        std::fs::copy(&settings, &backup).map_err(|e| {
            BackupError::WriteError(format!(
                "备份失败 ({} -> {}): {}",
                settings.display(),
                backup.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Disables settings.json by renaming it to settings.json.disabled.
    ///
    /// **Requires** a backup file to exist first (matching Python behavior).
    /// Returns `false` if settings.json is already gone.
    pub fn disable(&self) -> Result<bool, BackupError> {
        let settings = self.settings_path();
        let disabled = self.disabled_path();
        let backup = self.backup_path();

        if !backup.exists() {
            return Err(BackupError::CannotDisableWithoutBackup(format!(
                "{}",
                backup.display()
            )));
        }

        if !settings.exists() {
            return Ok(false);
        }

        std::fs::rename(&settings, &disabled).map_err(|e| {
            BackupError::WriteError(format!(
                "禁用设置失败 ({} -> {}): {}",
                settings.display(),
                disabled.display(),
                e
            ))
        })?;

        Ok(true)
    }

    /// Restores settings.json from backup (settings.json.bak).
    ///
    /// **Errors** if settings.json currently exists (must disable first).
    /// Copies backup to settings.json and removes .disabled if present.
    pub fn restore(&self) -> Result<bool, BackupError> {
        let backup = self.backup_path();
        let settings = self.settings_path();
        let disabled = self.disabled_path();

        if settings.exists() {
            return Err(BackupError::CannotRestoreWhileSettingsExist(format!(
                "{}",
                settings.display()
            )));
        }

        if !backup.exists() {
            return Ok(false);
        }

        std::fs::copy(&backup, &settings)
            .map_err(|e| BackupError::WriteError(format!("恢复设置失败: {}", e)))?;

        // Clean up .disabled file if it exists
        if disabled.exists() {
            let _ = std::fs::remove_file(&disabled);
        }

        Ok(true)
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_manager(dir: &std::path::Path) -> BackupManager {
        BackupManager::new(dir.to_path_buf())
    }

    fn write_settings_file(dir: &std::path::Path, content: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(SETTINGS_FILE), content).unwrap();
    }

    #[test]
    fn test_get_status_no_files() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        let status = mgr.get_status();
        assert!(!status.settings_exists);
        assert!(!status.settings_disabled);
        assert!(!status.backup_exists);
    }

    #[test]
    fn test_get_status_with_settings() {
        let dir = tempdir().unwrap();
        write_settings_file(dir.path(), r#"{"key": "value"}"#);
        let mgr = make_manager(dir.path());

        let status = mgr.get_status();
        assert!(status.settings_exists);
        assert!(!status.settings_disabled);
        assert!(!status.backup_exists);
    }

    #[test]
    fn test_read_settings() {
        let dir = tempdir().unwrap();
        let content = r#"{"test": true}"#;
        write_settings_file(dir.path(), content);
        let mgr = make_manager(dir.path());

        assert_eq!(mgr.read_settings().unwrap(), content);
    }

    #[test]
    fn test_read_settings_no_file() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());
        assert!(mgr.read_settings().is_none());
    }

    #[test]
    fn test_write_settings() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        mgr.write_settings(r#"{"written": true}"#).unwrap();
        assert_eq!(mgr.read_settings().unwrap(), r#"{"written": true}"#);
    }

    #[test]
    fn test_write_settings_when_disabled_exists() {
        // #22: write_settings should work when .disabled file exists but settings.json doesn't
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path()).unwrap();
        // Create only the .disabled file, no settings.json
        std::fs::write(dir.path().join(DISABLED_FILE), r#"{"old": true}"#).unwrap();
        let mgr = make_manager(dir.path());

        assert!(mgr.write_settings(r#"{"new": true}"#).is_ok());

        let content = std::fs::read_to_string(dir.path().join(SETTINGS_FILE)).unwrap();
        assert_eq!(content, r#"{"new": true}"#);
        // .disabled file should still exist (write_settings doesn't touch it)
        assert!(dir.path().join(DISABLED_FILE).exists());
    }

    #[test]
    fn test_write_settings_creates_dir() {
        let dir = tempdir().unwrap();
        let claude_dir = dir.path().join("subdir");
        let mgr = BackupManager::new(claude_dir.clone());

        mgr.write_settings(r#"{"auto_created": true}"#).unwrap();
        assert!(claude_dir.exists());
    }

    #[test]
    fn test_auto_backup_on_first_run() {
        let dir = tempdir().unwrap();
        write_settings_file(dir.path(), r#"{"original": true}"#);
        let mgr = make_manager(dir.path());

        assert!(mgr.auto_backup_on_first_run());
        assert!(mgr.get_status().backup_exists);

        // Second call should not create a new backup
        assert!(!mgr.auto_backup_on_first_run());
    }

    #[test]
    fn test_auto_backup_no_settings() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());
        assert!(!mgr.auto_backup_on_first_run());
    }

    #[test]
    fn test_backup() {
        let dir = tempdir().unwrap();
        write_settings_file(dir.path(), r#"{"backup_test": true}"#);
        let mgr = make_manager(dir.path());

        mgr.backup().unwrap();
        let backup_content = std::fs::read_to_string(dir.path().join(BACKUP_FILE)).unwrap();
        assert_eq!(backup_content, r#"{"backup_test": true}"#);
    }

    #[test]
    fn test_backup_no_settings() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());
        assert!(mgr.backup().is_err());
    }

    #[test]
    fn test_backup_overwrite() {
        let dir = tempdir().unwrap();
        write_settings_file(dir.path(), r#"{"version": 2}"#);
        let mgr = make_manager(dir.path());

        mgr.backup().unwrap();
        write_settings_file(dir.path(), r#"{"version": 3}"#);
        mgr.backup().unwrap();

        let backup_content = std::fs::read_to_string(dir.path().join(BACKUP_FILE)).unwrap();
        assert_eq!(backup_content, r#"{"version": 3}"#);
    }

    #[test]
    fn test_disable_requires_backup() {
        let dir = tempdir().unwrap();
        write_settings_file(dir.path(), r#"{"test": true}"#);
        let mgr = make_manager(dir.path());

        // Should error because no backup exists
        let err = mgr.disable().unwrap_err();
        assert!(err.to_string().contains("备份文件不存在"));
    }

    #[test]
    fn test_disable_after_backup() {
        let dir = tempdir().unwrap();
        write_settings_file(dir.path(), r#"{"disable_test": true}"#);
        let mgr = make_manager(dir.path());

        mgr.backup().unwrap();
        assert!(mgr.disable().unwrap());

        let status = mgr.get_status();
        assert!(!status.settings_exists);
        assert!(status.settings_disabled);
        assert!(status.backup_exists);
    }

    #[test]
    fn test_disable_no_settings_returns_false() {
        let dir = tempdir().unwrap();
        // Create only a backup, no settings
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(dir.path().join(BACKUP_FILE), "{}").unwrap();
        let mgr = make_manager(dir.path());

        // Returns Ok(false) since settings.json doesn't exist
        assert!(!mgr.disable().unwrap());
    }

    #[test]
    fn test_restore_errors_if_settings_exist() {
        let dir = tempdir().unwrap();
        write_settings_file(dir.path(), r#"{"current": true}"#);
        std::fs::write(dir.path().join(BACKUP_FILE), r#"{"old": true}"#).unwrap();
        let mgr = make_manager(dir.path());

        // Should error because settings.json exists
        let err = mgr.restore().unwrap_err();
        assert!(err.to_string().contains("当前设置文件已存在"));
    }

    #[test]
    fn test_restore_after_disable() {
        let dir = tempdir().unwrap();
        write_settings_file(dir.path(), r#"{"original": true}"#);
        let mgr = make_manager(dir.path());

        mgr.backup().unwrap();
        mgr.disable().unwrap();
        assert!(mgr.restore().unwrap());

        let restored = mgr.read_settings().unwrap();
        assert_eq!(restored, r#"{"original": true}"#);
    }

    #[test]
    fn test_restore_no_backup() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());
        // Returns Ok(false) since backup doesn't exist
        assert!(!mgr.restore().unwrap());
    }

    #[test]
    fn test_restore_clears_disabled() {
        let dir = tempdir().unwrap();
        let mgr = make_manager(dir.path());

        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(dir.path().join(BACKUP_FILE), r#"{"restored": true}"#).unwrap();
        std::fs::write(dir.path().join(DISABLED_FILE), r#"{"old": true}"#).unwrap();

        mgr.restore().unwrap();

        let status = mgr.get_status();
        assert!(status.settings_exists);
        assert!(!status.settings_disabled);
        assert_eq!(mgr.read_settings().unwrap(), r#"{"restored": true}"#);
    }

    #[test]
    fn test_full_cycle() {
        let dir = tempdir().unwrap();
        let original = r#"{"full_cycle": true, "version": 1}"#;
        write_settings_file(dir.path(), original);
        let mgr = make_manager(dir.path());

        // 1. Backup + disable
        mgr.backup().unwrap();
        mgr.disable().unwrap();
        assert!(!mgr.settings_path().exists());
        assert!(mgr.disabled_path().exists());
        assert!(mgr.backup_path().exists());

        // 2. Restore
        mgr.restore().unwrap();
        assert!(mgr.settings_path().exists());
        assert!(!mgr.disabled_path().exists());
        assert_eq!(mgr.read_settings().unwrap(), original);
    }

    #[test]
    fn test_status_text() {
        let status = BackupStatus {
            settings_exists: true,
            settings_disabled: false,
            backup_exists: false,
        };
        assert_eq!(status.status_text(), "正常");

        let status = BackupStatus {
            settings_exists: false,
            settings_disabled: true,
            backup_exists: false,
        };
        assert_eq!(status.status_text(), "已失效");

        let status = BackupStatus {
            settings_exists: false,
            settings_disabled: false,
            backup_exists: false,
        };
        assert_eq!(status.status_text(), "无文件");
    }
}
