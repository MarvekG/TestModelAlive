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

    let exact_duplicate = settings.endpoints.iter().position(|endpoint| {
        endpoint.endpoint_type == endpoint_type
            && endpoint.base_url == base_url
            && endpoint.name == name
            && endpoint.api_key == api_key
    });

    if request.overwrite.unwrap_or(false) {
        if let Some(index) = exact_duplicate {
            let endpoint = &mut settings.endpoints[index];
            endpoint.api_key = api_key;
            endpoint.name = name;
            endpoint.models = models;
            let endpoint = endpoint.clone();
            write_app_settings_for_app(&app, &settings)?;
            return Ok(endpoint);
        }
    }

    if exact_duplicate.is_some() {
        return Err("endpoint already exists".to_string());
    }

    if settings
        .endpoints
        .iter()
        .any(|endpoint| endpoint.endpoint_type == endpoint_type && endpoint.name == name)
    {
        return Err("endpoint name already exists for this type".to_string());
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
    let models = fetch_endpoint_models(&app, &endpoint, request.timeout)?;
    emit_log(&app, &format!("backend fetched {} models", models.len()));
    Ok(models)
}

fn validate_endpoint_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("name is required".to_string());
    }
    if !name.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Err("name can only contain English letters and numbers".to_string());
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

fn fetch_endpoint_models(
    app: &tauri::AppHandle,
    endpoint: &SavedEndpoint,
    timeout: u64,
) -> Result<Vec<String>, String> {
    let base_url = endpoint.base_url.trim_end_matches('/');
    let url = if matches!(endpoint.endpoint_type.as_str(), "codex" | "opencode") {
        format!("{base_url}/models")
    } else if base_url.ends_with("/v1") {
        format!("{base_url}/models")
    } else {
        format!("{base_url}/v1/models")
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout))
        .build()
        .map_err(|err| err.to_string())?;
    match fetch_models_from_url(&client, endpoint, &url) {
        Ok(models) => Ok(models),
        Err(first_error) => {
            if url.ends_with("/v1/models") || url_path_ends_with_v1(base_url) {
                return Err(first_error);
            }
            let retry_url = format!("{base_url}/v1/models");
            emit_log(
                app,
                &format!("model fetch failed, retrying with v1 url: {retry_url}"),
            );
            fetch_models_from_url(&client, endpoint, &retry_url).map_err(|retry_error| {
                format!("{first_error}; retry with {retry_url} failed: {retry_error}")
            })
        }
    }
}

fn url_path_ends_with_v1(raw_url: &str) -> bool {
    reqwest::Url::parse(raw_url)
        .ok()
        .and_then(|url| {
            url.path_segments().map(|segments| {
                segments
                    .filter(|segment| !segment.is_empty())
                    .next_back()
                    .is_some_and(|segment| segment.eq_ignore_ascii_case("v1"))
            })
        })
        .unwrap_or_else(|| {
            raw_url
                .trim_end_matches('/')
                .split('/')
                .next_back()
                .is_some_and(|segment| segment.eq_ignore_ascii_case("v1"))
        })
}

fn fetch_models_from_url(
    client: &reqwest::blocking::Client,
    endpoint: &SavedEndpoint,
    url: &str,
) -> Result<Vec<String>, String> {
    let mut request = client.get(url).header("Accept", "application/json");
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
    tracing::info!(target: "backend", "{message}");
    let _ = app.emit(
        "test-log",
        LogEvent {
            message: message.to_string(),
            stream: false,
        },
    );
}
