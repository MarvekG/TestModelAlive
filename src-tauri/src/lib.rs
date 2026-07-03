mod cli_config;
mod endpoints;
mod models;
mod paths;
mod settings;
mod test_runner;

pub fn run() {
    tauri::Builder::default()
        .manage(test_runner::AppState::default())
        .invoke_handler(tauri::generate_handler![
            endpoints::load_endpoints,
            endpoints::add_endpoint,
            endpoints::delete_endpoint,
            settings::load_test_settings,
            settings::save_test_settings,
            endpoints::fetch_models,
            test_runner::test_models,
            test_runner::stop_test,
            cli_config::commands::build_cli_config_preview,
            cli_config::commands::apply_cli_config,
            cli_config::commands::load_cli_config_baseline_items,
            cli_config::commands::restore_original_cli_config,
            test_runner::test_cli_with_real_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
