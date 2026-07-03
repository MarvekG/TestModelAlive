use std::fs;

use crate::models::SavedEndpoint;
use crate::paths::store_path;
use crate::test_runner::process::TestCommand;

pub(crate) fn prepare_claude(
    app: &tauri::AppHandle,
    endpoint: &SavedEndpoint,
    model: &str,
    prompt: &str,
) -> Result<TestCommand, String> {
    let settings_path = store_path(app, "claude-settings.json")?;
    let settings = serde_json::json!({
        "env": {
            "ANTHROPIC_BASE_URL": endpoint.base_url,
            "ANTHROPIC_AUTH_TOKEN": endpoint.api_key,
        }
    });
    fs::write(
        &settings_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&settings).map_err(|err| err.to_string())?
        ),
    )
    .map_err(|err| err.to_string())?;
    Ok(TestCommand {
        program: "claude".to_string(),
        args: vec![
            "--debug".to_string(),
            "--verbose".to_string(),
            "--settings".to_string(),
            settings_path.to_string_lossy().to_string(),
            "--model".to_string(),
            model.to_string(),
            "-p".to_string(),
            prompt.to_string(),
        ],
        envs: Vec::new(),
        env_remove: Vec::new(),
    })
}

pub(crate) fn real_config_command(prompt: &str) -> TestCommand {
    TestCommand {
        program: "claude".to_string(),
        args: vec![
            "--debug".to_string(),
            "--verbose".to_string(),
            "-p".to_string(),
            prompt.to_string(),
        ],
        envs: Vec::new(),
        env_remove: Vec::new(),
    }
}
