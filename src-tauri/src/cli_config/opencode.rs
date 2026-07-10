use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli_config::types::{CliConfigPreviewFile, CliConfigPreviewWarning};
use crate::models::SavedEndpoint;

use super::preview::{parse_jsonc_object, preview_file};

pub(crate) fn build_opencode_preview_with_warnings(
    endpoint: &SavedEndpoint,
    models: &[String],
    files: &[(String, PathBuf, String)],
    default_model: Option<String>,
    model_variants: &BTreeMap<String, Value>,
) -> Result<(Vec<CliConfigPreviewFile>, Vec<CliConfigPreviewWarning>), String> {
    let (file_id, path, language) = files
        .first()
        .ok_or_else(|| "OpenCode config path is missing".to_string())?;
    let mut warnings = Vec::new();
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
    if provider_object.contains_key(&endpoint.name) {
        warnings.push(CliConfigPreviewWarning::OpenCodeProviderOverwrite {
            provider: endpoint.name.clone(),
        });
    }
    provider_object.insert(
        endpoint.name.clone(),
        serde_json::json!({
            "npm": endpoint.opencode_sdk_package.as_str(),
            "options": {
                "baseURL": endpoint.base_url,
                "apiKey": endpoint.api_key
            },
            "models": build_model_entries(models, model_variants)
        }),
    );
    if let Some(model) = default_model {
        if !models.iter().any(|selected| selected == &model) {
            return Err("OpenCode default model must be one of the selected models".to_string());
        }
        let qualified_model = format!("{}/{model}", endpoint.name);
        object.insert("model".to_string(), Value::String(qualified_model.clone()));
        object.insert("small_model".to_string(), Value::String(qualified_model));
    }
    Ok((
        vec![preview_file(
            file_id,
            path,
            language,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&value).map_err(|err| err.to_string())?
            ),
        )],
        warnings,
    ))
}

pub(crate) fn build_model_entries(
    models: &[String],
    model_variants: &BTreeMap<String, Value>,
) -> serde_json::Map<String, Value> {
    models
        .iter()
        .map(|model| {
            let mut entry = serde_json::Map::new();
            entry.insert("name".to_string(), Value::String(model.clone()));
            if let Some(variants) = model_variants
                .get(model)
                .and_then(Value::as_object)
                .filter(|variants| !variants.is_empty())
            {
                entry.insert("variants".to_string(), Value::Object(variants.clone()));
            }
            (model.clone(), Value::Object(entry))
        })
        .collect()
}

pub(crate) fn remove_matching_provider(
    path: &Path,
    endpoint: &SavedEndpoint,
) -> Result<(String, bool), String> {
    if !path.exists() {
        return Ok((
            format!(
                "{}\n",
                serde_json::to_string_pretty(&serde_json::json!({}))
                    .map_err(|err| err.to_string())?
            ),
            false,
        ));
    }

    let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
    if text.trim().is_empty() {
        return Ok((
            format!(
                "{}\n",
                serde_json::to_string_pretty(&serde_json::json!({}))
                    .map_err(|err| err.to_string())?
            ),
            false,
        ));
    }
    let mut value = parse_jsonc_object(path, &text)?;
    let mut removed = false;
    if let Some(provider_object) = value
        .as_object_mut()
        .and_then(|object| object.get_mut("provider"))
        .and_then(Value::as_object_mut)
    {
        let keys = provider_object
            .iter()
            .filter_map(|(key, provider)| {
                provider_matches_endpoint(provider, endpoint).then(|| key.clone())
            })
            .collect::<Vec<_>>();
        for key in keys {
            provider_object.remove(&key);
            removed = true;
        }
    }
    if removed {
        remove_default_model_if_matches(&mut value, &endpoint.name);
    }
    Ok((
        format!(
            "{}\n",
            serde_json::to_string_pretty(&value).map_err(|err| err.to_string())?
        ),
        removed,
    ))
}

fn remove_default_model_if_matches(value: &mut Value, provider_name: &str) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    let keys = ["model", "small_model"]
        .into_iter()
        .filter(|key| {
            object
                .get(*key)
                .and_then(Value::as_str)
                .map(|model| {
                    model
                        .split_once('/')
                        .is_some_and(|(provider, _)| provider == provider_name)
                })
                .unwrap_or(false)
        })
        .map(str::to_string)
        .collect::<Vec<_>>();
    for key in keys {
        object.remove(&key);
    }
}

pub(crate) fn build_remove_opencode_preview(
    endpoint: &SavedEndpoint,
    files: &[(String, PathBuf, String)],
) -> Result<Vec<CliConfigPreviewFile>, String> {
    let (file_id, path, language) = files
        .first()
        .ok_or_else(|| "OpenCode config path is missing".to_string())?;
    let (content, removed) = remove_matching_provider(path, endpoint)?;
    if !removed {
        return Err("No matching OpenCode provider was found".to_string());
    }
    Ok(vec![preview_file(file_id, path, language, content)])
}

fn provider_matches_endpoint(provider: &Value, endpoint: &SavedEndpoint) -> bool {
    let Some(options) = provider.get("options").and_then(Value::as_object) else {
        return false;
    };
    let base_url = options
        .get("baseURL")
        .or_else(|| options.get("base_url"))
        .and_then(Value::as_str);
    let api_key = options
        .get("apiKey")
        .or_else(|| options.get("api_key"))
        .and_then(Value::as_str);
    base_url == Some(endpoint.base_url.as_str()) && api_key == Some(endpoint.api_key.as_str())
}
