use serde_json::Value;
use std::time::Duration;
use tauri::Emitter;
use time::OffsetDateTime;

use crate::models::{AddEndpointRequest, FetchModelsRequest, LogEvent, SavedEndpoint};
use crate::settings::{read_app_settings, write_app_settings_for_app};

#[tauri::command]
pub fn load_endpoints(app: tauri::AppHandle) -> Result<Vec<SavedEndpoint>, String> {
    Ok(read_app_settings(&app)?.endpoints)
}

#[tauri::command]
pub fn add_endpoint(
    app: tauri::AppHandle,
    request: AddEndpointRequest,
) -> Result<SavedEndpoint, String> {
    let mut settings = read_app_settings(&app)?;
    let name = request.name.trim().to_string();
    validate_endpoint_name(&name)?;
    let endpoint_type = request.endpoint_type.trim().to_string();
    validate_endpoint_type(&endpoint_type)?;
    let base_url = request.base_url.trim().trim_end_matches('/').to_string();
    let api_key = request.api_key.trim().to_string();
    let models = request
        .models
        .into_iter()
        .map(|model| model.trim().to_string())
        .filter(|model| !model.is_empty())
        .collect();

    if request.overwrite.unwrap_or(false) {
        if let Some(endpoint) = settings.endpoints.iter_mut().find(|endpoint| {
            endpoint.endpoint_type == endpoint_type && endpoint.base_url == base_url
        }) {
            endpoint.api_key = api_key;
            endpoint.name = name;
            endpoint.models = models;
            let endpoint = endpoint.clone();
            write_app_settings_for_app(&app, &settings)?;
            return Ok(endpoint);
        }
    }

    let endpoint = SavedEndpoint {
        id: new_id(&endpoint_type, &settings.endpoints)?,
        name,
        endpoint_type,
        base_url,
        api_key,
        models,
    };
    settings.endpoints.push(endpoint.clone());
    write_app_settings_for_app(&app, &settings)?;
    Ok(endpoint)
}

#[tauri::command]
pub fn delete_endpoint(app: tauri::AppHandle, endpoint_id: String) -> Result<(), String> {
    let mut settings = read_app_settings(&app)?;
    settings
        .endpoints
        .retain(|endpoint| endpoint.id != endpoint_id);
    write_app_settings_for_app(&app, &settings)
}

#[tauri::command]
pub fn fetch_models(
    app: tauri::AppHandle,
    request: FetchModelsRequest,
) -> Result<Vec<String>, String> {
    let endpoint = SavedEndpoint {
        id: String::new(),
        name: String::new(),
        endpoint_type: request.endpoint_type,
        base_url: request.base_url.trim().trim_end_matches('/').to_string(),
        api_key: request.api_key.trim().to_string(),
        models: Vec::new(),
    };
    emit_log(
        &app,
        &format!(
            "fetching models from backend: type={} url={}",
            endpoint.endpoint_type, endpoint.base_url
        ),
    );
    let models = fetch_endpoint_models(&endpoint, request.timeout)?;
    emit_log(&app, &format!("backend fetched {} models", models.len()));
    Ok(models)
}

fn validate_endpoint_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("name is required".to_string());
    }
    if !name.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return Err("name can only contain English letters".to_string());
    }
    Ok(())
}

fn validate_endpoint_type(endpoint_type: &str) -> Result<(), String> {
    if matches!(endpoint_type, "codex" | "claude" | "opencode") {
        return Ok(());
    }
    Err("type must be codex, claude, or opencode".to_string())
}

fn new_id(endpoint_type: &str, endpoints: &[SavedEndpoint]) -> Result<String, String> {
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    let stamp = format!(
        "{:04}{:02}{:02}{:02}{:02}{:02}",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );
    for index in 1..1000 {
        let candidate = format!("{endpoint_type}-{stamp}-{index:03}");
        if !endpoints.iter().any(|endpoint| endpoint.id == candidate) {
            return Ok(candidate);
        }
    }
    Err("could not allocate endpoint id".to_string())
}

fn fetch_endpoint_models(endpoint: &SavedEndpoint, timeout: u64) -> Result<Vec<String>, String> {
    let url = if matches!(endpoint.endpoint_type.as_str(), "codex" | "opencode") {
        format!("{}/models", endpoint.base_url.trim_end_matches('/'))
    } else if endpoint.base_url.trim_end_matches('/').ends_with("/v1") {
        format!("{}/models", endpoint.base_url.trim_end_matches('/'))
    } else {
        format!("{}/v1/models", endpoint.base_url.trim_end_matches('/'))
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout))
        .build()
        .map_err(|err| err.to_string())?;
    let mut request = client.get(&url).header("Accept", "application/json");
    if matches!(endpoint.endpoint_type.as_str(), "codex" | "opencode") {
        request = request.header("Authorization", format!("Bearer {}", endpoint.api_key));
    } else {
        request = request
            .header("Authorization", format!("Bearer {}", endpoint.api_key))
            .header("X-Api-Key", &endpoint.api_key)
            .header("Anthropic-Version", "2023-06-01");
    }
    let payload: Value = request
        .send()
        .map_err(|err| err.to_string())?
        .error_for_status()
        .map_err(|err| err.to_string())?
        .json()
        .map_err(|err| err.to_string())?;
    let raw_models = payload
        .get("data")
        .or_else(|| payload.get("models"))
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{url} did not return a JSON object with data/models: []"))?;
    let mut models = raw_models
        .iter()
        .filter_map(|item| {
            item.as_str().map(ToOwned::to_owned).or_else(|| {
                item.get("id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
        })
        .collect::<Vec<_>>();
    models.sort();
    models.dedup();
    Ok(models)
}

fn emit_log(app: &tauri::AppHandle, message: &str) {
    let _ = app.emit(
        "test-log",
        LogEvent {
            message: message.to_string(),
            stream: false,
        },
    );
}
