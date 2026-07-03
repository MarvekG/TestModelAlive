use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::ipc::Channel;
use tauri::Emitter;

use crate::models::{LogEvent, TestMessage};
use crate::test_runner::AppState;

pub(crate) struct TestCommand {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) envs: Vec<(String, String)>,
    pub(crate) env_remove: Vec<String>,
}

pub(crate) fn run_command(
    app: &tauri::AppHandle,
    on_event: &Channel<TestMessage>,
    state: &tauri::State<'_, AppState>,
    command: TestCommand,
    timeout: u64,
    success_keyword: &str,
) -> Result<(String, String), String> {
    let program_path = match resolve_program(&command.program) {
        Ok(program_path) => program_path,
        Err(err) => {
            let message = format!("failed to find CLI binary: {err}");
            emit_test_log(app, on_event, &message);
            return Err(message);
        }
    };
    emit_test_log(
        app,
        on_event,
        &format!("running: {}", shell_join(&program_path, &command.args)),
    );
    let mut process = command_process(&program_path, &command.args);
    for key in &command.env_remove {
        process.env_remove(key);
    }
    process
        .envs(command.envs.iter().map(|(key, value)| (key, value)))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
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
    let stdout_reader = spawn_stream_reader(on_event.clone(), stdout, output.clone());
    let stderr_reader = spawn_stream_reader(on_event.clone(), stderr, output.clone());

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
            return Ok((
                "UNAVAILABLE".to_string(),
                format!("timeout after {timeout}s"),
            ));
        }
        if let Some(status) = child
            .lock()
            .map_err(|err| err.to_string())?
            .try_wait()
            .map_err(|err| err.to_string())?
        {
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
            if let Ok(mut output) = output.lock() {
                output.push_str(&chunk);
            }
        }
    })
}

pub(crate) fn terminate_child(child: &Arc<Mutex<Child>>) -> Result<(), String> {
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

pub(crate) fn stop_requested(state: &tauri::State<'_, AppState>) -> bool {
    state
        .stop_requested
        .lock()
        .map(|value| *value)
        .unwrap_or(true)
}

pub(crate) fn emit_test_log(
    app: &tauri::AppHandle,
    on_event: &Channel<TestMessage>,
    message: &str,
) {
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
    Err(format!(
        "{program} command not found in PATH or common install locations"
    ))
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
            paths.push(
                PathBuf::from(userprofile)
                    .join("AppData")
                    .join("Roaming")
                    .join("npm"),
            );
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
        [".exe", ".cmd", ".bat", ".ps1", ""]
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
        use std::os::windows::process::CommandExt;

        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let extension = program
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat") {
            let mut command = Command::new("cmd");
            command.arg("/C").arg(program).args(args);
            command.creation_flags(CREATE_NO_WINDOW);
            return command;
        }

        let mut command = Command::new(program);
        command.args(args);
        command.creation_flags(CREATE_NO_WINDOW);
        return command;
    }

    #[cfg(not(windows))]
    {
        let mut command = Command::new(program);
        command.args(args);
        command
    }
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
        if value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "-_./:=\\".contains(ch))
        {
            return value.to_string();
        }
        return format!("\"{}\"", value.replace('"', "\\\""));
    }
    #[cfg(not(windows))]
    {
        if value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "-_./:=+".contains(ch))
        {
            return value.to_string();
        }
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}
