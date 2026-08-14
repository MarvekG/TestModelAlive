use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli_config::types::{CliConfigPreviewFile, CliConfigPreviewWarning};
use crate::model_metadata::{dsh_default_input, model_limit, opencode_model_variants};
use crate::models::SavedEndpoint;

use super::preview::preview_file;

const DEEPSEEK_PROVIDER_NOT_FOUND: &str = "deepseek_provider_not_found";
const DEFAULT_API_PROTOCOL: &str = "openai-completions";

pub(crate) fn dsh_provider_name(endpoint: &SavedEndpoint) -> String {
    dsh_provider_name_for_endpoint_name(&endpoint.name)
}

pub(crate) fn dsh_provider_name_for_endpoint_name(endpoint_name: &str) -> String {
    format!("tma-{endpoint_name}")
}

pub(crate) fn dsh_api_key_env(endpoint: &SavedEndpoint) -> String {
    dsh_api_key_env_for_name(&endpoint.name)
}

pub(crate) fn dsh_api_key_env_for_name(endpoint_name: &str) -> String {
    format!("TMA_DSH_{}_API_KEY", endpoint_name.to_ascii_uppercase())
}

pub(crate) fn build_deepseek_preview_with_warnings(
    endpoint: &SavedEndpoint,
    models: &[String],
    default_model: Option<String>,
    files: &[(String, PathBuf, String)],
    api_protocol: Option<String>,
) -> Result<(Vec<CliConfigPreviewFile>, Vec<CliConfigPreviewWarning>), String> {
    let default_model = select_default_model(models, default_model)?;
    let api_protocol = select_api_protocol(api_protocol.as_deref())?;
    let mut output = Vec::new();
    let mut warnings = Vec::new();
    for (file_id, path, language) in files {
        let content = match file_id.as_str() {
            "deepseek-settings" => {
                let (content, overwritten_providers) = build_settings_content_with_overwrite(
                    path,
                    endpoint,
                    models,
                    &default_model,
                    api_protocol,
                )?;
                for provider in overwritten_providers {
                    warnings.push(CliConfigPreviewWarning::DeepseekProviderOverwrite { provider });
                }
                content
            }
            "deepseek-credentials" => build_credentials_content(path, endpoint)?,
            _ => {
                return Err(format!(
                    "unexpected DeepSeek Harness config file: {file_id}"
                ))
            }
        };
        output.push(preview_file(file_id, path, language, content));
    }
    Ok((output, warnings))
}

#[cfg(test)]
pub(crate) fn build_settings_content(
    path: &Path,
    endpoint: &SavedEndpoint,
    model: &str,
) -> Result<String, String> {
    let models = vec![model.to_string()];
    Ok(
        build_settings_content_with_overwrite(
            path,
            endpoint,
            &models,
            model,
            DEFAULT_API_PROTOCOL,
        )?
        .0,
    )
}

pub(crate) fn build_test_settings_content(
    path: &Path,
    endpoint: &SavedEndpoint,
    model: &str,
) -> Result<String, String> {
    let models = vec![model.to_string()];
    Ok(
        build_settings_content_with_overwrite(
            path,
            endpoint,
            &models,
            model,
            DEFAULT_API_PROTOCOL,
        )?
        .0,
    )
}

pub(crate) fn build_remove_deepseek_provider_preview(
    endpoint: &SavedEndpoint,
    files: &[(String, PathBuf, String)],
) -> Result<Vec<CliConfigPreviewFile>, String> {
    build_deepseek_operation_preview(
        files,
        |path| remove_deepseek_provider_settings_content(path, endpoint),
        |path| remove_deepseek_provider_credentials_content(path, endpoint),
    )
}

fn build_deepseek_operation_preview<Settings, Credentials>(
    files: &[(String, PathBuf, String)],
    mut build_settings: Settings,
    mut build_credentials: Credentials,
) -> Result<Vec<CliConfigPreviewFile>, String>
where
    Settings: FnMut(&Path) -> Result<String, String>,
    Credentials: FnMut(&Path) -> Result<String, String>,
{
    let mut output = Vec::new();
    for (file_id, path, language) in files {
        let content = match file_id.as_str() {
            "deepseek-settings" => build_settings(path)?,
            "deepseek-credentials" => build_credentials(path)?,
            _ => {
                return Err(format!(
                    "unexpected DeepSeek Harness config file: {file_id}"
                ))
            }
        };
        output.push(preview_file(file_id, path, language, content));
    }
    Ok(output)
}

fn remove_deepseek_provider_settings_content(
    path: &Path,
    endpoint: &SavedEndpoint,
) -> Result<String, String> {
    let mut root = read_yaml_mapping(path)?;
    let provider_name = dsh_provider_name(endpoint);
    if !remove_pi_ai_provider(&mut root, &provider_name)? {
        return Err(DEEPSEEK_PROVIDER_NOT_FOUND.to_string());
    }
    remove_default_model_for_provider(&mut root, &provider_name);
    serialize_yaml_mapping(&root)
}

fn remove_deepseek_provider_credentials_content(
    path: &Path,
    endpoint: &SavedEndpoint,
) -> Result<String, String> {
    let mut credentials = read_yaml_mapping(path)?;
    credentials.remove(&yaml_key(&dsh_api_key_env(endpoint)));
    serialize_yaml_mapping(&credentials)
}

fn remove_default_model_for_provider(root: &mut Mapping, provider_name: &str) -> bool {
    let Some(default_model) = root
        .get(&yaml_key("agent-default-model"))
        .and_then(Value::as_mapping)
    else {
        return false;
    };
    let is_provider = default_model
        .get(&yaml_key("provider"))
        .and_then(Value::as_str)
        == Some(provider_name);
    if is_provider {
        root.remove(&yaml_key("agent-default-model"));
    }
    is_provider
}

fn build_settings_content_with_overwrite(
    path: &Path,
    endpoint: &SavedEndpoint,
    models: &[String],
    default_model: &str,
    api_protocol: &str,
) -> Result<(String, Vec<String>), String> {
    let mut root = read_yaml_mapping(path)?;
    let provider_name = dsh_provider_name(endpoint);
    let mut overwritten_providers = Vec::new();
    let llm = nested_mapping(&mut root, "llm-pi-ai", "settings.llm-pi-ai")?;
    let providers = nested_mapping(llm, "providers", "settings.llm-pi-ai.providers")?;
    let provider_key = yaml_key(&provider_name);
    if providers.contains_key(&provider_key) {
        overwritten_providers.push(provider_name.clone());
    }
    providers.insert(
        provider_key,
        provider_definition(endpoint, models, api_protocol),
    );

    let mut default_model_config = Mapping::new();
    default_model_config.insert(yaml_key("provider"), Value::String(provider_name));
    default_model_config.insert(yaml_key("model"), Value::String(default_model.to_string()));
    root.insert(
        yaml_key("agent-default-model"),
        Value::Mapping(default_model_config),
    );

    Ok((serialize_yaml_mapping(&root)?, overwritten_providers))
}

fn remove_pi_ai_provider(root: &mut Mapping, provider_name: &str) -> Result<bool, String> {
    let Some(llm) = root
        .get_mut(&yaml_key("llm-pi-ai"))
        .and_then(Value::as_mapping_mut)
    else {
        return Ok(false);
    };
    let Some(providers) = llm
        .get_mut(&yaml_key("providers"))
        .and_then(Value::as_mapping_mut)
    else {
        return Ok(false);
    };
    Ok(providers.remove(&yaml_key(provider_name)).is_some())
}

fn build_credentials_content(path: &Path, endpoint: &SavedEndpoint) -> Result<String, String> {
    let mut credentials = read_yaml_mapping(path)?;
    credentials.insert(
        yaml_key(&dsh_api_key_env(endpoint)),
        Value::String(endpoint.api_key.clone()),
    );
    serialize_yaml_mapping(&credentials)
}

fn provider_definition(endpoint: &SavedEndpoint, models: &[String], api_protocol: &str) -> Value {
    let model_entries = models.iter().map(|model| model_definition(model)).collect();

    let mut provider = Mapping::new();
    provider.insert(
        yaml_key("displayName"),
        Value::String(endpoint.name.clone()),
    );
    provider.insert(
        yaml_key("apiKeyEnv"),
        Value::String(dsh_api_key_env(endpoint)),
    );
    provider.insert(yaml_key("api"), Value::String(api_protocol.to_string()));
    provider.insert(
        yaml_key("baseURL"),
        Value::String(endpoint.base_url.clone()),
    );
    provider.insert(yaml_key("models"), Value::Sequence(model_entries));
    Value::Mapping(provider)
}

fn model_definition(model: &str) -> Value {
    let mut model_entry = Mapping::new();
    model_entry.insert(yaml_key("id"), Value::String(model.to_string()));
    if let Some(profile) = known_model_profile(model) {
        // Reuse the model capabilities from OpenCode, but emit DSH's YAML schema.
        if let Some(context_window) = profile.context_window {
            model_entry.insert(yaml_key("contextWindow"), Value::from(context_window));
        }
        if let Some(max_tokens) = profile.max_tokens {
            model_entry.insert(yaml_key("maxTokens"), Value::from(max_tokens));
        }
        model_entry.insert(
            yaml_key("input"),
            Value::Sequence(profile.input.iter().cloned().map(Value::String).collect()),
        );
        if !profile.reasoning_efforts.is_empty() {
            let mut reasoning_efforts = Mapping::new();
            for (level, wire_value) in &profile.reasoning_efforts {
                reasoning_efforts.insert(
                    yaml_key(level),
                    wire_value
                        .as_ref()
                        .map(|value| Value::String(value.clone()))
                        .unwrap_or(Value::Null),
                );
            }
            model_entry.insert(
                yaml_key("reasoningEfforts"),
                Value::Mapping(reasoning_efforts),
            );
        }
    }
    Value::Mapping(model_entry)
}

struct ModelProfile {
    context_window: Option<u64>,
    max_tokens: Option<u64>,
    input: Vec<String>,
    reasoning_efforts: Vec<(String, Option<String>)>,
}

fn known_model_profile(model: &str) -> Option<ModelProfile> {
    let (context_window, max_tokens) = model_limit(model)
        .and_then(|limit| {
            Some((
                limit.get("context")?.as_u64()?,
                limit.get("output")?.as_u64()?,
            ))
        })
        .map(|(context_window, max_tokens)| (Some(context_window), Some(max_tokens)))
        .unwrap_or((None, None));
    let mut reasoning_efforts: Vec<(String, Option<String>)> = opencode_model_variants()
        .get(model)
        .and_then(serde_json::Value::as_object)
        .map(|variants| {
            variants
                .keys()
                .filter_map(|level| match level.as_str() {
                    "none" => Some(("off".to_string(), None)),
                    "minimal" | "low" | "medium" | "high" | "xhigh" | "max" => {
                        Some((level.clone(), Some(level.clone())))
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    if reasoning_efforts.iter().all(|(level, _)| level == "off") {
        reasoning_efforts.clear();
    }
    if context_window.is_none() && max_tokens.is_none() && reasoning_efforts.is_empty() {
        return None;
    }
    Some(ModelProfile {
        context_window,
        max_tokens,
        input: dsh_default_input().to_vec(),
        reasoning_efforts,
    })
}

fn select_default_model(
    models: &[String],
    default_model: Option<String>,
) -> Result<String, String> {
    let default_model = default_model
        .filter(|model| models.iter().any(|selected| selected == model))
        .ok_or_else(|| {
            "DeepSeek Harness default model must be one of the selected models".to_string()
        })?;
    Ok(default_model)
}

fn select_api_protocol(api_protocol: Option<&str>) -> Result<&str, String> {
    let api_protocol = api_protocol.unwrap_or(DEFAULT_API_PROTOCOL);
    if matches!(
        api_protocol,
        "openai-completions" | "openai-responses" | "anthropic-messages"
    ) {
        return Ok(api_protocol);
    }
    Err(format!(
        "DeepSeek Harness API protocol must be openai-completions, openai-responses, or anthropic-messages, got {api_protocol}"
    ))
}

pub(crate) fn parse_yaml_mapping(path: &Path, text: &str) -> Result<Mapping, String> {
    let value = serde_yaml::from_str::<Value>(text)
        .map_err(|err| format!("{} is not valid YAML: {err}", path.display()))?;
    value
        .as_mapping()
        .cloned()
        .ok_or_else(|| format!("{} must contain a YAML mapping", path.display()))
}

pub(crate) fn serialize_yaml_mapping(mapping: &Mapping) -> Result<String, String> {
    let content = serde_yaml::to_string(mapping).map_err(|err| err.to_string())?;
    Ok(if content.ends_with('\n') {
        content
    } else {
        format!("{content}\n")
    })
}

fn read_yaml_mapping(path: &Path) -> Result<Mapping, String> {
    if !path.exists() {
        return Ok(Mapping::new());
    }
    let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
    if text.trim().is_empty() {
        return Ok(Mapping::new());
    }
    parse_yaml_mapping(path, &text)
}

fn nested_mapping<'a>(
    parent: &'a mut Mapping,
    key: &str,
    label: &str,
) -> Result<&'a mut Mapping, String> {
    let key = yaml_key(key);
    if !parent.contains_key(&key) {
        parent.insert(key.clone(), Value::Mapping(Mapping::new()));
    }
    parent
        .get_mut(&key)
        .and_then(Value::as_mapping_mut)
        .ok_or_else(|| format!("DeepSeek Harness {label} must be a YAML mapping"))
}

fn yaml_key(key: &str) -> Value {
    Value::String(key.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use super::{
        build_deepseek_preview_with_warnings, build_settings_content, dsh_api_key_env,
        remove_deepseek_provider_credentials_content, remove_deepseek_provider_settings_content,
        DEEPSEEK_PROVIDER_NOT_FOUND,
    };
    use crate::models::SavedEndpoint;

    fn endpoint() -> SavedEndpoint {
        SavedEndpoint {
            id: "deepseek-test-001".to_string(),
            name: "Example".to_string(),
            endpoint_type: "deepseek".to_string(),
            opencode_sdk_package: "@ai-sdk/openai-compatible".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_key: "sk-test".to_string(),
            models: vec!["example-model".to_string()],
        }
    }

    #[test]
    fn settings_use_an_openai_compatible_route_and_default_model() {
        let content = build_settings_content(
            Path::new("deepseek-settings-does-not-exist.yaml"),
            &endpoint(),
            "example-model",
        )
        .expect("settings should be generated");
        let value: serde_json::Value =
            serde_yaml::from_str(&content).expect("generated settings should be valid YAML");

        assert_eq!(
            value["llm-pi-ai"]["providers"]["tma-Example"],
            json!({
                "displayName": "Example",
                "apiKeyEnv": "TMA_DSH_EXAMPLE_API_KEY",
                "api": "openai-completions",
                "baseURL": "https://api.example.com/v1",
                "models": [{ "id": "example-model" }]
            })
        );
        assert_eq!(
            value["agent-default-model"],
            json!({ "provider": "tma-Example", "model": "example-model" })
        );
    }

    #[test]
    fn credentials_env_name_uses_the_readable_endpoint_name() {
        assert_eq!(dsh_api_key_env(&endpoint()), "TMA_DSH_EXAMPLE_API_KEY");
        let mut other_name = endpoint();
        other_name.name = "Other".to_string();
        assert_ne!(dsh_api_key_env(&endpoint()), dsh_api_key_env(&other_name));
    }

    #[test]
    fn preview_keeps_all_selected_models_and_uses_the_selected_default() {
        let models = vec!["first-model".to_string(), "second-model".to_string()];
        let (files, warnings) = build_deepseek_preview_with_warnings(
            &endpoint(),
            &models,
            Some("second-model".to_string()),
            &[(
                "deepseek-settings".to_string(),
                PathBuf::from("deepseek-preview-does-not-exist.yaml"),
                "yaml".to_string(),
            )],
            None,
        )
        .expect("preview should be generated");
        let value: serde_json::Value =
            serde_yaml::from_str(&files[0].content).expect("preview should be valid YAML");

        assert!(warnings.is_empty());
        assert_eq!(
            value["llm-pi-ai"]["providers"]["tma-Example"]["models"],
            json!([{ "id": "first-model" }, { "id": "second-model" }])
        );
        assert_eq!(value["agent-default-model"]["model"], json!("second-model"));
    }

    #[test]
    fn preview_uses_the_selected_api_protocol() {
        for api_protocol in [
            "openai-completions",
            "openai-responses",
            "anthropic-messages",
        ] {
            let (files, warnings) = build_deepseek_preview_with_warnings(
                &endpoint(),
                &["example-model".to_string()],
                Some("example-model".to_string()),
                &[(
                    "deepseek-settings".to_string(),
                    PathBuf::from("deepseek-preview-does-not-exist.yaml"),
                    "yaml".to_string(),
                )],
                Some(api_protocol.to_string()),
            )
            .expect("preview should be generated");
            let value: serde_json::Value =
                serde_yaml::from_str(&files[0].content).expect("preview should be valid YAML");

            assert!(warnings.is_empty());
            assert_eq!(
                value["llm-pi-ai"]["providers"]["tma-Example"]["api"],
                json!(api_protocol)
            );
        }
    }

    #[test]
    fn preview_rejects_an_unknown_api_protocol() {
        let result = build_deepseek_preview_with_warnings(
            &endpoint(),
            &["example-model".to_string()],
            Some("example-model".to_string()),
            &[(
                "deepseek-settings".to_string(),
                PathBuf::from("deepseek-preview-does-not-exist.yaml"),
                "yaml".to_string(),
            )],
            Some("unsupported".to_string()),
        );

        assert!(result.is_err());
    }

    #[test]
    fn remove_deepseek_provider_preserves_other_configuration() {
        let path = temporary_settings_path();
        fs::write(
            &path,
            "llm-pi-ai:\n  providers:\n    tma-Example:\n      api: openai-completions\n    existing:\n      api: openai-completions\nagent-default-model:\n  provider: tma-Example\n  model: example-model\n",
        )
        .expect("test settings should be written");
        let result = remove_deepseek_provider_settings_content(&path, &endpoint());
        let _ = fs::remove_file(&path);
        let content = result.expect("provider should be removed");
        let value: serde_json::Value =
            serde_yaml::from_str(&content).expect("updated settings should be valid YAML");

        assert!(value["llm-pi-ai"]["providers"].get("tma-Example").is_none());
        assert_eq!(
            value["llm-pi-ai"]["providers"]["existing"]["api"],
            json!("openai-completions")
        );
        assert!(value.get("agent-default-model").is_none());
    }

    #[test]
    fn remove_deepseek_provider_returns_a_code_when_it_is_missing() {
        let path = temporary_settings_path();

        assert_eq!(
            remove_deepseek_provider_settings_content(&path, &endpoint()),
            Err(DEEPSEEK_PROVIDER_NOT_FOUND.to_string())
        );
    }

    #[test]
    fn remove_deepseek_provider_removes_only_its_credentials() {
        let path = temporary_settings_path();
        fs::write(
            &path,
            "TMA_DSH_EXAMPLE_API_KEY: current-key\nOTHER_API_KEY: keep\n",
        )
        .expect("test credentials should be written");
        let result = remove_deepseek_provider_credentials_content(&path, &endpoint());
        let _ = fs::remove_file(&path);
        let content = result.expect("provider credentials should be removed");
        let value: serde_json::Value =
            serde_yaml::from_str(&content).expect("updated credentials should be valid YAML");

        assert!(value.get("TMA_DSH_EXAMPLE_API_KEY").is_none());
        assert_eq!(value["OTHER_API_KEY"], json!("keep"));
    }

    #[test]
    fn deepseek_v4_models_stay_in_pi_ai() {
        let (files, warnings) = build_deepseek_preview_with_warnings(
            &endpoint(),
            &["deepseek-v4-flash".to_string(), "example-model".to_string()],
            Some("example-model".to_string()),
            &[(
                "deepseek-settings".to_string(),
                PathBuf::from("deepseek-preview-does-not-exist.yaml"),
                "yaml".to_string(),
            )],
            None,
        )
        .expect("preview should be generated");
        let value: serde_json::Value = serde_yaml::from_str(&files[0].content)
            .expect("generated settings should be valid YAML");

        assert!(warnings.is_empty());
        assert_eq!(
            value["llm-pi-ai"]["providers"]["tma-Example"]["models"],
            json!([
                {
                    "id": "deepseek-v4-flash",
                    "contextWindow": 1_000_000,
                    "maxTokens": 384_000,
                    "input": ["text"],
                    "reasoningEfforts": { "low": "low", "medium": "medium", "high": "high", "max": "max" }
                },
                { "id": "example-model" }
            ])
        );
        assert_eq!(
            value["agent-default-model"],
            json!({ "provider": "tma-Example", "model": "example-model" })
        );
    }

    #[test]
    fn deepseek_v4_models_can_use_the_generic_provider_by_default() {
        let (files, warnings) = build_deepseek_preview_with_warnings(
            &endpoint(),
            &["deepseek-v4-flash".to_string(), "example-model".to_string()],
            Some("deepseek-v4-flash".to_string()),
            &[(
                "deepseek-settings".to_string(),
                PathBuf::from("deepseek-preview-does-not-exist.yaml"),
                "yaml".to_string(),
            )],
            None,
        )
        .expect("generic provider settings should be generated");
        let value: serde_json::Value = serde_yaml::from_str(&files[0].content)
            .expect("generated settings should be valid YAML");

        assert!(warnings.is_empty());
        assert_eq!(
            value["llm-pi-ai"]["providers"]["tma-Example"]["models"],
            json!([
                {
                    "id": "deepseek-v4-flash",
                    "contextWindow": 1_000_000,
                    "maxTokens": 384_000,
                    "input": ["text"],
                    "reasoningEfforts": { "low": "low", "medium": "medium", "high": "high", "max": "max" }
                },
                { "id": "example-model" }
            ])
        );
        assert_eq!(
            value["agent-default-model"],
            json!({ "provider": "tma-Example", "model": "deepseek-v4-flash" })
        );
    }

    #[test]
    fn opencode_none_variant_maps_to_dsh_off() {
        let content = build_settings_content(
            Path::new("deepseek-settings-does-not-exist.yaml"),
            &endpoint(),
            "gpt-5.4",
        )
        .expect("settings should be generated");
        let value: serde_json::Value =
            serde_yaml::from_str(&content).expect("generated settings should be valid YAML");
        let reasoning =
            &value["llm-pi-ai"]["providers"]["tma-Example"]["models"][0]["reasoningEfforts"];

        assert!(reasoning.get("none").is_none());
        assert_eq!(reasoning["off"], serde_json::Value::Null);
        assert_eq!(reasoning["low"], json!("low"));
    }

    #[test]
    fn unsupported_reasoning_variants_do_not_leave_an_off_only_map() {
        let content = build_settings_content(
            Path::new("deepseek-settings-does-not-exist.yaml"),
            &endpoint(),
            "minimax-m3",
        )
        .expect("settings should be generated");
        let value: serde_json::Value =
            serde_yaml::from_str(&content).expect("generated settings should be valid YAML");
        let model = &value["llm-pi-ai"]["providers"]["tma-Example"]["models"][0];

        assert!(model.get("reasoningEfforts").is_none());
    }

    #[test]
    fn settings_merge_preserves_existing_plugins_and_providers() {
        let path = temporary_settings_path();
        fs::write(
            &path,
            "other-plugin:\n  enabled: true\nllm-pi-ai:\n  providers:\n    existing:\n      api: openai-completions\n",
        )
        .expect("test settings should be written");
        let result = build_settings_content(&path, &endpoint(), "example-model");
        let _ = fs::remove_file(&path);
        let content = result.expect("settings should be merged");
        let value: serde_json::Value =
            serde_yaml::from_str(&content).expect("merged settings should be valid YAML");

        assert_eq!(value["other-plugin"]["enabled"], json!(true));
        assert_eq!(
            value["llm-pi-ai"]["providers"]["existing"]["api"],
            json!("openai-completions")
        );
        assert_eq!(
            value["llm-pi-ai"]["providers"]["tma-Example"]["models"],
            json!([{ "id": "example-model" }])
        );
    }

    fn temporary_settings_path() -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "tma-deepseek-settings-{}-{timestamp}.yaml",
            std::process::id()
        ))
    }
}
