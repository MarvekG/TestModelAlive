use std::path::PathBuf;

use crate::cli_config::backup::{backup_cli_file, ensure_baseline_item};
use crate::cli_config::preview::{
    build_preview, parse_jsonc_object, validate_cli_target, validate_edited_config,
};
use crate::cli_config::restore::restore_original_cli_config_impl;
use crate::cli_config::types::{
    ApplyCliConfigResult, CliConfigPreview, CliConfigTargetKind, CliConfigWriteResult,
    EditedCliConfig, RestoreCliConfigResult, RestoreSelection,
};
use crate::models::SavedEndpoint;
use crate::paths::{cli_target_files, store_path, APP_SETTINGS_FILE};
use crate::settings::{
    read_app_settings, timestamp, timestamp_id, trim_apply_history, write_app_settings,
    write_app_settings_for_app, write_text_file, CliConfigApplyHistoryFile,
    CliConfigApplyHistoryItem, CliConfigBaselineView,
};

#[tauri::command]
pub fn build_cli_config_preview(
    _app: tauri::AppHandle,
    endpoint: SavedEndpoint,
    target: CliConfigTargetKind,
    selected_models: Vec<String>,
) -> Result<CliConfigPreview, String> {
    build_preview(endpoint, target, selected_models)
}

#[tauri::command]
pub fn apply_cli_config(
    app: tauri::AppHandle,
    endpoint: SavedEndpoint,
    target: CliConfigTargetKind,
    edited_config: EditedCliConfig,
) -> Result<ApplyCliConfigResult, String> {
    validate_cli_target(&endpoint, &target, &edited_config.selected_models)?;
    let allowed = cli_target_files(&target)?;
    validate_edited_config(&allowed, &edited_config)?;
    let mut settings = read_app_settings(&app)?;
    let settings_path = store_path(&app, APP_SETTINGS_FILE)?;
    let mut results = Vec::new();
    let mut history_files = Vec::new();
    let mut history_backups = Vec::new();
    let target_name = target.as_str().to_string();
    let endpoint_id = endpoint.id.clone();
    let endpoint_type = endpoint.endpoint_type.clone();
    let apply_id = timestamp_id("apply");

    for file in &edited_config.files {
        let target_path = PathBuf::from(&file.path);
        ensure_baseline_item(
            &app,
            &mut settings,
            &target_name,
            &file.file_id,
            &target_path,
        )?;
    }
    write_app_settings(&settings_path, &settings)?;

    for file in edited_config.files {
        let target_path = PathBuf::from(&file.path);
        let mut backup_paths = Vec::new();
        if target_path.exists() {
            match backup_cli_file(&app, &target_name, &target_path, "apply") {
                Ok(path) => {
                    let text = path.to_string_lossy().to_string();
                    history_backups.push(text.clone());
                    backup_paths.push(text);
                }
                Err(err) => {
                    results.push(CliConfigWriteResult {
                        target: target_name.clone(),
                        path: file.path,
                        backup_paths,
                        action: "backup".to_string(),
                        ok: false,
                        error: Some(err),
                    });
                    continue;
                }
            }
        }
        let action = if target_path.exists() {
            "updated"
        } else {
            "created"
        }
        .to_string();
        let content = normalize_edited_content(&target, &file.file_id, &target_path, &file.content);
        let write_result = content.and_then(|content| write_text_file(&target_path, &content));
        history_files.push(CliConfigApplyHistoryFile {
            file_id: file.file_id.clone(),
            path: file.path.clone(),
            action: action.clone(),
        });
        results.push(CliConfigWriteResult {
            target: target_name.clone(),
            path: file.path,
            backup_paths,
            action,
            ok: write_result.is_ok(),
            error: write_result.err(),
        });
    }

    settings
        .cli_config
        .apply_history
        .push(CliConfigApplyHistoryItem {
            apply_id,
            target: target_name.clone(),
            endpoint_id,
            created_at: timestamp(),
            backup_paths: history_backups,
            files: history_files,
        });
    trim_apply_history(&mut settings.cli_config);
    write_app_settings_for_app(&app, &settings)?;

    Ok(ApplyCliConfigResult {
        baseline_id: settings.cli_config.baseline_id,
        baseline_path: settings_path.to_string_lossy().to_string(),
        endpoint_type,
        target: target_name,
        results,
    })
}

fn normalize_edited_content(
    target: &CliConfigTargetKind,
    file_id: &str,
    path: &std::path::Path,
    content: &str,
) -> Result<String, String> {
    if matches!(target, CliConfigTargetKind::Opencode) && file_id == "opencode-config" {
        let value = parse_jsonc_object(path, content)?;
        return Ok(format!(
            "{}\n",
            serde_json::to_string_pretty(&value).map_err(|err| err.to_string())?
        ));
    }
    Ok(content.to_string())
}

#[tauri::command]
pub fn load_cli_config_baseline_items(
    app: tauri::AppHandle,
) -> Result<Vec<CliConfigBaselineView>, String> {
    Ok(read_app_settings(&app)?
        .cli_config
        .baseline_items
        .into_iter()
        .map(|item| CliConfigBaselineView {
            target: item.target,
            file_id: item.file_id,
            path: item.path,
            existed_before: item.existed_before,
            created_at: item.created_at,
        })
        .collect())
}

#[tauri::command]
pub fn restore_original_cli_config(
    app: tauri::AppHandle,
    selected_items: Vec<RestoreSelection>,
) -> Result<RestoreCliConfigResult, String> {
    restore_original_cli_config_impl(app, selected_items)
}
