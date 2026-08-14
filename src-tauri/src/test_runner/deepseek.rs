use serde_yaml::{Mapping, Value};
use std::fs;

use crate::cli_config::deepseek::{
    build_test_settings_content, dsh_api_key_env, dsh_api_key_env_for_endpoint_id,
    dsh_api_key_env_for_name, dsh_provider_name_for_endpoint_name, parse_yaml_mapping,
};
use crate::models::SavedEndpoint;
use crate::paths::{app_data_dir, dsh_home_dir};
use crate::test_runner::process::TestCommand;

pub(crate) fn prepare_deepseek(
    app: &tauri::AppHandle,
    endpoint: &SavedEndpoint,
    model: &str,
    prompt: &str,
) -> Result<TestCommand, String> {
    let dsh_home = app_data_dir(app)?.join("dsh-home");
    fs::create_dir_all(&dsh_home).map_err(|err| err.to_string())?;
    let settings_path = dsh_home.join("settings.yaml");
    let settings = build_test_settings_content(&settings_path, endpoint, model)?;
    fs::write(&settings_path, settings).map_err(|err| err.to_string())?;

    Ok(TestCommand {
        program: "dsh".to_string(),
        args: vec![
            "--profile".to_string(),
            "headless".to_string(),
            prompt.to_string(),
        ],
        envs: vec![
            (
                "DSH_HOME".to_string(),
                dsh_home.to_string_lossy().to_string(),
            ),
            (dsh_api_key_env(endpoint), endpoint.api_key.clone()),
            ("DSH_PERMISSION_MODE".to_string(), "read-only".to_string()),
            ("DSH_TELEMETRY_DISABLED".to_string(), "1".to_string()),
        ],
        env_remove: Vec::new(),
    })
}

pub(crate) fn configured_default_model(endpoint_name: &str) -> Result<String, String> {
    let path = dsh_home_dir()?.join("settings.yaml");
    let text = fs::read_to_string(&path).map_err(|err| err.to_string())?;
    let settings = parse_yaml_mapping(&path, &text)?;
    configured_default_model_from_settings(&settings, endpoint_name)
}

fn configured_default_model_from_settings(
    settings: &Mapping,
    endpoint_name: &str,
) -> Result<String, String> {
    let default_model = settings
        .get(&Value::String("agent-default-model".to_string()))
        .and_then(Value::as_mapping)
        .ok_or_else(|| "DeepSeek Harness agent-default-model is missing".to_string())?;
    let provider = mapping_string(default_model, "provider")
        .ok_or_else(|| "DeepSeek Harness default provider is missing".to_string())?;
    let expected_provider = dsh_provider_name_for_endpoint_name(endpoint_name);
    if provider != expected_provider && provider != "deepseek-official" {
        return Err(format!(
            "DeepSeek Harness default provider is {provider}, not {expected_provider} or deepseek-official"
        ));
    }
    mapping_string(default_model, "model")
        .map(str::to_string)
        .ok_or_else(|| "DeepSeek Harness default model is missing".to_string())
}

fn mapping_string<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a str> {
    mapping
        .get(&Value::String(key.to_string()))
        .and_then(Value::as_str)
}

pub(crate) fn real_config_command(
    endpoint_name: &str,
    endpoint_id: &str,
    prompt: &str,
) -> TestCommand {
    TestCommand {
        program: "dsh".to_string(),
        args: vec![
            "--profile".to_string(),
            "headless".to_string(),
            prompt.to_string(),
        ],
        envs: Vec::new(),
        // Force this check to use the credential file written by the applied configuration.
        env_remove: vec![
            dsh_api_key_env_for_endpoint_id(endpoint_id),
            dsh_api_key_env_for_name(endpoint_name),
        ],
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::cli_config::deepseek::parse_yaml_mapping;

    use super::configured_default_model_from_settings;

    #[test]
    fn reads_the_configured_default_for_the_expected_provider() {
        let settings = parse_yaml_mapping(
            Path::new("settings.yaml"),
            "agent-default-model:\n  provider: tma-Example\n  model: second-model\n",
        )
        .expect("settings should parse");

        assert_eq!(
            configured_default_model_from_settings(&settings, "Example"),
            Ok("second-model".to_string())
        );
    }

    #[test]
    fn rejects_a_default_for_another_provider() {
        let settings = parse_yaml_mapping(
            Path::new("settings.yaml"),
            "agent-default-model:\n  provider: tma-Other\n  model: model\n",
        )
        .expect("settings should parse");

        assert!(configured_default_model_from_settings(&settings, "Example").is_err());
    }

    #[test]
    fn accepts_the_direct_deepseek_provider() {
        let settings = parse_yaml_mapping(
            Path::new("settings.yaml"),
            "agent-default-model:\n  provider: deepseek-official\n  model: deepseek-v4-flash\n",
        )
        .expect("settings should parse");

        assert_eq!(
            configured_default_model_from_settings(&settings, "Example"),
            Ok("deepseek-v4-flash".to_string())
        );
    }
}
