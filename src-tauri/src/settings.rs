use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

use crate::models::{
    default_opencode_sdk_package, SavedEndpoint, TestSettings, DEFAULT_PROMPT,
    DEFAULT_SUCCESS_KEYWORD,
};
use crate::paths::{app_data_dir, store_path, APP_SETTINGS_FILE, CLI_APPLY_HISTORY_FILE};

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
    #[serde(default = "default_apply_history_limit")]
    pub apply_history_limit: usize,
    pub baseline_items: Vec<CliConfigBaselineItem>,
    #[serde(default, skip_serializing)]
    pub apply_history: Vec<CliConfigApplyHistoryItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CliConfigApplyHistoryStore {
    #[serde(default = "default_apply_history_limit")]
    pub limit: usize,
    #[serde(default)]
    pub items: Vec<CliConfigApplyHistoryItem>,
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
    let mut removed_opencode_variants = false;
    let mut settings = if path.exists() {
        let text = fs::read_to_string(&path).map_err(|err| err.to_string())?;
        let mut value: Value = serde_json::from_str(&text).map_err(|err| err.to_string())?;
        removed_opencode_variants = remove_opencode_model_variants(&mut value);
        serde_json::from_value::<AppSettings>(value).map_err(|err| err.to_string())?
    } else {
        default_app_settings(app)?
    };
    let migrated_apply_history = migrate_apply_history_out_of_settings(app, &mut settings)?;
    let normalized_settings = normalize_app_settings(app, &mut settings)?;
    if !path.exists() || removed_opencode_variants || migrated_apply_history || normalized_settings
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

pub fn read_apply_history_store(
    app: &tauri::AppHandle,
) -> Result<CliConfigApplyHistoryStore, String> {
    let path = store_path(app, CLI_APPLY_HISTORY_FILE)?;
    if !path.exists() {
        return Ok(default_apply_history_store());
    }
    let text = fs::read_to_string(&path).map_err(|err| err.to_string())?;
    let mut store: CliConfigApplyHistoryStore =
        serde_json::from_str(&text).map_err(|err| err.to_string())?;
    if store.limit == 0 {
        store.limit = default_apply_history_limit();
    }
    trim_apply_history_store(&mut store);
    Ok(store)
}

pub fn write_apply_history_store(
    app: &tauri::AppHandle,
    store: &CliConfigApplyHistoryStore,
) -> Result<(), String> {
    let path = store_path(app, CLI_APPLY_HISTORY_FILE)?;
    let text = serde_json::to_string_pretty(store).map_err(|err| err.to_string())?;
    write_text_file(&path, &format!("{text}\n"))
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

pub fn timestamp() -> String {
    OffsetDateTime::now_utc().unix_timestamp_nanos().to_string()
}

fn default_settings_version() -> u8 {
    1
}

fn default_cli_config_settings() -> CliConfigSettings {
    CliConfigSettings {
        baseline_id: timestamp_id("baseline"),
        backup_root: String::new(),
        apply_history_limit: default_apply_history_limit(),
        baseline_items: Vec::new(),
        apply_history: Vec::new(),
    }
}

fn default_apply_history_store() -> CliConfigApplyHistoryStore {
    CliConfigApplyHistoryStore {
        limit: default_apply_history_limit(),
        items: Vec::new(),
    }
}

fn default_apply_history_limit() -> usize {
    20
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
            apply_history_limit: default_apply_history_limit(),
            baseline_items: Vec::new(),
            apply_history: Vec::new(),
        },
    })
}

fn normalize_app_settings(
    app: &tauri::AppHandle,
    settings: &mut AppSettings,
) -> Result<bool, String> {
    let mut changed = false;
    settings.endpoints.retain(|endpoint| {
        !endpoint.id.is_empty()
            && is_valid_endpoint_name(&endpoint.name)
            && is_valid_endpoint_type(&endpoint.endpoint_type)
            && !endpoint.endpoint_type.is_empty()
            && !endpoint.base_url.is_empty()
    });
    for endpoint in &mut settings.endpoints {
        if endpoint.endpoint_type != "opencode" {
            endpoint.opencode_sdk_package = default_opencode_sdk_package();
        } else if !is_valid_opencode_sdk_package(&endpoint.opencode_sdk_package) {
            endpoint.opencode_sdk_package = default_opencode_sdk_package();
            changed = true;
        }
    }
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
    if settings.cli_config.apply_history_limit == 0 {
        settings.cli_config.apply_history_limit = default_apply_history_limit();
    }
    Ok(changed)
}

pub fn trim_apply_history_store(store: &mut CliConfigApplyHistoryStore) {
    if store.limit > 0 && store.items.len() > store.limit {
        let drop_count = store.items.len() - store.limit;
        store.items.drain(0..drop_count);
    }
}

fn migrate_apply_history_out_of_settings(
    app: &tauri::AppHandle,
    settings: &mut AppSettings,
) -> Result<bool, String> {
    if settings.cli_config.apply_history.is_empty() {
        return Ok(false);
    }
    let mut store = read_apply_history_store(app)?;
    if store.items.is_empty() {
        store.items = std::mem::take(&mut settings.cli_config.apply_history);
    } else {
        store.items.append(&mut settings.cli_config.apply_history);
    }
    store.limit = settings.cli_config.apply_history_limit;
    trim_apply_history_store(&mut store);
    write_apply_history_store(app, &store)?;
    Ok(true)
}

fn normalize_test_settings(settings: &mut TestSettings) {
    if settings.success_keyword.trim().is_empty() {
        settings.success_keyword = DEFAULT_SUCCESS_KEYWORD.to_string();
    }
    if settings.prompt.trim().is_empty() {
        settings.prompt = DEFAULT_PROMPT.to_string();
    }
}

fn remove_opencode_model_variants(value: &mut Value) -> bool {
    value
        .as_object_mut()
        .and_then(|object| object.remove("opencode_model_variants"))
        .is_some()
}

fn is_valid_endpoint_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|ch| ch.is_ascii_alphanumeric())
}

fn is_valid_endpoint_type(endpoint_type: &str) -> bool {
    matches!(endpoint_type, "codex" | "claude" | "opencode" | "deepseek")
}

fn is_valid_opencode_sdk_package(package: &str) -> bool {
    matches!(package, "@ai-sdk/openai" | "@ai-sdk/openai-compatible")
}

#[allow(dead_code)]
fn _path_text(path: PathBuf) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use crate::model_metadata::opencode_model_variants;
    use serde_json::json;

    use super::remove_opencode_model_variants;

    #[test]
    fn requested_models_use_advertised_reasoning_variants() {
        let variants = opencode_model_variants();
        let generic = json!({
            "low": { "reasoningEffort": "low" },
            "medium": { "reasoningEffort": "medium" },
            "high": { "reasoningEffort": "high" }
        });
        let north = json!({
            "none": { "reasoningEffort": "none" },
            "high": { "reasoningEffort": "high" }
        });

        for model in ["hy3", "laguna-s-2.1", "ling-3.0-flash"] {
            assert_eq!(variants.get(model), Some(&generic));
        }
        assert_eq!(variants.get("north-mini-code"), Some(&north));
        for model in [
            "ling-3.0-tiny",
            "mimo-v2.5",
            "nemotron-3-ultra",
            "nemotron-3.5-lightning",
        ] {
            assert_eq!(variants.get(model), Some(&json!({})));
        }
    }

    #[test]
    fn legacy_settings_drop_persisted_model_variants() {
        let mut settings = json!({
            "opencode_model_variants": { "deepseek-v4-flash": { "max": {} } },
            "endpoints": []
        });

        assert!(remove_opencode_model_variants(&mut settings));
        assert!(settings.get("opencode_model_variants").is_none());
    }
}
