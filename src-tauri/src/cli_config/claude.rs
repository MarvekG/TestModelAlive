use serde_json::Value;
use std::path::PathBuf;

use crate::cli_config::types::CliConfigPreviewFile;
use crate::models::SavedEndpoint;

use super::preview::{preview_file, read_json_object};

pub(crate) fn build_claude_preview(
    endpoint: &SavedEndpoint,
    model: &str,
    files: &[(String, PathBuf, String)],
) -> Result<Vec<CliConfigPreviewFile>, String> {
    let mut output = Vec::new();
    for (file_id, path, language) in files {
        let mut value = read_json_object(path)?;
        if file_id == "claude-settings" {
            let object = value
                .as_object_mut()
                .ok_or_else(|| "Claude settings must be a JSON object".to_string())?;
            object.entry("$schema".to_string()).or_insert(Value::String(
                "https://json.schemastore.org/claude-code-settings.json".to_string(),
            ));
            object
                .entry("includeGitInstructions".to_string())
                .or_insert(Value::Bool(false));
            let env = object
                .entry("env".to_string())
                .or_insert_with(|| serde_json::json!({}));
            let env_object = env
                .as_object_mut()
                .ok_or_else(|| "Claude settings env must be a JSON object".to_string())?;
            env_object.insert(
                "ANTHROPIC_BASE_URL".to_string(),
                Value::String(endpoint.base_url.clone()),
            );
            env_object.insert(
                "ANTHROPIC_API_KEY".to_string(),
                Value::String(endpoint.api_key.clone()),
            );
            for key in [
                "ANTHROPIC_MODEL",
                "ANTHROPIC_DEFAULT_SONNET_MODEL",
                "ANTHROPIC_DEFAULT_OPUS_MODEL",
                "ANTHROPIC_DEFAULT_HAIKU_MODEL",
                "ANTHROPIC_DEFAULT_FABLE_MODEL",
                "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME",
                "ANTHROPIC_DEFAULT_OPUS_MODEL_NAME",
                "ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME",
                "ANTHROPIC_DEFAULT_FABLE_MODEL_NAME",
            ] {
                env_object.insert(key.to_string(), Value::String(model.to_string()));
            }
            for (key, value) in [
                ("CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS", "1"),
                ("ENABLE_PROMPT_CACHING_1H", "1"),
                ("CLAUDE_CODE_ATTRIBUTION_HEADER", "0"),
                ("DISABLE_TELEMETRY", "1"),
                ("DISABLE_ERROR_REPORTING", "1"),
                ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1"),
                ("CLAUDE_CODE_ENABLE_BACKGROUND_PLUGIN_REFRESH", "0"),
            ] {
                env_object
                    .entry(key.to_string())
                    .or_insert_with(|| Value::String(value.to_string()));
            }
        } else {
            let object = value
                .as_object_mut()
                .ok_or_else(|| "Claude state must be a JSON object".to_string())?;
            object.insert("hasCompletedOnboarding".to_string(), Value::Bool(true));
        }
        output.push(preview_file(
            file_id,
            path,
            language,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&value).map_err(|err| err.to_string())?
            ),
        ));
    }
    Ok(output)
}
