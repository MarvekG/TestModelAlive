use serde::Deserialize;
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::ipc::Channel;
use tauri::Manager;

pub(crate) mod claude;
pub(crate) mod codex;
pub(crate) mod opencode;
pub(crate) mod process;

use crate::cli_config::types::CliConfigTargetKind;
use crate::models::{SavedEndpoint, TestMessage, TestModelsRequest, TestResult};
use process::{
    emit_test_log, run_command, stop_requested, terminate_child, RestorableFile, TestCommand,
};

#[derive(Default)]
pub struct AppState {
    pub(crate) current_child: Mutex<Option<Arc<Mutex<Child>>>>,
    pub(crate) stop_requested: Mutex<bool>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RealCliTestRequest {
    pub target: CliConfigTargetKind,
    pub endpoint_name: Option<String>,
    pub models: Vec<String>,
    pub timeout: u64,
    pub prompt: String,
    pub success_keyword: String,
}

#[tauri::command]
pub fn stop_test(state: tauri::State<'_, AppState>) -> Result<(), String> {
    *state.stop_requested.lock().map_err(|err| err.to_string())? = true;
    let child = state
        .current_child
        .lock()
        .map_err(|err| err.to_string())?
        .clone();
    if let Some(child) = child {
        terminate_child(&child)?;
    }
    Ok(())
}

#[tauri::command]
pub fn test_models(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    request: TestModelsRequest,
    on_event: Channel<TestMessage>,
) -> Result<(), String> {
    *state.stop_requested.lock().map_err(|err| err.to_string())? = false;
    let app_handle = app.clone();
    std::thread::spawn(move || {
        let state = app_handle.state::<AppState>();
        let mut models = request.models;
        if request.endpoint.endpoint_type == "claude" && request.append_1m {
            models = models
                .into_iter()
                .map(|model| format!("{model}[1m]"))
                .collect();
        }
        emit_test_log(
            &app_handle,
            &on_event,
            &format!("starting test: {} models", models.len()),
        );
        for model in models {
            if stop_requested(&state) {
                break;
            }
            emit_test_log(&app_handle, &on_event, &format!("testing model: {model}"));
            let result = match run_model_test(
                &app_handle,
                &on_event,
                &state,
                &request.endpoint,
                &model,
                request.timeout,
                &request.prompt,
                &request.success_keyword,
            ) {
                Ok(result) => result,
                Err(err) => TestResult {
                    model: model.clone(),
                    status: "UNAVAILABLE".to_string(),
                    seconds: 0.0,
                    detail: err,
                },
            };
            let _ = on_event.send(TestMessage::Result {
                result: result.clone(),
            });
            emit_test_log(
                &app_handle,
                &on_event,
                &format!(
                    "MODEL_STATUS={} model={} elapsed={:.1}s",
                    result.status, result.model, result.seconds
                ),
            );
            if result.status == "STOPPED" {
                break;
            }
        }
        let _ = on_event.send(TestMessage::Finished);
        emit_test_log(&app_handle, &on_event, "test finished");
    });
    Ok(())
}

#[tauri::command]
pub fn test_cli_with_real_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    request: RealCliTestRequest,
    on_event: Channel<TestMessage>,
) -> Result<(), String> {
    validate_real_cli_test_request(&request)?;
    *state.stop_requested.lock().map_err(|err| err.to_string())? = false;
    let app_handle = app.clone();
    std::thread::spawn(move || {
        let state = app_handle.state::<AppState>();
        emit_test_log(
            &app_handle,
            &on_event,
            &format!(
                "starting real CLI config test: target={}",
                request.target.as_str()
            ),
        );
        for model in request.models {
            if stop_requested(&state) {
                break;
            }
            let start = Instant::now();
            let command = real_config_command(
                &request.target,
                request.endpoint_name.as_deref(),
                &model,
                &request.prompt,
            );
            let result = match run_command(
                &app_handle,
                &on_event,
                &state,
                command,
                request.timeout,
                &request.success_keyword,
            ) {
                Ok((status, detail)) => TestResult {
                    model: model.clone(),
                    status,
                    seconds: start.elapsed().as_secs_f64(),
                    detail,
                },
                Err(err) => TestResult {
                    model: model.clone(),
                    status: "UNAVAILABLE".to_string(),
                    seconds: start.elapsed().as_secs_f64(),
                    detail: err,
                },
            };
            let _ = on_event.send(TestMessage::Result {
                result: result.clone(),
            });
            emit_test_log(
                &app_handle,
                &on_event,
                &format!(
                    "MODEL_STATUS={} model={} elapsed={:.1}s",
                    result.status, result.model, result.seconds
                ),
            );
            if result.status == "STOPPED" {
                break;
            }
        }
        let _ = on_event.send(TestMessage::Finished);
        emit_test_log(&app_handle, &on_event, "real CLI config test finished");
    });
    Ok(())
}

fn run_model_test(
    app: &tauri::AppHandle,
    on_event: &Channel<TestMessage>,
    state: &tauri::State<'_, AppState>,
    endpoint: &SavedEndpoint,
    model: &str,
    timeout: u64,
    prompt: &str,
    success_keyword: &str,
) -> Result<TestResult, String> {
    let start = Instant::now();
    let (command, guards) = if endpoint.endpoint_type == "codex" {
        codex::prepare_codex(app, endpoint, model, prompt)?
    } else {
        claude::prepare_claude(app, endpoint, model, prompt)?
    };
    let (status, detail) = run_command(app, on_event, state, command, timeout, success_keyword)?;
    drop(guards);
    Ok(TestResult {
        model: model.to_string(),
        status,
        seconds: start.elapsed().as_secs_f64(),
        detail,
    })
}

fn real_config_command(
    target: &CliConfigTargetKind,
    endpoint_name: Option<&str>,
    model: &str,
    prompt: &str,
) -> TestCommand {
    match target {
        CliConfigTargetKind::Codex => codex::real_config_command(prompt),
        CliConfigTargetKind::Claude => claude::real_config_command(prompt),
        CliConfigTargetKind::Opencode => {
            opencode::real_config_command(endpoint_name.unwrap_or(""), model, prompt)
        }
    }
}

fn validate_real_cli_test_request(request: &RealCliTestRequest) -> Result<(), String> {
    match request.target {
        CliConfigTargetKind::Codex | CliConfigTargetKind::Claude => {
            if request.models.len() != 1 {
                return Err(
                    "Codex and Claude real config tests require exactly one selected model"
                        .to_string(),
                );
            }
        }
        CliConfigTargetKind::Opencode => {
            if request
                .endpoint_name
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                return Err("OpenCode real config test requires an endpoint name".to_string());
            }
            if request.models.is_empty() {
                return Err(
                    "OpenCode real config test requires at least one selected model".to_string(),
                );
            }
        }
    }
    Ok(())
}

pub(crate) type TestGuards = Vec<RestorableFile>;
