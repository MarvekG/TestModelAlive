# TestModelAlive

[中文](README.md)

TestModelAlive is a Tauri desktop app for managing Codex / Claude / OpenCode / DeepSeek Harness-compatible API endpoints and checking whether saved models are alive through the local CLI tools.

The app is bilingual, with Chinese and English UI support. Chinese is the default language.

## Overall Design

1. Home page: enter the model API information, fetch the model list, and save the API endpoint with its selected models.
2. Model testing page: generate temporary config files for model availability checks, optionally replace Codex / Claude / OpenCode / DeepSeek Harness config, and test with the replaced config.

Model availability checking means verifying whether a model can be used. The app sends a configured prompt to the model and marks the test as passed when the command output contains the configured success keyword.

## Home Page Usage

1. Fill in the endpoint name, type, URL, and API key.
2. Click "Fetch Models", confirm the model list, and select the models you want to save.
3. Click "Save Endpoint", then use "Saved Endpoints" to select, load, copy, or delete endpoints.

![Home page](https://github.com/user-attachments/assets/3d0cca52-9309-4e4d-8375-2eb44c27cde1)

## Model Testing Page Usage

1. Select a saved endpoint on the home page and click "Test" to open the model testing page. The page shows the endpoint type, URL, and masked API key.
2. Confirm the models to test. You can fetch models again, save models, or quickly adjust the range with select all, select none, or invert selection.
3. Set the timeout as needed. Claude endpoints can enable "Append 1M context [1m] to model" to test long-context model names.
4. Click "Test Settings" to edit the test prompt and success keyword. The prompt must require the model to output that keyword, which the app uses to decide whether the test passed.
5. Click "Start Test" to test the current endpoint, or click "Test Current Config" to verify the existing local CLI config. During testing, you can stop the run and inspect status, elapsed time, errors, and logs in the result and log areas. After a model passes, apply the config to Codex, Claude, OpenCode, or DeepSeek Harness as needed.

Check whether models are available:

![Model availability check](https://github.com/user-attachments/assets/1a769d4a-210c-42fd-8851-869d46eaf66c)

Replace CLI config:

![Replace CLI config](https://github.com/user-attachments/assets/92beed45-5e0f-4279-b5dc-fe2794b87370)

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
   - `opencode` for OpenCode endpoints.
   - `dsh` for DeepSeek Harness endpoints, installable with `npm install -g @deepseek-ai/dsh`.

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

- `settings.json`: saved endpoints, model lists, test settings, and CLI config restore baselines.
- `cli-config-apply-history.json`: CLI config apply history.
- `claude-settings.json`: temporary Claude CLI settings created during tests, overwritten on each test, and kept for troubleshooting.
- `codex-home/`: isolated Codex home used during tests.
- `opencode-home/`: isolated OpenCode home used during tests.
- `dsh-home/`: isolated `DSH_HOME` used during DeepSeek Harness tests.
- `cli-config-backups/`: backups created before applying or restoring real CLI config files.

## Testing Models

Model tests run through the local CLI tools:

- Codex tests run with an isolated `CODEX_HOME` under `~/.TestModelAlive/codex-home`.
- Claude tests run with `~/.TestModelAlive/claude-settings.json` as the settings file.
- OpenCode tests run with an isolated home under `~/.TestModelAlive/opencode-home`.
- DeepSeek Harness tests run with an isolated `DSH_HOME` under `~/.TestModelAlive/dsh-home` through `dsh --profile headless`; the API key is injected only into the child process environment.

The test dialog shows CLI output in real time. The backend no longer mirrors test logs to the terminal.

The success condition is configurable:

- Set a test prompt.
- Set a success keyword.
- The prompt must explicitly include the success keyword and require the model to output it.
- A model is marked available when the command output contains the success keyword.

## DeepSeek Harness Configuration

- For a DeepSeek Harness endpoint, select one or more models, click "Apply to DeepSeek Harness", and choose one of them as the default model.
- The app merges DSH's `settings.yaml` and `.credentials.yaml` under `DSH_HOME` when set, or under the default `~/.dsh/` directory.
- The OpenAI-compatible provider is named `tma-<endpoint name>`, and the selected model becomes DSH's `agent-default-model`.
- The app reuses OpenCode's maintained model context, maximum-output, and reasoning-level values, converting them to DSH YAML fields; unknown models retain DSH defaults.
- Model metadata is embedded in `src-tauri/src/model_metadata.json`, not written to `~/.TestModelAlive/settings.json`; startup automatically removes the legacy `opencode_model_variants` field.
- DeepSeek Harness is currently in developer preview and may introduce breaking configuration changes. Run "Test Current Config" after upgrading DSH.

## Platform Notes

- Windows support includes `.cmd` / `.bat` CLI shims and process-tree termination through `taskkill /T /F`.
- macOS and Linux include common CLI search paths such as `/usr/local/bin`, `/opt/homebrew/bin`, and `~/.local/bin`.
- Cross-compiling Tauri apps can require platform-specific SDKs and resource compilers beyond Rust targets.

## Friend Links

- [LINUX DO](https://linux.do/)

## Security

API keys are stored in plaintext inside `~/.TestModelAlive/settings.json`; applying an endpoint to a local CLI also writes them into that CLI's config files.

Do not commit or share runtime data files. Relevant local data files are ignored by `.gitignore`.
