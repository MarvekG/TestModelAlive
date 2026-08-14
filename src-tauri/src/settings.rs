use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
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
    #[serde(default = "default_opencode_model_variants")]
    pub opencode_model_variants: BTreeMap<String, Value>,
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
    let mut missing_opencode_variants = false;
    let mut settings = if path.exists() {
        let text = fs::read_to_string(&path).map_err(|err| err.to_string())?;
        missing_opencode_variants = !text.contains("\"opencode_model_variants\"");
        serde_json::from_str::<AppSettings>(&text).map_err(|err| err.to_string())?
    } else {
        default_app_settings(app)?
    };
    let migrated_apply_history = migrate_apply_history_out_of_settings(app, &mut settings)?;
    let normalized_settings = normalize_app_settings(app, &mut settings)?;
    if !path.exists() || missing_opencode_variants || migrated_apply_history || normalized_settings
    {
        write_app_settings(&path, &settings)?;
    }
    Ok(settings)
}

pub fn read_opencode_model_variants(
    app: &tauri::AppHandle,
) -> Result<BTreeMap<String, Value>, String> {
    Ok(read_app_settings(app)?.opencode_model_variants)
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

fn default_opencode_model_variants() -> BTreeMap<String, Value> {
    let gpt_variants = serde_json::json!({
        "none": {},
        "low": {
            "reasoningEffort": "low",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        },
        "medium": {
            "reasoningEffort": "medium",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        },
        "high": {
            "reasoningEffort": "high",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        },
        "xhigh": {
            "reasoningEffort": "xhigh",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        }
    });
    let gpt_pro_variants = serde_json::json!({
        "medium": {
            "reasoningEffort": "medium",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        },
        "high": {
            "reasoningEffort": "high",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        },
        "xhigh": {
            "reasoningEffort": "xhigh",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        }
    });
    let gpt_56_variants = serde_json::json!({
        "low": {
            "reasoningEffort": "low",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        },
        "medium": {
            "reasoningEffort": "medium",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        },
        "high": {
            "reasoningEffort": "high",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        },
        "xhigh": {
            "reasoningEffort": "xhigh",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        },
        "max": {
            "reasoningEffort": "max",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        },
        "ultra": {
            "reasoningEffort": "ultra",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        }
    });
    let gpt_56_luna_variants = serde_json::json!({
        "low": {
            "reasoningEffort": "low",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        },
        "medium": {
            "reasoningEffort": "medium",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        },
        "high": {
            "reasoningEffort": "high",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        },
        "xhigh": {
            "reasoningEffort": "xhigh",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        },
        "max": {
            "reasoningEffort": "max",
            "textVerbosity": "low",
            "reasoningSummary": "auto"
        }
    });
    let generic_reasoning_variants = serde_json::json!({
        "low": { "reasoningEffort": "low" },
        "medium": { "reasoningEffort": "medium" },
        "high": { "reasoningEffort": "high" }
    });
    let deepseek_v4_variants = serde_json::json!({
        "low": { "reasoningEffort": "low" },
        "medium": { "reasoningEffort": "medium" },
        "high": { "reasoningEffort": "high" },
        "max": { "reasoningEffort": "max" }
    });
    let glm_variants = serde_json::json!({
        "high": { "reasoningEffort": "high" },
        "max": { "reasoningEffort": "max" }
    });
    let minimax_m3_variants = serde_json::json!({
        "none": { "thinking": { "type": "disabled" } },
        "thinking": { "thinking": { "type": "adaptive" } }
    });
    let north_mini_code_variants = serde_json::json!({
        "none": { "reasoningEffort": "none" },
        "high": { "reasoningEffort": "high" }
    });
    let empty_variants = serde_json::json!({});
    [
        ("gpt-5.4", gpt_variants.clone()),
        ("gpt-5.4-pro", gpt_pro_variants.clone()),
        ("gpt-5.4-mini", gpt_variants.clone()),
        ("gpt-5.4-nano", gpt_variants.clone()),
        ("gpt-5.5", gpt_variants.clone()),
        ("gpt-5.5-pro", gpt_pro_variants),
        ("gpt-5.6", gpt_56_variants.clone()),
        ("gpt-5.6-sol", gpt_56_variants.clone()),
        ("gpt-5.6-terra", gpt_56_variants.clone()),
        ("gpt-5.6-luna", gpt_56_luna_variants.clone()),
        ("claude-fable-5", generic_reasoning_variants.clone()),
        ("claude-opus-4-8", generic_reasoning_variants.clone()),
        ("claude-opus-4-7", generic_reasoning_variants.clone()),
        ("claude-opus-4-6", generic_reasoning_variants.clone()),
        ("claude-opus-4-5", generic_reasoning_variants.clone()),
        ("claude-sonnet-5", generic_reasoning_variants.clone()),
        ("claude-sonnet-4-6", generic_reasoning_variants.clone()),
        ("claude-sonnet-4-5", generic_reasoning_variants.clone()),
        ("claude-haiku-4-5", generic_reasoning_variants.clone()),
        ("gemini-3.5-flash", generic_reasoning_variants.clone()),
        ("gemini-3.1-pro", generic_reasoning_variants.clone()),
        ("gemini-3-flash", generic_reasoning_variants.clone()),
        ("deepseek-v4-pro", deepseek_v4_variants.clone()),
        ("deepseek-v4-flash", deepseek_v4_variants),
        ("grok-4.5", generic_reasoning_variants.clone()),
        ("grok-4.6", generic_reasoning_variants.clone()),
        ("hy3", generic_reasoning_variants.clone()),
        ("laguna-s-2.1", generic_reasoning_variants.clone()),
        ("ling-3.0-flash", generic_reasoning_variants.clone()),
        ("ling-3.0-tiny", empty_variants.clone()),
        ("glm-5.2", glm_variants.clone()),
        ("glm-5.1", empty_variants.clone()),
        ("glm-5", empty_variants.clone()),
        ("kimi-k2.7-code", empty_variants.clone()),
        ("kimi-k2.6", empty_variants.clone()),
        ("kimi-k2.5", empty_variants.clone()),
        ("mimo-v2.5", empty_variants.clone()),
        ("minimax-m3", minimax_m3_variants),
        ("minimax-m2.7", empty_variants.clone()),
        ("minimax-m2.5", empty_variants.clone()),
        ("nemotron-3-ultra", empty_variants.clone()),
        ("nemotron-3.5-lightning", empty_variants.clone()),
        ("north-mini-code", north_mini_code_variants.clone()),
        ("qwen3.7-max", empty_variants.clone()),
        ("qwen3.7-plus", empty_variants.clone()),
        ("qwen3.6-plus", empty_variants.clone()),
        ("qwen3.5-plus", empty_variants.clone()),
        ("grok-build-0.1", empty_variants.clone()),
        ("big-pickle", empty_variants.clone()),
        ("mimo-v2.5-free", empty_variants.clone()),
        ("nemotron-3-ultra-free", empty_variants.clone()),
        ("north-mini-code-free", north_mini_code_variants),
    ]
    .into_iter()
    .map(|(model, params)| (model.to_string(), params))
    .collect()
}

fn default_app_settings(app: &tauri::AppHandle) -> Result<AppSettings, String> {
    Ok(AppSettings {
        version: 1,
        endpoints: Vec::new(),
        test_settings: default_test_settings(),
        opencode_model_variants: default_opencode_model_variants(),
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
    let default_variants = default_opencode_model_variants();
    let mut changed = false;
    for (model, variants) in default_variants {
        if !settings.opencode_model_variants.contains_key(&model) {
            settings.opencode_model_variants.insert(model, variants);
            changed = true;
        }
    }
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

fn is_valid_endpoint_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|ch| ch.is_ascii_alphanumeric())
}

fn is_valid_endpoint_type(endpoint_type: &str) -> bool {
    matches!(endpoint_type, "codex" | "claude" | "opencode")
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
    use super::default_opencode_model_variants;
    use serde_json::json;

    #[test]
    fn requested_models_use_advertised_reasoning_variants() {
        let variants = default_opencode_model_variants();
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
}
