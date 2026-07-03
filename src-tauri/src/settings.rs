use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

use crate::models::{
    EndpointStore, SavedEndpoint, TestSettings, DEFAULT_PROMPT, DEFAULT_SUCCESS_KEYWORD,
};
use crate::paths::{app_data_dir, store_path, APP_SETTINGS_FILE, DATA_FILE, TEST_SETTINGS_FILE};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppSettings {
    #[serde(default = "default_settings_version")]
    pub version: u8,
    #[serde(default)]
    pub endpoints: Vec<SavedEndpoint>,
    #[serde(default = "default_test_settings")]
    pub test_settings: TestSettings,
    #[serde(default = "default_cli_config_settings")]
    pub cli_config: CliConfigSettings,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CliConfigSettings {
    pub baseline_id: String,
    pub backup_root: String,
    pub baseline_items: Vec<CliConfigBaselineItem>,
    pub apply_history: Vec<CliConfigApplyHistoryItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CliConfigBaselineItem {
    pub target: String,
    pub file_id: String,
    pub path: String,
    pub existed_before: bool,
    pub backup_path: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CliConfigApplyHistoryItem {
    pub apply_id: String,
    pub target: String,
    pub endpoint_id: String,
    pub created_at: String,
    pub backup_paths: Vec<String>,
    pub files: Vec<CliConfigApplyHistoryFile>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CliConfigApplyHistoryFile {
    pub file_id: String,
    pub path: String,
    pub action: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CliConfigBaselineView {
    pub target: String,
    pub file_id: String,
    pub path: String,
    pub existed_before: bool,
    pub created_at: String,
}

#[tauri::command]
pub fn load_test_settings(app: tauri::AppHandle) -> Result<TestSettings, String> {
    let mut settings = read_app_settings(&app)?.test_settings;
    normalize_test_settings(&mut settings);
    Ok(settings)
}

#[tauri::command]
pub fn save_test_settings(app: tauri::AppHandle, settings: TestSettings) -> Result<(), String> {
    let prompt = settings.prompt.trim().to_string();
    let success_keyword = settings.success_keyword.trim().to_string();
    if prompt.is_empty() {
        return Err("prompt is required".to_string());
    }
    if success_keyword.is_empty() {
        return Err("success keyword is required".to_string());
    }
    if !prompt.contains(&success_keyword) {
        return Err("prompt must contain success keyword".to_string());
    }
    let mut app_settings = read_app_settings(&app)?;
    app_settings.test_settings = TestSettings {
        prompt,
        success_keyword,
    };
    write_app_settings_for_app(&app, &app_settings)
}

pub fn default_test_settings() -> TestSettings {
    TestSettings {
        prompt: DEFAULT_PROMPT.to_string(),
        success_keyword: DEFAULT_SUCCESS_KEYWORD.to_string(),
    }
}

pub fn read_app_settings(app: &tauri::AppHandle) -> Result<AppSettings, String> {
    let path = store_path(app, APP_SETTINGS_FILE)?;
    let mut settings = if path.exists() {
        let text = fs::read_to_string(&path).map_err(|err| err.to_string())?;
        serde_json::from_str::<AppSettings>(&text).map_err(|err| err.to_string())?
    } else {
        default_app_settings(app)?
    };
    migrate_legacy_into_settings(app, &mut settings)?;
    normalize_app_settings(app, &mut settings)?;
    if !path.exists()
        || settings.endpoints.is_empty()
        || is_default_test_settings(&settings.test_settings)
    {
        write_app_settings(&path, &settings)?;
    }
    Ok(settings)
}

pub fn write_app_settings_for_app(
    app: &tauri::AppHandle,
    settings: &AppSettings,
) -> Result<(), String> {
    write_app_settings(&store_path(app, APP_SETTINGS_FILE)?, settings)
}

pub fn write_app_settings(path: &Path, settings: &AppSettings) -> Result<(), String> {
    let text = serde_json::to_string_pretty(settings).map_err(|err| err.to_string())?;
    write_text_file(path, &format!("{text}\n"))
}

pub fn write_text_file(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let tmp = path.with_file_name(format!(
        "{}.tmp-{}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    {
        let mut file = fs::File::create(&tmp).map_err(|err| err.to_string())?;
        file.write_all(content.as_bytes())
            .map_err(|err| err.to_string())?;
        file.sync_all().map_err(|err| err.to_string())?;
    }
    if path.exists() {
        fs::remove_file(path).map_err(|err| err.to_string())?;
    }
    fs::rename(&tmp, path).map_err(|err| err.to_string())
}

pub fn timestamp_id(prefix: &str) -> String {
    format!(
        "{prefix}-{}",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    )
}

fn default_settings_version() -> u8 {
    1
}

fn default_cli_config_settings() -> CliConfigSettings {
    CliConfigSettings {
        baseline_id: timestamp_id("baseline"),
        backup_root: String::new(),
        baseline_items: Vec::new(),
        apply_history: Vec::new(),
    }
}

fn default_app_settings(app: &tauri::AppHandle) -> Result<AppSettings, String> {
    Ok(AppSettings {
        version: 1,
        endpoints: Vec::new(),
        test_settings: default_test_settings(),
        cli_config: CliConfigSettings {
            baseline_id: timestamp_id("baseline"),
            backup_root: app_data_dir(app)?
                .join("cli-config-backups")
                .to_string_lossy()
                .to_string(),
            baseline_items: Vec::new(),
            apply_history: Vec::new(),
        },
    })
}

fn normalize_app_settings(
    app: &tauri::AppHandle,
    settings: &mut AppSettings,
) -> Result<(), String> {
    settings.endpoints.retain(|endpoint| {
        !endpoint.id.is_empty()
            && !endpoint.endpoint_type.is_empty()
            && !endpoint.base_url.is_empty()
    });
    normalize_test_settings(&mut settings.test_settings);
    if settings.cli_config.baseline_id.trim().is_empty() {
        settings.cli_config.baseline_id = timestamp_id("baseline");
    }
    if settings.cli_config.backup_root.trim().is_empty() {
        settings.cli_config.backup_root = app_data_dir(app)?
            .join("cli-config-backups")
            .to_string_lossy()
            .to_string();
    }
    Ok(())
}

fn normalize_test_settings(settings: &mut TestSettings) {
    if settings.success_keyword.trim().is_empty() {
        settings.success_keyword = DEFAULT_SUCCESS_KEYWORD.to_string();
    }
    if settings.prompt.trim().is_empty() {
        settings.prompt = DEFAULT_PROMPT.to_string();
    }
}

fn migrate_legacy_into_settings(
    app: &tauri::AppHandle,
    settings: &mut AppSettings,
) -> Result<(), String> {
    if settings.endpoints.is_empty() {
        if let Some(store) = read_legacy_endpoint_store(app)? {
            settings.endpoints = store
                .endpoints
                .into_iter()
                .filter(|endpoint| {
                    !endpoint.id.is_empty()
                        && !endpoint.endpoint_type.is_empty()
                        && !endpoint.base_url.is_empty()
                })
                .collect();
        }
    }
    if is_default_test_settings(&settings.test_settings) {
        if let Some(mut test_settings) = read_legacy_test_settings(app)? {
            normalize_test_settings(&mut test_settings);
            settings.test_settings = test_settings;
        }
    }
    Ok(())
}

fn read_legacy_endpoint_store(app: &tauri::AppHandle) -> Result<Option<EndpointStore>, String> {
    let path = store_path(app, DATA_FILE)?;
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
    Ok(Some(
        serde_json::from_str(&text).map_err(|err| err.to_string())?,
    ))
}

fn read_legacy_test_settings(app: &tauri::AppHandle) -> Result<Option<TestSettings>, String> {
    let path = store_path(app, TEST_SETTINGS_FILE)?;
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
    Ok(Some(
        serde_json::from_str(&text).map_err(|err| err.to_string())?,
    ))
}

fn is_default_test_settings(settings: &TestSettings) -> bool {
    settings.prompt == DEFAULT_PROMPT && settings.success_keyword == DEFAULT_SUCCESS_KEYWORD
}

#[allow(dead_code)]
fn _path_text(path: PathBuf) -> String {
    path.to_string_lossy().to_string()
}
