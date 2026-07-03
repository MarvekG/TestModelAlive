use std::fs;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

use crate::paths::app_data_dir;
use crate::settings::{timestamp_id, AppSettings, CliConfigBaselineItem};

pub(crate) fn ensure_baseline_item(
    app: &tauri::AppHandle,
    settings: &mut AppSettings,
    target: &str,
    file_id: &str,
    target_path: &Path,
) -> Result<(), String> {
    let path_text = target_path.to_string_lossy().to_string();
    if settings
        .cli_config
        .baseline_items
        .iter()
        .any(|item| item.target == target && item.file_id == file_id && item.path == path_text)
    {
        return Ok(());
    }
    let existed_before = target_path.exists();
    let backup_path = if existed_before {
        Some(
            backup_cli_file(app, target, target_path, "baseline")?
                .to_string_lossy()
                .to_string(),
        )
    } else {
        None
    };
    settings
        .cli_config
        .baseline_items
        .push(CliConfigBaselineItem {
            target: target.to_string(),
            file_id: file_id.to_string(),
            path: path_text,
            existed_before,
            backup_path,
            created_at: timestamp_id("at"),
        });
    Ok(())
}

pub(crate) fn backup_cli_file(
    app: &tauri::AppHandle,
    target: &str,
    path: &Path,
    reason: &str,
) -> Result<PathBuf, String> {
    let backup_dir = app_data_dir(app)?.join("cli-config-backups").join(target);
    fs::create_dir_all(&backup_dir).map_err(|err| err.to_string())?;
    let stem = path.file_name().unwrap_or_default().to_string_lossy();
    for index in 0..1000 {
        let candidate = backup_dir.join(format!(
            "{stem}.{}.{}.{}.bak",
            reason,
            OffsetDateTime::now_utc().unix_timestamp_nanos(),
            index
        ));
        if !candidate.exists() {
            fs::copy(path, &candidate).map_err(|err| err.to_string())?;
            return Ok(candidate);
        }
    }
    Err(format!(
        "could not allocate backup path for {}",
        path.display()
    ))
}
