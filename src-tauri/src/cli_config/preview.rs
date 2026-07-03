use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli_config::types::{
    CliConfigPreview, CliConfigPreviewFile, CliConfigTargetKind, EditedCliConfig,
};
use crate::models::SavedEndpoint;
use crate::paths::cli_target_files;

use super::{claude, codex, opencode};

pub(crate) fn build_preview(
    endpoint: SavedEndpoint,
    target: CliConfigTargetKind,
    selected_models: Vec<String>,
) -> Result<CliConfigPreview, String> {
    validate_cli_target(&endpoint, &target, &selected_models)?;
    let files = cli_target_files(&target)?;
    let files = match target {
        CliConfigTargetKind::Codex => {
            codex::build_codex_preview(&endpoint, &selected_models[0], &files)?
        }
        CliConfigTargetKind::Claude => {
            claude::build_claude_preview(&endpoint, &selected_models[0], &files)?
        }
        CliConfigTargetKind::Opencode => {
            opencode::build_opencode_preview(&endpoint, &selected_models, &files)?
        }
    };
    Ok(CliConfigPreview {
        endpoint_type: endpoint.endpoint_type,
        target: target.as_str().to_string(),
        files,
    })
}

pub(crate) fn validate_cli_target(
    endpoint: &SavedEndpoint,
    target: &CliConfigTargetKind,
    selected_models: &[String],
) -> Result<(), String> {
    match target {
        CliConfigTargetKind::Codex => {
            if endpoint.endpoint_type != "codex" {
                return Err("Codex config requires a codex endpoint".to_string());
            }
            if selected_models.len() != 1 {
                return Err("Codex config requires exactly one selected model".to_string());
            }
        }
        CliConfigTargetKind::Claude => {
            if endpoint.endpoint_type != "claude" {
                return Err("Claude config requires a claude endpoint".to_string());
            }
            if selected_models.len() != 1 {
                return Err("Claude config requires exactly one selected model".to_string());
            }
        }
        CliConfigTargetKind::Opencode => {
            if endpoint.endpoint_type != "codex" {
                return Err("OpenCode config uses codex-compatible endpoints".to_string());
            }
            if selected_models.is_empty() {
                return Err("OpenCode config requires at least one selected model".to_string());
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_edited_config(
    allowed: &[(String, PathBuf, String)],
    edited: &EditedCliConfig,
) -> Result<(), String> {
    if edited.files.len() != allowed.len() {
        return Err("edited config files do not match target files".to_string());
    }
    for (file_id, path, _) in allowed {
        let Some(file) = edited.files.iter().find(|file| file.file_id == *file_id) else {
            return Err(format!("missing edited config file: {file_id}"));
        };
        if PathBuf::from(&file.path) != *path {
            return Err(format!(
                "edited path for {file_id} does not match expected target path"
            ));
        }
        if file.content.trim().is_empty() {
            return Err(format!("edited content for {file_id} is empty"));
        }
    }
    Ok(())
}

pub(crate) fn preview_file(
    file_id: &str,
    path: &Path,
    language: &str,
    content: String,
) -> CliConfigPreviewFile {
    CliConfigPreviewFile {
        file_id: file_id.to_string(),
        path: path.to_string_lossy().to_string(),
        language: language.to_string(),
        content,
    }
}

pub(crate) fn read_json_object(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
    if text.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    let value: Value = serde_json::from_str(&text)
        .map_err(|err| format!("{} is not valid JSON: {err}", path.display()))?;
    if !value.is_object() {
        return Err(format!("{} must contain a JSON object", path.display()));
    }
    Ok(value)
}

pub(crate) fn parse_jsonc_object(path: &Path, text: &str) -> Result<Value, String> {
    let normalized = strip_jsonc(text);
    let value: Value = serde_json::from_str(&normalized)
        .map_err(|err| format!("{} is not valid JSON/JSONC: {err}", path.display()))?;
    if !value.is_object() {
        return Err(format!("{} must contain a JSON object", path.display()));
    }
    Ok(value)
}

pub(crate) fn strip_jsonc(text: &str) -> String {
    remove_trailing_commas(&remove_jsonc_comments(text))
}

fn remove_jsonc_comments(text: &str) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;

    while index < chars.len() {
        let ch = chars[index];
        if in_string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            output.push(ch);
            index += 1;
            continue;
        }

        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            index += 2;
            while index < chars.len() && chars[index] != '\n' && chars[index] != '\r' {
                index += 1;
            }
            continue;
        }

        if ch == '/' && chars.get(index + 1) == Some(&'*') {
            index += 2;
            while index + 1 < chars.len() && !(chars[index] == '*' && chars[index + 1] == '/') {
                index += 1;
            }
            index = (index + 2).min(chars.len());
            continue;
        }

        output.push(ch);
        index += 1;
    }

    output
}

fn remove_trailing_commas(text: &str) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;

    while index < chars.len() {
        let ch = chars[index];
        if in_string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            output.push(ch);
            index += 1;
            continue;
        }

        if ch == ',' {
            let mut lookahead = index + 1;
            while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                lookahead += 1;
            }
            if matches!(chars.get(lookahead), Some('}') | Some(']')) {
                index += 1;
                continue;
            }
        }

        output.push(ch);
        index += 1;
    }

    output
}
