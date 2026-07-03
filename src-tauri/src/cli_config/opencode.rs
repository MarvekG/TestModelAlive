use serde_json::Value;
use std::fs;
use std::path::PathBuf;

use crate::cli_config::types::CliConfigPreviewFile;
use crate::models::SavedEndpoint;

use super::preview::{parse_jsonc_object, preview_file};

pub(crate) fn build_opencode_preview(
    endpoint: &SavedEndpoint,
    models: &[String],
    files: &[(String, PathBuf, String)],
) -> Result<Vec<CliConfigPreviewFile>, String> {
    let (file_id, path, language) = files
        .first()
        .ok_or_else(|| "OpenCode config path is missing".to_string())?;
    let mut value = if path.exists() {
        let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
        if text.trim().is_empty() {
            serde_json::json!({})
        } else {
            parse_jsonc_object(path, &text)?
        }
    } else {
        serde_json::json!({})
    };
    let object = value
        .as_object_mut()
        .ok_or_else(|| "OpenCode config must be a JSON object".to_string())?;
    let provider = object
        .entry("provider".to_string())
        .or_insert_with(|| serde_json::json!({}));
    let provider_object = provider
        .as_object_mut()
        .ok_or_else(|| "OpenCode provider must be a JSON object".to_string())?;
    let provider_key = next_provider_key(provider_object, &endpoint.name);
    provider_object.insert(
        provider_key,
        serde_json::json!({
            "npm": "@ai-sdk/openai-compatible",
            "options": {
                "baseURL": endpoint.base_url,
                "apiKey": endpoint.api_key
            },
            "models": models.iter().map(|model| (model.clone(), serde_json::json!({ "name": model }))).collect::<serde_json::Map<String, Value>>()
        }),
    );
    Ok(vec![preview_file(
        file_id,
        path,
        language,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&value).map_err(|err| err.to_string())?
        ),
    )])
}

fn next_provider_key(provider_object: &serde_json::Map<String, Value>, base: &str) -> String {
    if !provider_object.contains_key(base) {
        return base.to_string();
    }
    for index in 2.. {
        let candidate = format!("{base}-{index}");
        if !provider_object.contains_key(&candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded provider key search should always return")
}
