use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

use crate::cli_config::types::CliConfigTargetKind;

pub const DATA_FILE: &str = "tma_endpoints.json";
pub const TEST_SETTINGS_FILE: &str = "test_settings.json";
pub const APP_SETTINGS_FILE: &str = "settings.json";
pub const CLI_APPLY_HISTORY_FILE: &str = "cli-config-apply-history.json";

pub fn store_path(app: &tauri::AppHandle, file_name: &str) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let path = dir.join(file_name);
    let legacy_path = Path::new(file_name);
    if !path.exists() && legacy_path.exists() {
        fs::copy(legacy_path, &path).map_err(|err| err.to_string())?;
    }
    Ok(path)
}

pub fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = user_home_dir()
        .or_else(|_| app.path().app_data_dir().map_err(|err| err.to_string()))?
        .join(".TestModelAlive");
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir)
}

pub fn user_home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| "HOME/USERPROFILE is not set".to_string())
}

pub fn cli_target_files(
    target: &CliConfigTargetKind,
) -> Result<Vec<(String, PathBuf, String)>, String> {
    let home = user_home_dir()?;
    match target {
        CliConfigTargetKind::Codex => Ok(vec![
            (
                "codex-auth".to_string(),
                home.join(".codex").join("auth.json"),
                "json".to_string(),
            ),
            (
                "codex-config".to_string(),
                home.join(".codex").join("config.toml"),
                "toml".to_string(),
            ),
        ]),
        CliConfigTargetKind::Claude => Ok(vec![
            (
                "claude-settings".to_string(),
                home.join(".claude").join("settings.json"),
                "json".to_string(),
            ),
            (
                "claude-state".to_string(),
                home.join(".claude.json"),
                "json".to_string(),
            ),
        ]),
        CliConfigTargetKind::Opencode => Ok(vec![(
            "opencode-config".to_string(),
            opencode_config_path()?,
            "json".to_string(),
        )]),
    }
}

pub fn opencode_config_path() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        return Ok(user_home_dir()?
            .join(".config")
            .join("opencode")
            .join("opencode.json"));
    }
    #[cfg(target_os = "macos")]
    {
        return Ok(user_home_dir()?
            .join("Library")
            .join("Application Support")
            .join("opencode")
            .join("opencode.json"));
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or(user_home_dir()?.join(".config"));
        Ok(base.join("opencode").join("opencode.json"))
    }
}
