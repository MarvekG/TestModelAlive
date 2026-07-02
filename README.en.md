# TestModelAlive

[中文](README.md)

TestModelAlive is a Tauri desktop app for managing Codex / Claude-compatible API endpoints and checking whether saved models are alive through the local CLI tools.

The app is bilingual, with Chinese and English UI support. Chinese is the default language.

## Features

- Add and save `codex` / `claude` API endpoints.
- Fetch model lists from endpoint `/models` APIs.
- Select models and save them with the endpoint.
- Manage saved endpoints with single or batch deletion.
- Copy endpoint URL / API key.
- Test saved models through local `codex` or `claude` CLI commands.
- Stream test output into the test dialog in real time.
- Configure the test prompt and success keyword.
- Persist endpoint data and test settings in the user's home directory.
- Switch UI language between Chinese and English.

## Tech Stack

- Frontend: TypeScript + Vite, source in `frontend/`.
- Desktop/runtime: Tauri 2 + Rust, source in `src-tauri/`.
- Legacy Python/PyQt files are kept under `lagacy/`.

## Requirements

- Node.js and npm.
- Rust stable toolchain.
- Tauri system dependencies for your platform.
- Local CLI tools depending on what you test:
  - `codex` for Codex endpoints.
  - `claude` for Claude endpoints.

The app searches for CLI executables in `PATH` and common install locations, including npm global paths on Windows and Homebrew paths on macOS.

## Development

Install dependencies:

```bash
npm install
```

Start Tauri development mode:

```bash
npm run tauri dev
```

Build the frontend only:

```bash
npm run build
```

Build the desktop app:

```bash
npm run tauri build
```

## Data Location

Runtime data is stored under the user's home directory:

```text
~/.TestModelAlive/
```

On Windows this resolves to:

```text
%USERPROFILE%\.TestModelAlive\
```

Files stored there include:

- `tsa_endpoints.json`: saved endpoints and model lists.
- `test_settings.json`: test prompt and success keyword.
- `claude-settings.json`: temporary Claude CLI settings used during tests.
- `codex-home/`: isolated Codex home used during tests.

If old `tsa_endpoints.json` or `test_settings.json` files exist in the current working directory, the app copies them into `~/.TestModelAlive/` on first use.

## Testing Models

Model tests run through the local CLI tools:

- Codex tests run with an isolated `CODEX_HOME` under `~/.TestModelAlive/codex-home`.
- Claude tests run with `~/.TestModelAlive/claude-settings.json` as the settings file.

The test dialog shows CLI output in real time. The backend no longer mirrors test logs to the terminal.

The success condition is configurable:

- Set a test prompt.
- Set a success keyword.
- The prompt must explicitly include the success keyword and require the model to output it.
- A model is marked available when the command output contains the success keyword.

## Cross-Platform Notes

- Linux is the primary development environment in this repository.
- Windows support includes `.cmd` / `.bat` CLI shims and process-tree termination through `taskkill /T /F`.
- macOS and Linux include common CLI search paths such as `/usr/local/bin`, `/opt/homebrew/bin`, and `~/.local/bin`.
- Cross-compiling Tauri apps can require platform-specific SDKs and resource compilers beyond Rust targets.

## Security

API keys are stored in plaintext inside `~/.TestModelAlive/tsa_endpoints.json`.

Do not commit or share runtime data files. Relevant local data files are ignored by `.gitignore`.
