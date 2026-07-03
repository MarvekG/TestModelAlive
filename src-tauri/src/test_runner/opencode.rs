use std::fs;

use crate::models::SavedEndpoint;
use crate::paths::app_data_dir;
use crate::test_runner::process::TestCommand;
use crate::test_runner::TestGuards;

pub(crate) fn prepare_opencode(
    app: &tauri::AppHandle,
    endpoint: &SavedEndpoint,
    model: &str,
    prompt: &str,
) -> Result<(TestCommand, TestGuards), String> {
    let opencode_home = app_data_dir(app)?.join("opencode-home");
    let config_path = opencode_home
        .join(".config")
        .join("opencode")
        .join("opencode.json");
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let config = serde_json::json!({
        "provider": {
            endpoint.name.clone(): {
                "npm": "@ai-sdk/openai-compatible",
                "options": {
                    "baseURL": endpoint.base_url,
                    "apiKey": endpoint.api_key
                },
                "models": {
                    model: { "name": model }
                }
            }
        }
    });
    fs::write(
        &config_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&config).map_err(|err| err.to_string())?
        ),
    )
    .map_err(|err| err.to_string())?;
    Ok((
        TestCommand {
            program: "opencode".to_string(),
            args: vec![
                "run".to_string(),
                "--model".to_string(),
                format!("{}/{model}", endpoint.name),
                prompt.to_string(),
            ],
            envs: vec![
                (
                    "HOME".to_string(),
                    opencode_home.to_string_lossy().to_string(),
                ),
                (
                    "USERPROFILE".to_string(),
                    opencode_home.to_string_lossy().to_string(),
                ),
            ],
            env_remove: Vec::new(),
        },
        Vec::new(),
    ))
}

pub(crate) fn real_config_command(provider: &str, model: &str, prompt: &str) -> TestCommand {
    TestCommand {
        program: "opencode".to_string(),
        args: vec![
            "run".to_string(),
            "--model".to_string(),
            format!("{provider}/{model}"),
            prompt.to_string(),
        ],
        envs: Vec::new(),
        env_remove: Vec::new(),
    }
}
