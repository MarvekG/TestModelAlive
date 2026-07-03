use std::fs;

use crate::models::SavedEndpoint;
use crate::paths::app_data_dir;
use crate::test_runner::process::TestCommand;
use crate::test_runner::TestGuards;

pub(crate) fn prepare_codex(
    app: &tauri::AppHandle,
    endpoint: &SavedEndpoint,
    model: &str,
    prompt: &str,
) -> Result<(TestCommand, TestGuards), String> {
    let codex_dir = app_data_dir(app)?.join("codex-home");
    fs::create_dir_all(&codex_dir).map_err(|err| err.to_string())?;
    let auth_path = codex_dir.join("auth.json");
    let config_path = codex_dir.join("config.toml");
    fs::write(
        &auth_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(
                &serde_json::json!({ "OPENAI_API_KEY": endpoint.api_key })
            )
            .map_err(|err| err.to_string())?
        ),
    )
    .map_err(|err| err.to_string())?;
    let config = format!(
        "model_provider = \"custom\"\nmodel = {}\nmodel_reasoning_effort = \"high\"\ndisable_response_storage = true\n\n[model_providers.custom]\nname = \"model-test\"\nbase_url = {}\nwire_api = \"responses\"\nrequires_openai_auth = true\n",
        serde_json::to_string(model).map_err(|err| err.to_string())?,
        serde_json::to_string(&endpoint.base_url).map_err(|err| err.to_string())?
    );
    fs::write(config_path, config).map_err(|err| err.to_string())?;
    Ok((
        TestCommand {
            program: "codex".to_string(),
            args: vec![
                "exec".to_string(),
                "--skip-git-repo-check".to_string(),
                prompt.to_string(),
            ],
            envs: vec![(
                "CODEX_HOME".to_string(),
                codex_dir.to_string_lossy().to_string(),
            )],
            env_remove: Vec::new(),
        },
        Vec::new(),
    ))
}

pub(crate) fn real_config_command(prompt: &str) -> TestCommand {
    TestCommand {
        program: "codex".to_string(),
        args: vec![
            "exec".to_string(),
            "--skip-git-repo-check".to_string(),
            prompt.to_string(),
        ],
        envs: Vec::new(),
        env_remove: vec!["CODEX_HOME".to_string()],
    }
}
