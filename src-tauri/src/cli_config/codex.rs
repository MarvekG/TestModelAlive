use std::fs;
use std::path::{Path, PathBuf};

use crate::cli_config::types::CliConfigPreviewFile;
use crate::model_metadata::model_profile;
use crate::models::SavedEndpoint;

use super::preview::preview_file;

pub(crate) fn build_codex_preview(
    endpoint: &SavedEndpoint,
    model: &str,
    files: &[(String, PathBuf, String)],
) -> Result<Vec<CliConfigPreviewFile>, String> {
    let mut output = Vec::new();
    for (file_id, path, language) in files {
        let content = if file_id == "codex-auth" {
            let value = serde_json::json!({ "OPENAI_API_KEY": endpoint.api_key });
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
    if let Some(context_window) = model_profile(model).and_then(|profile| profile.context_window) {
        let context_window = i64::try_from(context_window).map_err(|_| {
            format!("model context window for {model} exceeds the TOML integer range")
        })?;
        doc["model_context_window"] = toml_edit::value(context_window);
    }
    let providers = doc["model_providers"].or_insert(toml_edit::table());
    let provider = providers[endpoint.name.as_str()].or_insert(toml_edit::table());
    provider["name"] = toml_edit::value(endpoint.name.as_str());
    provider["base_url"] = toml_edit::value(endpoint.base_url.as_str());
    provider["wire_api"] = toml_edit::value("responses");
    provider["model_catalog_url"] = toml_edit::value(format!(
        "{}/models",
        endpoint.base_url.trim_end_matches('/')
    ));
    provider["experimental_bearer_token"] = toml_edit::value(endpoint.api_key.as_str());
    if let Some(table) = provider.as_table_mut() {
        table.remove("env_key");
        table.remove("requires_openai_auth");
        table.remove("auth");
    }
    doc["features"].or_insert(toml_edit::table())["api_key_model_discovery"] =
        toml_edit::value(true);
    Ok(format!("{}\n", doc))
}

#[cfg(test)]
mod tests {
    use super::build_codex_preview;
    use crate::models::SavedEndpoint;
    use std::path::PathBuf;

    #[test]
    fn preview_includes_the_model_context_window_when_metadata_is_available() {
        let endpoint = SavedEndpoint {
            id: "codex-endpoint".to_string(),
            name: "test-provider".to_string(),
            endpoint_type: "codex".to_string(),
            opencode_sdk_package: "@ai-sdk/openai-compatible".to_string(),
            base_url: "https://example.com/v1".to_string(),
            api_key: "test-key".to_string(),
            models: vec!["gpt-5.4".to_string()],
        };
        let files = vec![(
            "codex-config".to_string(),
            PathBuf::from("/path/that/does/not/exist/config.toml"),
            "toml".to_string(),
        )];

        let preview = build_codex_preview(&endpoint, "gpt-5.4", &files).unwrap();
        let config = preview[0]
            .content
            .parse::<toml_edit::DocumentMut>()
            .unwrap();

        assert!(config["model_context_window"]
            .as_integer()
            .is_some_and(|value| value > 0));
    }
}
