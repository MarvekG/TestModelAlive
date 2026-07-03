use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli_config::types::CliConfigPreviewFile;
use crate::models::SavedEndpoint;

use super::preview::{preview_file, read_json_object};

pub(crate) fn build_codex_preview(
    endpoint: &SavedEndpoint,
    model: &str,
    files: &[(String, PathBuf, String)],
) -> Result<Vec<CliConfigPreviewFile>, String> {
    let mut output = Vec::new();
    for (file_id, path, language) in files {
        let content = if file_id == "codex-auth" {
            let mut value = read_json_object(path)?;
            value["OPENAI_API_KEY"] = Value::String(endpoint.api_key.clone());
            format!(
                "{}\n",
                serde_json::to_string_pretty(&value).map_err(|err| err.to_string())?
            )
        } else {
            merge_codex_toml(path, endpoint, model)?
        };
        output.push(preview_file(file_id, path, language, content));
    }
    Ok(output)
}

fn merge_codex_toml(path: &Path, endpoint: &SavedEndpoint, model: &str) -> Result<String, String> {
    let text = if path.exists() {
        fs::read_to_string(path).map_err(|err| err.to_string())?
    } else {
        String::new()
    };
    let mut doc = if text.trim().is_empty() {
        toml_edit::DocumentMut::new()
    } else {
        text.parse::<toml_edit::DocumentMut>()
            .map_err(|err| format!("{} is not valid TOML: {err}", path.display()))?
    };
    doc["model"] = toml_edit::value(model);
    doc["model_provider"] = toml_edit::value(endpoint.name.as_str());
    let providers = doc["model_providers"].or_insert(toml_edit::table());
    let provider = providers[endpoint.name.as_str()].or_insert(toml_edit::table());
    provider["name"] = toml_edit::value(endpoint.name.as_str());
    provider["base_url"] = toml_edit::value(endpoint.base_url.as_str());
    provider["env_key"] = toml_edit::value("OPENAI_API_KEY");
    provider["wire_api"] = toml_edit::value("responses");
    if let Some(table) = provider.as_table_mut() {
        table.remove("requires_openai_auth");
    }
    Ok(format!("{}\n", doc))
}
