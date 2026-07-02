use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{ipc::Channel, Emitter, Manager};
use time::OffsetDateTime;

const DATA_FILE: &str = "tsa_endpoints.json";
const TEST_SETTINGS_FILE: &str = "test_settings.json";
const DEFAULT_SUCCESS_KEYWORD: &str = "OKK";
const DEFAULT_PROMPT: &str = "You must output exactly OKK and nothing else. Do not explain. Do not add punctuation.";

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SavedEndpoint {
    id: String,
    #[serde(rename = "type")]
    endpoint_type: String,
    base_url: String,
    api_key: String,
    models: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct EndpointStore {
    version: u8,
    endpoints: Vec<SavedEndpoint>,
}

#[derive(Debug, Deserialize)]
struct AddEndpointRequest {
    #[serde(rename = "type")]
    endpoint_type: String,
    base_url: String,
    api_key: String,
    models: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FetchModelsRequest {
    #[serde(rename = "type")]
    endpoint_type: String,
    base_url: String,
    api_key: String,
    timeout: u64,
}

#[derive(Debug, Deserialize)]
struct TestModelsRequest {
    endpoint: SavedEndpoint,
    models: Vec<String>,
    timeout: u64,
    append_1m: bool,
    prompt: String,
    success_keyword: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TestSettings {
    prompt: String,
    success_keyword: String,
}

#[derive(Clone, Debug, Serialize)]
struct TestResult {
    model: String,
    status: String,
    seconds: f64,
    detail: String,
}

#[derive(Clone, Debug, Serialize)]
struct LogEvent {
    message: String,
    stream: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind")]
enum TestMessage {
    #[serde(rename = "log")]
    Log { message: String, stream: bool },
    #[serde(rename = "result")]
    Result { result: TestResult },
    #[serde(rename = "finished")]
    Finished,
}

#[derive(Default)]
struct AppState {
    current_child: Mutex<Option<Arc<Mutex<Child>>>>,
    stop_requested: Mutex<bool>,
}

struct RestorableFile {
    path: PathBuf,
    backup: Option<PathBuf>,
    existed: bool,
}

struct TestCommand {
    program: String,
    args: Vec<String>,
    envs: Vec<(String, String)>,
}

impl RestorableFile {
    fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            backup: None,
            existed: false,
        }
    }

    fn begin(&mut self) -> Result<(), String> {
        self.existed = self.path.exists();
        if self.existed {
            let backup = next_backup_path(&self.path)?;
            if let Some(parent) = backup.parent() {
                fs::create_dir_all(parent).map_err(|err| err.to_string())?;
            }
            fs::copy(&self.path, &backup).map_err(|err| err.to_string())?;
            self.backup = Some(backup);
        }
        Ok(())
    }
}

impl Drop for RestorableFile {
    fn drop(&mut self) {
        if let Some(backup) = &self.backup {
            if let Some(parent) = self.path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::copy(backup, &self.path);
            let _ = fs::remove_file(backup);
        } else if !self.existed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            load_endpoints,
            add_endpoint,
            delete_endpoint,
            load_test_settings,
            save_test_settings,
            fetch_models,
            test_models,
            stop_test
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn load_endpoints(app: tauri::AppHandle) -> Result<Vec<SavedEndpoint>, String> {
    Ok(read_store(&store_path(&app, DATA_FILE)?)?.endpoints)
}

#[tauri::command]
fn add_endpoint(app: tauri::AppHandle, request: AddEndpointRequest) -> Result<SavedEndpoint, String> {
    let path = store_path(&app, DATA_FILE)?;
    let mut store = read_store(&path)?;
    let endpoint_type = request.endpoint_type.trim().to_string();
    let endpoint = SavedEndpoint {
        id: new_id(&endpoint_type, &store.endpoints)?,
        endpoint_type,
        base_url: request.base_url.trim().trim_end_matches('/').to_string(),
        api_key: request.api_key.trim().to_string(),
        models: request
            .models
            .into_iter()
            .map(|model| model.trim().to_string())
            .filter(|model| !model.is_empty())
            .collect(),
    };
    store.endpoints.push(endpoint.clone());
    write_store(&path, &store)?;
    Ok(endpoint)
}

#[tauri::command]
fn delete_endpoint(app: tauri::AppHandle, endpoint_id: String) -> Result<(), String> {
    let path = store_path(&app, DATA_FILE)?;
    let mut store = read_store(&path)?;
    store.endpoints.retain(|endpoint| endpoint.id != endpoint_id);
    write_store(&path, &store)
}

#[tauri::command]
fn load_test_settings(app: tauri::AppHandle) -> Result<TestSettings, String> {
    let path = store_path(&app, TEST_SETTINGS_FILE)?;
    if !path.exists() {
        return Ok(default_test_settings());
    }
    let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let mut settings: TestSettings = serde_json::from_str(&text).map_err(|err| err.to_string())?;
    if settings.success_keyword.trim().is_empty() {
        settings.success_keyword = DEFAULT_SUCCESS_KEYWORD.to_string();
    }
    if settings.prompt.trim().is_empty() {
        settings.prompt = DEFAULT_PROMPT.to_string();
    }
    Ok(settings)
}

#[tauri::command]
fn save_test_settings(app: tauri::AppHandle, settings: TestSettings) -> Result<(), String> {
    let prompt = settings.prompt.trim().to_string();
    let success_keyword = settings.success_keyword.trim().to_string();
    if prompt.is_empty() {
        return Err("prompt is required".to_string());
    }
    if success_keyword.is_empty() {
        return Err("success keyword is required".to_string());
    }
    if !prompt.contains(&success_keyword) {
        return Err("prompt must contain success keyword".to_string());
    }
    let settings = TestSettings { prompt, success_keyword };
    let text = serde_json::to_string_pretty(&settings).map_err(|err| err.to_string())?;
    fs::write(store_path(&app, TEST_SETTINGS_FILE)?, format!("{text}\n")).map_err(|err| err.to_string())
}

#[tauri::command]
fn fetch_models(app: tauri::AppHandle, request: FetchModelsRequest) -> Result<Vec<String>, String> {
    let endpoint = SavedEndpoint {
        id: String::new(),
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

#[tauri::command]
fn stop_test(state: tauri::State<'_, AppState>) -> Result<(), String> {
    *state.stop_requested.lock().map_err(|err| err.to_string())? = true;
    let child = state.current_child.lock().map_err(|err| err.to_string())?.clone();
    if let Some(child) = child {
        terminate_child(&child)?;
    }
    Ok(())
}

#[tauri::command]
fn test_models(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    request: TestModelsRequest,
    on_event: Channel<TestMessage>,
) -> Result<(), String> {
    {
        *state.stop_requested.lock().map_err(|err| err.to_string())? = false;
    }
    let app_handle = app.clone();
    std::thread::spawn(move || {
        let state = app_handle.state::<AppState>();
        let mut models = request.models;
        if request.endpoint.endpoint_type == "claude" && request.append_1m {
            models = models.into_iter().map(|model| format!("{model}[1m]")).collect();
        }
        emit_test_log(&app_handle, &on_event, &format!("starting test: {} models", models.len()));
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
            let _ = on_event.send(TestMessage::Result { result: result.clone() });
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

fn read_store(path: &Path) -> Result<EndpointStore, String> {
    if !path.exists() {
        return Ok(EndpointStore {
            version: 1,
            endpoints: Vec::new(),
        });
    }
    let text = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let mut store: EndpointStore = serde_json::from_str(&text).map_err(|err| err.to_string())?;
    store.endpoints.retain(|endpoint| {
        !endpoint.id.is_empty() && !endpoint.endpoint_type.is_empty() && !endpoint.base_url.is_empty()
    });
    Ok(store)
}

fn write_store(path: &Path, store: &EndpointStore) -> Result<(), String> {
    let text = serde_json::to_string_pretty(store).map_err(|err| err.to_string())?;
    fs::write(path, format!("{text}\n")).map_err(|err| err.to_string())
}

fn store_path(app: &tauri::AppHandle, file_name: &str) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let path = dir.join(file_name);
    let legacy_path = Path::new(file_name);
    if !path.exists() && legacy_path.exists() {
        fs::copy(legacy_path, &path).map_err(|err| err.to_string())?;
    }
    Ok(path)
}

fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = user_home_dir()
        .or_else(|_| app.path().app_data_dir().map_err(|err| err.to_string()))?
        .join(".TestModelAlive");
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir)
}

fn user_home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| "HOME/USERPROFILE is not set".to_string())
}

fn default_test_settings() -> TestSettings {
    TestSettings {
        prompt: DEFAULT_PROMPT.to_string(),
        success_keyword: DEFAULT_SUCCESS_KEYWORD.to_string(),
    }
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
    let url = if endpoint.endpoint_type == "codex" {
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
    if endpoint.endpoint_type == "codex" {
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
            item.as_str()
                .map(ToOwned::to_owned)
                .or_else(|| item.get("id").and_then(Value::as_str).map(ToOwned::to_owned))
        })
        .collect::<Vec<_>>();
    models.sort();
    models.dedup();
    Ok(models)
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
        prepare_codex(app, endpoint, model, prompt)?
    } else {
        prepare_claude(app, endpoint, model, prompt)?
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

fn prepare_codex(app: &tauri::AppHandle, endpoint: &SavedEndpoint, model: &str, prompt: &str) -> Result<(TestCommand, Vec<RestorableFile>), String> {
    let codex_dir = app_data_dir(app)?.join("codex-home");
    fs::create_dir_all(&codex_dir).map_err(|err| err.to_string())?;
    let auth_path = codex_dir.join("auth.json");
    let config_path = codex_dir.join("config.toml");
    fs::write(
        &auth_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&serde_json::json!({ "OPENAI_API_KEY": endpoint.api_key }))
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
                "--ignore-user-config".to_string(),
                prompt.to_string(),
            ],
            envs: vec![("CODEX_HOME".to_string(), codex_dir.to_string_lossy().to_string())],
        },
        Vec::new(),
    ))
}

fn prepare_claude(app: &tauri::AppHandle, endpoint: &SavedEndpoint, model: &str, prompt: &str) -> Result<(TestCommand, Vec<RestorableFile>), String> {
    let settings_path = store_path(app, "claude-settings.json")?;
    let mut guard = RestorableFile::new(&settings_path);
    guard.begin()?;
    let settings = serde_json::json!({
        "env": {
            "ANTHROPIC_BASE_URL": endpoint.base_url,
            "ANTHROPIC_AUTH_TOKEN": endpoint.api_key,
        }
    });
    fs::write(
        &settings_path,
        format!("{}\n", serde_json::to_string_pretty(&settings).map_err(|err| err.to_string())?),
    )
    .map_err(|err| err.to_string())?;
    Ok((
        TestCommand {
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
        },
        vec![guard],
    ))
}

fn run_command(
    app: &tauri::AppHandle,
    on_event: &Channel<TestMessage>,
    state: &tauri::State<'_, AppState>,
    command: TestCommand,
    timeout: u64,
    success_keyword: &str,
) -> Result<(String, String), String> {
    let program_path = resolve_program(&command.program)?;
    emit_test_log(app, on_event, &format!("running: {}", shell_join(&program_path, &command.args)));
    let mut process = command_process(&program_path, &command.args);
    process.envs(command.envs.iter().map(|(key, value)| (key, value))).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            process.pre_exec(|| {
                nix::unistd::setsid().map_err(std::io::Error::other)?;
                Ok(())
            });
        }
    }
    let child = Arc::new(Mutex::new(process.spawn().map_err(|err| {
        let message = format!("failed to start command: {err}");
        emit_test_log(app, on_event, &message);
        message
    })?));
    *state.current_child.lock().map_err(|err| err.to_string())? = Some(child.clone());
    let stdout = child
        .lock()
        .map_err(|err| err.to_string())?
        .stdout
        .take()
        .ok_or_else(|| "failed to capture stdout".to_string())?;
    let stderr = child
        .lock()
        .map_err(|err| err.to_string())?
        .stderr
        .take()
        .ok_or_else(|| "failed to capture stderr".to_string())?;
    let output = Arc::new(Mutex::new(String::new()));
    let stdout_reader = spawn_stream_reader(on_event.clone(), stdout, output.clone(), false);
    let stderr_reader = spawn_stream_reader(on_event.clone(), stderr, output.clone(), true);

    let deadline = Instant::now() + Duration::from_secs(timeout);
    let status = loop {
        if stop_requested(state) {
            terminate_child(&child)?;
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            *state.current_child.lock().map_err(|err| err.to_string())? = None;
            return Ok(("STOPPED".to_string(), "stopped by user".to_string()));
        }
        if Instant::now() > deadline {
            terminate_child(&child)?;
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            *state.current_child.lock().map_err(|err| err.to_string())? = None;
            return Ok(("UNAVAILABLE".to_string(), format!("timeout after {timeout}s")));
        }
        if let Some(status) = child.lock().map_err(|err| err.to_string())?.try_wait().map_err(|err| err.to_string())? {
            break status;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let _ = stdout_reader.join();
    let _ = stderr_reader.join();
    *state.current_child.lock().map_err(|err| err.to_string())? = None;
    let output = output.lock().map_err(|err| err.to_string())?.clone();
    let detail = tail_chars(&output, 1000);
    if status.success() && output.contains(success_keyword) {
        return Ok(("AVAILABLE".to_string(), detail));
    }
    if status.success() {
        return Ok((
            "UNAVAILABLE".to_string(),
            format!("command exited 0 but did not return expected '{success_keyword}'\n{detail}"),
        ));
    }
    Ok(("UNAVAILABLE".to_string(), detail))
}

fn spawn_stream_reader<R: Read + Send + 'static>(
    on_event: Channel<TestMessage>,
    mut stream: R,
    output: Arc<Mutex<String>>,
    is_stderr: bool,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 1024];
        loop {
            let count = match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => count,
                Err(_) => break,
            };
            let chunk = String::from_utf8_lossy(&buffer[..count]).to_string();
            let _ = on_event.send(TestMessage::Log {
                message: chunk.clone(),
                stream: true,
            });
            if is_stderr {
                let _ = std::io::stderr().write_all(&buffer[..count]);
                let _ = std::io::stderr().flush();
            } else {
                let _ = std::io::stdout().write_all(&buffer[..count]);
                let _ = std::io::stdout().flush();
            }
            if let Ok(mut output) = output.lock() {
                output.push_str(&chunk);
            }
        }
    })
}

fn terminate_child(child: &Arc<Mutex<Child>>) -> Result<(), String> {
    let mut child = child.lock().map_err(|err| err.to_string())?;
    #[cfg(unix)]
    {
        let pid = nix::unistd::Pid::from_raw(child.id() as i32);
        let _ = nix::sys::signal::killpg(pid, nix::sys::signal::Signal::SIGTERM);
        std::thread::sleep(Duration::from_millis(500));
        if child.try_wait().map_err(|err| err.to_string())?.is_none() {
            let _ = nix::sys::signal::killpg(pid, nix::sys::signal::Signal::SIGKILL);
        }
    }
    #[cfg(not(unix))]
    {
        let pid = child.id().to_string();
        let status = Command::new("taskkill")
            .args(["/PID", &pid, "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if status.is_err() {
            let _ = child.kill();
        }
    }
    Ok(())
}

fn next_backup_path(path: &Path) -> Result<PathBuf, String> {
    let stamp = OffsetDateTime::now_utc().unix_timestamp_nanos();
    for index in 0..1000 {
        let name = format!("{}.{}.{}.bak", path.file_name().unwrap_or_default().to_string_lossy(), stamp, index);
        let candidate = path.with_file_name(name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!("could not allocate backup path for {}", path.display()))
}

fn stop_requested(state: &tauri::State<'_, AppState>) -> bool {
    state.stop_requested.lock().map(|value| *value).unwrap_or(true)
}

fn emit_log(app: &tauri::AppHandle, message: &str) {
    println!("{message}");
    let _ = app.emit(
        "test-log",
        LogEvent {
            message: message.to_string(),
            stream: false,
        },
    );
}

fn emit_test_log(app: &tauri::AppHandle, on_event: &Channel<TestMessage>, message: &str) {
    println!("{message}");
    let _ = on_event.send(TestMessage::Log {
        message: message.to_string(),
        stream: false,
    });
    let _ = app.emit(
        "test-log",
        LogEvent {
            message: message.to_string(),
            stream: false,
        },
    );
}

fn tail_chars(value: &str, limit: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let start = chars.len().saturating_sub(limit);
    chars[start..].iter().collect()
}

fn resolve_program(program: &str) -> Result<PathBuf, String> {
    let program_path = Path::new(program);
    if program_path.components().count() > 1 && program_path.exists() {
        return Ok(program_path.to_path_buf());
    }
    for dir in cli_search_paths() {
        for candidate in executable_candidates(&dir, program) {
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(format!("{program} command not found in PATH or common install locations"))
}

fn cli_search_paths() -> Vec<PathBuf> {
    let mut paths = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            paths.push(PathBuf::from(appdata).join("npm"));
        }
        if let Some(userprofile) = std::env::var_os("USERPROFILE") {
            paths.push(PathBuf::from(userprofile).join("AppData").join("Roaming").join("npm"));
        }
    }
    #[cfg(not(windows))]
    {
        paths.push(PathBuf::from("/usr/local/bin"));
        paths.push(PathBuf::from("/opt/homebrew/bin"));
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            paths.push(home.join(".local/bin"));
            paths.push(home.join(".npm-global/bin"));
        }
    }
    paths
}

fn executable_candidates(dir: &Path, program: &str) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        ["", ".exe", ".cmd", ".bat", ".ps1"]
            .iter()
            .map(|extension| dir.join(format!("{program}{extension}")))
            .collect()
    }
    #[cfg(not(windows))]
    {
        vec![dir.join(program)]
    }
}

fn command_process(program: &Path, args: &[String]) -> Command {
    #[cfg(windows)]
    {
        let extension = program.extension().and_then(|value| value.to_str()).unwrap_or_default();
        if extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat") {
            let mut command = Command::new("cmd");
            command.arg("/C").arg(program).args(args);
            return command;
        }
    }
    let mut command = Command::new(program);
    command.args(args);
    command
}

fn shell_join(program: &Path, args: &[String]) -> String {
    std::iter::once(program.to_string_lossy().to_string())
        .chain(args.iter().cloned())
        .map(|part| shell_quote(&part))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(value: &str) -> String {
    #[cfg(windows)]
    {
        if value.chars().all(|ch| ch.is_ascii_alphanumeric() || "-_./:=\\".contains(ch)) {
            return value.to_string();
        }
        return format!("\"{}\"", value.replace('"', "\\\""));
    }
    #[cfg(not(windows))]
    {
    if value.chars().all(|ch| ch.is_ascii_alphanumeric() || "-_./:=+".contains(ch)) {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
    }
}
