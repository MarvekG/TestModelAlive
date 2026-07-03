use crate::test_runner::process::TestCommand;

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
