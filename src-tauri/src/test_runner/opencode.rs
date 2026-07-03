use crate::test_runner::process::TestCommand;

pub(crate) fn real_config_command(model: &str, prompt: &str) -> TestCommand {
    TestCommand {
        program: "opencode".to_string(),
        args: vec![
            "run".to_string(),
            "--model".to_string(),
            format!("testmodelalive/{model}"),
            prompt.to_string(),
        ],
        envs: Vec::new(),
        env_remove: Vec::new(),
    }
}
