use std::fs;
use std::path::PathBuf;

use crate::cli_config::backup::backup_cli_file;
use crate::cli_config::types::{CliConfigRestoreResult, RestoreCliConfigResult, RestoreSelection};
use crate::settings::read_app_settings;

pub(crate) fn restore_original_cli_config_impl(
    app: tauri::AppHandle,
    selected_items: Vec<RestoreSelection>,
) -> Result<RestoreCliConfigResult, String> {
    if selected_items.is_empty() {
        return Err("select at least one config item to restore".to_string());
    }
    let settings = read_app_settings(&app)?;
    let mut results = Vec::new();
    for selection in selected_items {
        let target_name = selection.target.as_str().to_string();
        let Some(item) = settings.cli_config.baseline_items.iter().find(|item| {
            item.target == target_name
                && item.file_id == selection.file_id
                && item.path == selection.path
        }) else {
            results.push(CliConfigRestoreResult {
                target: target_name,
                path: selection.path,
                action: "missing-baseline".to_string(),
                ok: false,
                error: Some("baseline item not found".to_string()),
            });
            continue;
        };
        let target_path = PathBuf::from(&item.path);
        if target_path.exists() {
            if let Err(err) = backup_cli_file(&app, &target_name, &target_path, "pre-restore") {
                results.push(CliConfigRestoreResult {
                    target: target_name.clone(),
                    path: item.path.clone(),
                    action: "backup".to_string(),
                    ok: false,
                    error: Some(err),
                });
                continue;
            }
        }
        let restore_result = if item.existed_before {
            let backup = item
                .backup_path
                .as_ref()
                .ok_or_else(|| "baseline backup path is missing".to_string());
            backup.and_then(|path| {
                let backup_path = PathBuf::from(path);
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent).map_err(|err| err.to_string())?;
                }
                fs::copy(&backup_path, &target_path)
                    .map(|_| ())
                    .map_err(|err| err.to_string())
            })
        } else if target_path.exists() {
            fs::remove_file(&target_path).map_err(|err| err.to_string())
        } else {
            Ok(())
        };
        results.push(CliConfigRestoreResult {
            target: target_name,
            path: item.path.clone(),
            action: if item.existed_before {
                "restored"
            } else {
                "deleted"
            }
            .to_string(),
            ok: restore_result.is_ok(),
            error: restore_result.err(),
        });
    }
    Ok(RestoreCliConfigResult {
        baseline_id: settings.cli_config.baseline_id,
        results,
    })
}
