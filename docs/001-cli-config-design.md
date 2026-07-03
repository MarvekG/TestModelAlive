# 测试模型页导出 CLI API 配置设计

## 背景

当前应用支持保存 `codex` / `claude` / `opencode` 端点，并在“测试模型”弹窗内用所选端点临时执行 CLI 测试：

- Codex 测试写入隔离目录 `~/.TestModelAlive/codex-home/`，通过 `CODEX_HOME` 指向临时配置。
- Claude 测试写入 `~/.TestModelAlive/claude-settings.json`，通过 `claude --settings` 指定临时 settings 文件。
- OpenCode 测试写入隔离目录 `~/.TestModelAlive/opencode-home/`，通过临时 `HOME` / `USERPROFILE` 指向临时配置。

新需求是在“测试设置”按钮后面增加按当前端点类型展示的应用按钮，并在主界面增加一个统一的“还原配置”按钮：

- `codex` 类型模型测试页：显示“应用到 Codex”。
- `claude` 类型模型测试页：显示“应用到 Claude”。
- `opencode` 类型模型测试页：显示“应用到 OpenCode”和“从 OpenCode 移除”。
- 主界面：显示“还原配置”，点击后弹窗勾选需要还原的配置，勾选哪些还原哪些。
- 测试模型页常驻“测试当前配置”按钮；配置应用完成结果弹窗中也提供“用当前配置测试”入口，直接使用本机真实 CLI 配置测试 Codex、Claude、OpenCode，不再使用临时配置。

写入目标仍受端点类型约束：

- `codex` 类型凭证：可应用到 Codex。
- `claude` 类型凭证：可应用到 Claude。
- `opencode` 类型凭证：可应用到 OpenCode。
- Codex 和 Claude 都写入对应 CLI 配置文件，写入前备份。Codex 的 `auth.json` 预览当前只生成 `OPENAI_API_KEY` 基础结构；`config.toml` 在原配置基础上合并。
- OpenCode 是按端点名称写入 provider；如果同名 provider 已存在，必须先提示用户确认覆盖。

应用自身配置统一集中到 `~/.TestModelAlive/settings.json`，包括端点数据、测试提示词设置、CLI 配置备份索引和原始基线信息。真实 CLI 配置备份文件统一保存到 `~/.TestModelAlive/` 下的子目录，不散落在 CLI 原配置目录旁边。

该功能是持久修改用户本机 CLI 配置，不应复用测试时的临时恢复机制。

## 目标

- 在测试模型弹窗的“测试设置”按钮后增加按当前端点类型展示的应用按钮。
- 在主界面增加一个“还原配置”按钮，通过弹窗勾选需要还原的配置。
- 使用当前测试弹窗中的端点作为配置来源。
- 根据端点类型展示可用应用按钮：`codex` 展示 Codex，`claude` 展示 Claude，`opencode` 展示 OpenCode。
- 写入真实 CLI 配置前显示编辑确认弹窗，展示即将写入的新配置内容，允许用户修改，确认后才执行备份和替换。
- 对目标配置文件做 `baseline` 或 `apply` 备份后写入。Codex `config.toml`、Claude JSON、OpenCode JSON/JSONC 会在可解析时合并；Codex `auth.json` 当前预览为基础 JSON。
- `opencode` 使用端点名称作为 provider key 写入入口；同名 provider 只有在用户确认后才允许覆盖。
- 每个目标文件首次被本应用修改前创建原始配置基线，记录原文件路径、备份路径、原文件是否存在等信息。
- 提供“一键还原”能力，可以基于原始配置基线恢复到本应用修改前的最原始配置状态；后续在应用内无论修改多少次，都不覆盖该基线。
- 应用配置集中写入 `~/.TestModelAlive/settings.json`，包括原有提示词设置和 CLI 备份路径。
- 备份文件写入 `~/.TestModelAlive/cli-config-backups/<target>/` 子目录。
- 写入完成后展示成功结果和备份路径。
- 测试模型页和应用成功结果弹窗都提供真实 CLI 配置测试入口。
- 失败时保留已生成的备份，不自动删除，便于用户恢复。

## 非目标

- 不实现导入已有 provider 的 UI；OpenCode 已实现按当前端点 URL 和 API Key 匹配 provider 的“从 OpenCode 移除”流程，移除前同样展示可编辑预览。
- 不实现任意历史备份浏览、差异对比的 UI；初版通过勾选列表选择要还原的配置，并还原到本应用首次修改该配置前的原始状态。
- 不自动探测或安装 `codex` / `claude` / `opencode` CLI。
- 新增其他端点类型时应按同一套 fetch/test/apply 分支接入，不复用既有类型。
- 不写入模型列表。按钮只替换或新增 API URL 和 Key 相关配置。
- 不加密存储 API Key。目标 CLI 配置本身通常也是明文或可读配置。
- 不把 `claude` 类型凭证写入 Codex 或 OpenCode。
- 不把 `codex` 类型凭证写入 Claude。

## UI 设计

位置：`frontend/ui/renderApp.ts` 的测试弹窗控制栏。当前按钮顺序为：

```text
超时时间 / 开始测试 / 测试当前配置 / 停止 / 测试设置 / 状态
```

调整为：

```text
codex 端点：超时时间 / 开始测试 / 测试当前配置 / 停止 / 测试设置 / 应用到 Codex / 状态
claude 端点：超时时间 / 1M 选项 / 开始测试 / 测试当前配置 / 停止 / 测试设置 / 应用到 Claude / 状态
opencode 端点：超时时间 / 开始测试 / 测试当前配置 / 停止 / 测试设置 / 应用到 OpenCode / 从 OpenCode 移除 / 状态
```

主界面新增：

```text
还原配置
```

按钮行为：

- 应用按钮文案：中文为“应用到 Codex”、“应用到 Claude”、“应用到 OpenCode”、“从 OpenCode 移除”；英文为“Apply to Codex”、“Apply to Claude”、“Apply to OpenCode”、“Remove from OpenCode”。
- 主界面还原按钮文案：中文为“还原配置”，英文为“Restore Config”。
- 未打开测试弹窗或没有 `testEndpoint` 时，不展示或禁用应用按钮。
- 当前端点为 `codex` 时，只展示“应用到 Codex”。
- 当前端点为 `claude` 时，只展示“应用到 Claude”。
- 当前端点为 `opencode` 时，只展示“应用到 OpenCode”和“从 OpenCode 移除”。
- 点击“应用到 Codex”或“应用到 Claude”前，必须且只能勾选一个模型；应用到 Claude 时会询问是否在写入配置的模型名后追加 `[1m]` 后缀。
- 点击“应用到 OpenCode”前，必须至少勾选一个模型，允许多选。
- 点击应用按钮后先生成目标 CLI 的新配置内容，并弹出编辑确认框。
- 编辑确认框展示目标标题、当前端点类型、选中模型、目标文件 ID、目标路径，以及即将写入的完整配置内容；API Key 可能出现在可编辑配置内容中，确认提示会说明将明文写入本机 CLI 配置。
- 用户可在编辑框中修改即将写入的配置内容；点击确认后，前端把修改后的内容传给后端。
- 用户确认后调用后端 `apply_cli_config(app, endpoint, target, edited_config)`。
- 成功后 toast 显示“已应用到 <target>”，并在结果弹窗中展示写入文件、操作备份文件、原始基线备份文件。
- 应用成功结果弹窗中展示“用当前配置测试”按钮，英文为“Test Current Config”；测试模型页也有同名按钮，可不经过应用结果弹窗直接验证本机真实配置。
- 点击“用当前配置测试”后调用后端 `test_cli_with_real_config(app, state, request, on_event)`，请求中包含 `target`、`endpoint_name`、`models`、`prompt`、`success_keyword`、`timeout`。
- 真实配置测试不写入临时配置，不设置 `CODEX_HOME`，Claude 不传 `--settings`，OpenCode 使用默认配置目录。
- 真实配置测试结果复用现有测试结果展示样式，日志会输出 `starting real CLI config test` 和目标类型，避免与临时配置测试混淆。
- 失败后 toast 显示错误，并保留错误详情。
- 点击主界面“还原配置”后弹出还原选择框。
- 还原选择框通过 `load_cli_config_baseline_items` 从 `settings.json.cli_config.baseline_items` 读取可还原项，当前实现以列表展示，默认全选。
- 用户勾选哪些配置就还原哪些配置；如果确认时没有勾选，前端提示错误。
- 用户确认后调用后端 `restore_original_cli_config(app, selected_items)`。
- 还原成功后 toast 显示“已还原配置”，并展示每个目标文件的还原结果。

不提供 6 个固定按钮。应用按钮只在对应类型的模型测试页面出现；还原入口统一为主界面的一个“还原配置”按钮，通过弹窗勾选控制还原范围。

## 后端接口

新增 Tauri command：

```rust
#[tauri::command]
fn build_cli_config_preview(app: tauri::AppHandle, endpoint: SavedEndpoint, target: CliConfigTargetKind, selected_models: Vec<String>, default_model: Option<String>) -> Result<CliConfigPreview, String>

#[tauri::command]
fn build_remove_opencode_config_preview(app: tauri::AppHandle, endpoint: SavedEndpoint) -> Result<CliConfigPreview, String>

#[tauri::command]
fn apply_cli_config(app: tauri::AppHandle, endpoint: SavedEndpoint, target: CliConfigTargetKind, edited_config: EditedCliConfig) -> Result<ApplyCliConfigResult, String>

#[tauri::command]
fn restore_original_cli_config(app: tauri::AppHandle, selected_items: Vec<RestoreSelection>) -> Result<RestoreCliConfigResult, String>

#[tauri::command]
fn load_cli_config_baseline_items(app: tauri::AppHandle) -> Result<Vec<CliConfigBaselineView>, String>

#[tauri::command]
fn test_cli_with_real_config(app: tauri::AppHandle, state: tauri::State<AppState>, request: RealCliTestRequest, on_event: Channel<TestMessage>) -> Result<(), String>
```

返回结构：

```rust
#[derive(Debug, Serialize)]
struct CliConfigPreview {
    endpoint_type: String,
    target: String,
    files: Vec<CliConfigPreviewFile>,
    warnings: Vec<CliConfigPreviewWarning>,
}

enum CliConfigPreviewWarning {
    OpenCodeProviderOverwrite { provider: String },
}

#[derive(Debug, Serialize)]
struct CliConfigPreviewFile {
    file_id: String,
    path: String,
    language: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct EditedCliConfig {
    files: Vec<EditedCliConfigFile>,
    selected_models: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EditedCliConfigFile {
    file_id: String,
    path: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ApplyCliConfigResult {
    baseline_id: String,
    baseline_path: String,
    endpoint_type: String,
    target: String,
    results: Vec<CliConfigWriteResult>,
}

#[derive(Debug, Serialize)]
struct CliConfigWriteResult {
    target: String,
    path: String,
    backup_paths: Vec<String>,
    action: String,
    ok: bool,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct RestoreCliConfigResult {
    baseline_id: String,
    results: Vec<CliConfigRestoreResult>,
}

#[derive(Debug, Serialize)]
struct CliConfigRestoreResult {
    target: String,
    path: String,
    action: String,
    ok: bool,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum CliConfigTargetKind {
    Codex,
    Claude,
    Opencode,
}

#[derive(Debug, Deserialize)]
struct RestoreSelection {
    target: CliConfigTargetKind,
    file_id: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct RealCliTestRequest {
    target: CliConfigTargetKind,
    endpoint_name: Option<String>,
    models: Vec<String>,
    prompt: String,
    success_keyword: String,
    timeout: u64,
}
```

说明：

- `CliConfigTargetKind` 前后端传输值为 `codex`、`claude`、`opencode`。
- `build_cli_config_preview` 的 `default_model` 仅用于 OpenCode；传入后会写入顶层 `model` 和 `small_model`，格式为 `<provider>/<model>`。
- `build_remove_opencode_config_preview` 只生成移除匹配 OpenCode provider 后的预览，不直接写入；确认后仍复用 `apply_cli_config` 写入。
- `load_cli_config_baseline_items` 返回给还原弹窗展示的精简基线项，不包含 `backup_path`。
- `test_cli_with_real_config` 通过 `Channel<TestMessage>` 流式返回日志和结果，不同步返回 `Vec<TestResult>`。
- `RealCliTestRequest.endpoint_name` 用于 OpenCode 真实配置测试，命令会拼出 `<endpoint_name>/<model>`。

`action` 建议值：

- `replaced`
- `added`
- `created`
- `updated`
- `skipped`
- `restored`
- `deleted`

命令按 `target` 只执行指定工具：

- `target == Codex`：只写入 Codex，要求 `endpoint.type == "codex"`。
- `target == Claude`：只写入 Claude，要求 `endpoint.type == "claude"`。
- `target == Opencode`：只写入 OpenCode，要求 `endpoint.type == "opencode"`。

应用流程：

1. 前端点击“应用到 X”。
2. 前端收集当前测试弹窗选中的模型。Codex/Claude 必须且只能选择 1 个模型；OpenCode 可以选择多个模型。
3. 前端调用 `build_cli_config_preview` 生成默认配置内容；OpenCode 可先询问是否设置默认模型，并把选中的默认模型作为 `default_model` 传给后端。
4. 前端弹出编辑确认框，用户检查并可修改每个目标文件内容。
5. 用户确认后，前端调用 `apply_cli_config`，传入 `edited_config`，其中包含编辑后的文件内容和本次选中的模型列表。
6. 后端对 `edited_config` 做基础校验，先创建原始基线和操作备份，再把用户确认后的内容写入真实 CLI 配置。

`build_cli_config_preview` 不写任何真实 CLI 配置，也不创建备份。

初版不建议部分成功后自动回滚，因为：

- 配置写入是跨多个真实用户文件的持久操作。
- 自动回滚可能覆盖用户在写入期间由其他进程产生的新变化。
- 保留备份路径并明确报告部分成功更安全。

如果需要更强一致性，可在后续版本加入“预写临时文件 + 全部校验通过后 rename”的事务化流程。

## 路径解析

新增函数 `user_config_dir()`，基于当前已有 `user_home_dir()` 派生路径。

应用配置目录：

- `~/.TestModelAlive/`

应用统一配置文件：

- `~/.TestModelAlive/settings.json`

CLI 配置备份目录：

- Codex：`~/.TestModelAlive/cli-config-backups/codex/`
- Claude：`~/.TestModelAlive/cli-config-backups/claude/`
- OpenCode：`~/.TestModelAlive/cli-config-backups/opencode/`

建议目标路径：

- Codex：`~/.codex/config.toml` 和 `~/.codex/auth.json`
- Claude：`~/.claude/settings.json` 和 `~/.claude.json`
- OpenCode：由 `paths.rs` 按平台解析 OpenCode 配置路径，Linux/XDG 示例为 `~/.config/opencode/opencode.json`

写入目标使用真实 CLI 配置路径，因为该功能目标是修改真实 CLI 配置。备份文件和本应用的索引信息使用 `~/.TestModelAlive/`，避免在用户 CLI 配置目录下散落本应用生成的备份文件。

## 跨平台兼容要求

目标平台包括 Windows、macOS、Linux。实现时不能依赖 shell 语法或某个平台的路径分隔符。

### 路径规则

- 不在代码里拼接 `/` 或 `\\`，统一使用 `PathBuf::join`。
- `~` 只在文档中作为展示写法；代码必须通过 home 目录函数解析成绝对路径。
- 应用配置目录固定为 home 下的 `.TestModelAlive`：
  - Windows：`%USERPROFILE%\.TestModelAlive`
  - macOS/Linux：`$HOME/.TestModelAlive`
- Codex 真实配置路径固定为 home 下：
  - `~/.codex/config.toml`
  - `~/.codex/auth.json`
- Claude 真实配置路径固定为 home 下：
  - `~/.claude/settings.json`
  - `~/.claude.json`
- OpenCode 配置目录优先使用系统 config dir：
  - Windows：`%USERPROFILE%\.config\opencode\opencode.json`。
  - macOS：`$HOME/Library/Application Support/opencode/opencode.json` 或 CLI 实际文档路径。
  - Linux：`${XDG_CONFIG_HOME:-$HOME/.config}/opencode/opencode.json`。
- 当前文档中的 `~/.config/opencode/opencode.json` 只能作为 Linux/XDG 示例；实现已在 `paths.rs` 中集中封装 OpenCode 路径解析，避免散落硬编码。

### 文件写入与替换

- 写配置文件时使用“同目录临时文件 + fsync/flush + replace”策略。
- Windows 上不能直接依赖 `rename(tmp, target)` 覆盖已存在文件；需要先删除目标再 rename，或使用支持 replace 语义的 crate/API。
- 删除再 rename 前必须已经完成原始基线备份和本次操作备份。
- 临时文件必须和目标文件在同一目录，避免跨文件系统 rename 失败。
- 备份文件名只使用 ASCII、数字、点、短横线和下划线，避免 Windows 保留字符：`<>:"/\\|?*`。

### 命令执行

- 不通过 shell 执行命令，不拼接整条命令字符串。
- 使用 `std::process::Command` 并逐个传参，避免 Windows CMD、PowerShell、bash/zsh 的转义差异。
- 文档中的命令行示例只用于说明，实际实现必须使用参数数组。
- Prompt 中可以包含空格、引号、换行；实现必须作为单个参数传给 CLI。
- CLI 可执行文件名按 PATH 查找：`codex`、`claude`、`opencode`。Windows 下不要手写 `.exe`，交给系统 PATH 解析。

### 环境变量

- 临时测试和真实配置测试必须明确区分环境变量。
- 真实配置测试不得设置 `CODEX_HOME` 或临时 OpenCode 配置目录。
- 如需删除某个环境变量，使用 `Command::env_remove`，不要设置为空字符串来模拟删除。
- 继承用户环境时要注意 Windows 环境变量大小写不敏感。

### 换行与编码

- 所有 JSON/TOML 配置按 UTF-8 写入。
- 写入时统一使用 `\n` 换行即可，Windows CLI 通常可接受；不要根据平台混用导致 diff 和预览不稳定。

### 前端展示

- UI 中展示路径时使用后端返回的 display string。
- 不在前端自行拼接 home 路径或配置路径。
- 复制命令或路径时只复制实际路径，不复制文档中的 `~` 示例路径。

## 统一设置文件

应用自己的配置集中保存到 `~/.TestModelAlive/settings.json`。此前分散保存的提示词设置、端点设置，以及新增的 CLI 配置备份路径和原始基线，都应逐步迁移到该文件。

`settings.json` 使用普通 JSON，不使用 JSONC，便于 Rust 后端用 `serde_json` 直接读写。写入时使用 pretty JSON，并采用“写入临时文件 + 平台兼容的 replace”方式降低配置损坏风险。

### 顶层结构

建议结构：

以下路径仅为 Windows 示例。实际写入必须使用 `paths.rs` 返回的平台绝对路径。

```json
{
  "version": 1,
  "endpoints": [
    {
      "id": "codex-20260703153045-001",
      "name": "Kimi Codex",
      "type": "codex",
      "base_url": "https://api.example.com/v1",
      "api_key": "sk-xxx",
      "models": ["gpt-4.1"]
    }
  ],
  "test_settings": {
    "prompt": "You must output exactly OKK and nothing else. Do not explain. Do not add punctuation.",
    "success_keyword": "OKK"
  },
  "cli_config": {
    "baseline_id": "baseline-1780000000000000000",
    "backup_root": "C:\\Users\\me\\.TestModelAlive\\cli-config-backups",
    "apply_history_limit": 20,
    "baseline_items": [
      {
        "target": "codex",
        "file_id": "codex-config",
        "path": "C:\\Users\\me\\.codex\\config.toml",
        "existed_before": true,
        "backup_path": "C:\\Users\\me\\.TestModelAlive\\cli-config-backups\\codex\\config.toml.baseline.1780000000000000000.0.bak",
        "created_at": "1780000000000000000"
      }
    ]
  }
}
```

`~/.TestModelAlive/cli-config-apply-history.json` 保存应用历史：

```json
{
  "limit": 20,
  "items": [
    {
      "apply_id": "apply-1780000000000000000",
      "target": "opencode",
      "endpoint_id": "codex-20260703153045-001",
      "created_at": "1780000000000000000",
      "backup_paths": [
        "C:\\Users\\me\\.TestModelAlive\\cli-config-backups\\opencode\\opencode.json.apply.1780000000000000000.0.bak"
      ],
      "files": [
        {
          "file_id": "opencode-config",
          "path": "C:\\Users\\me\\.config\\opencode\\opencode.json",
          "action": "updated"
        }
      ]
    }
  ]
}
```

说明：

- `settings.json` 是应用内配置和原始基线索引的入口。
- `cli-config-apply-history.json` 单独保存每次应用配置的操作历史。
- 真实备份文件仍以独立文件保存在 `cli-config-backups/<target>/` 下，`settings.json` 只保存路径和元数据。
- 迁移旧提示词设置时，如果旧配置存在而 `settings.json` 缺失，应在启动时读取旧值并写入 `settings.json`。
- 后续读写提示词设置和 CLI 原始基线索引时，只读写 `settings.json`；应用历史读写 `cli-config-apply-history.json`。

### 字段定义

`version`：配置格式版本。初版固定为 `1`。后续格式变更时增加版本号，并在启动时做迁移。

`endpoints`：用户保存的端点列表，替代旧的分散端点配置。

- `id`：端点稳定 ID。建议格式为 `<type>-<timestamp>-<seq>`。
- `name`：用户可读名称。
- `type`：端点类型，只允许 `codex`、`claude` 或 `opencode`。
- `base_url`：端点 URL，保存时去掉尾部 `/`。
- `api_key`：API Key，按现有设计明文保存。
- `models`：用户在测试页选择或保存的模型 ID 列表。
- 当前 `SavedEndpoint` 不保存创建时间或更新时间字段。

`test_settings`：测试模型页的提示词和成功判断关键词。

- `prompt`：测试提示词。当前默认值为 `You must output exactly OKK and nothing else. Do not explain. Do not add punctuation.`。
- `success_keyword`：成功关键词。当前默认值为 `OKK`，保存时要求提示词包含该关键词。
- 超时时间不保存在 `settings.json.test_settings` 中，当前由测试弹窗输入框提供默认值。

`cli_config`：真实 CLI 配置的备份索引和应用历史。

- `baseline_id`：原始基线 ID。首次创建 `cli_config` 时生成，后续不变。
- `backup_root`：备份根目录，默认 `~/.TestModelAlive/cli-config-backups` 展开后的绝对路径。
- `apply_history_limit`：独立应用历史文件最多保留的记录数，默认 `20`。
- `baseline_items`：本应用首次修改某个真实 CLI 配置文件前记录的原始状态。
- `apply_history` 不再保存在 `settings.json`；历史记录保存到 `cli-config-apply-history.json`。

`baseline_items[]` 字段：

- `target`：目标工具，取值为 `codex`、`claude`、`opencode`。
- `file_id`：稳定文件 ID，取值见下方“文件 ID”。
- `path`：真实 CLI 配置文件绝对路径。
- `existed_before`：本应用首次修改前该文件是否存在。
- `backup_path`：本应用首次修改前的备份文件路径；`existed_before == false` 时为 `null`。
- `created_at`：该基线项创建时间，当前为 Unix timestamp nanos 字符串。

`cli-config-apply-history.json.items[]` 字段：

- `apply_id`：本次应用操作 ID，每次应用生成一次。
- `target`：本次应用目标工具。
- `endpoint_id`：本次应用使用的端点 ID，当前为字符串。
- `created_at`：本次应用时间，当前为 Unix timestamp nanos 字符串。
- `backup_paths`：本次操作产生的普通备份路径列表。
- `files`：本次操作涉及的目标文件列表。

`cli-config-apply-history.json.items[].files[]` 字段：

- `file_id`：稳定文件 ID。
- `path`：真实 CLI 配置文件绝对路径。
- `action`：本次操作动作，例如 `replaced`、`added`、`created`、`updated`、`skipped`。

### 文件 ID

文件 ID 固定为以下值，避免 UI 和恢复逻辑依赖路径字符串判断：

- `codex-auth`：`~/.codex/auth.json`
- `codex-config`：`~/.codex/config.toml`
- `claude-settings`：`~/.claude/settings.json`
- `claude-state`：`~/.claude.json`
- `opencode-config`：由 `paths.rs` 按平台解析的 OpenCode 配置文件

### 默认值

如果 `settings.json` 不存在，启动时创建：

```json
{
  "version": 1,
  "endpoints": [],
  "test_settings": {
    "prompt": "You must output exactly OKK and nothing else. Do not explain. Do not add punctuation.",
    "success_keyword": "OKK"
  },
  "cli_config": {
    "baseline_id": "baseline-1780000000000000000",
    "backup_root": "C:\\Users\\me\\.TestModelAlive\\cli-config-backups",
    "apply_history_limit": 20,
    "baseline_items": []
  }
}
```

说明：

- `baseline_id` 在默认配置创建或规范化时生成；如果读取到空值，会重新生成。
- `backup_root` 在默认配置创建或规范化时写入展开后的绝对路径；如果读取到空值，会重新填充。

### 读写规则

- 启动时读取 `settings.json`，不存在则创建默认配置。
- 读取失败或 JSON 解析失败时，不自动覆盖原文件；提示用户配置损坏，并建议手动处理或备份后重建。
- 写入前先确保 `~/.TestModelAlive/` 存在。
- 写入时先写 `settings.json.tmp`，成功后用平台兼容的 replace 覆盖 `settings.json`。不要直接假设 `std::fs::rename(tmp, target)` 可以覆盖已有文件，因为 Windows 上 rename 到已存在目标通常会失败。
- 更新 `baseline_items` 时只追加缺失项，不覆盖已有 `target + file_id + path` 项。
- 更新 `cli-config-apply-history.json` 时追加新记录，并按 `settings.json.cli_config.apply_history_limit` 裁剪。

## 备份与原始基线策略

新增持久备份函数和原始基线记录，不使用当前 `RestorableFile` 的 `Drop` 恢复逻辑：

```rust
fn backup_if_exists(path: &Path) -> Result<Option<PathBuf>, String>
fn ensure_original_baseline(targets: &[CliConfigTarget]) -> Result<CliConfigBaseline, String>
fn save_original_baseline(baseline: &CliConfigBaseline) -> Result<PathBuf, String>
```

行为：

- 文件存在时复制到 `~/.TestModelAlive/cli-config-backups/<target>/`。
- 备份名使用当前项目已有 `next_backup_path` 风格，例如：
  - `config.toml.baseline.1780000000000000000.0.bak`
  - `auth.json.apply.1780000000000000000.0.bak`
  - `settings.json.pre-restore.1780000000000000000.0.bak`
  - `opencode.json.apply.1780000000000000000.0.bak`
- 文件不存在时返回 `None`。
- 写入前确保父目录存在。
- 所有会被修改的目标文件必须先完成备份和原始基线记录，再执行任何配置写入。
- 如果原始基线写入失败，停止本次应用，不修改真实 CLI 配置。
- 原始基线按目标文件记录，只在该目标第一次被本应用修改前创建；后续应用再次修改同一目标时，不覆盖已有基线。

备份文件不放在真实 CLI 配置目录旁边，而是放在应用配置目录下：

- Codex 备份：`~/.TestModelAlive/cli-config-backups/codex/`
- Claude 备份：`~/.TestModelAlive/cli-config-backups/claude/`
- OpenCode 备份：`~/.TestModelAlive/cli-config-backups/opencode/`

Codex 有两个目标文件，两个文件都需要备份。返回结果使用 `backup_paths: Vec<String>`，避免丢失 `auth.json` 的备份信息。

OpenCode 虽然是新增接入点，不替换整个配置语义，但仍会修改 `opencode.json`。如果文件已存在，也建议先备份，再合并写入，便于用户手动恢复。

### 原始配置基线

原始配置基线用于“一键还原到本应用修改前的最原始配置”，必须保存到 `~/.TestModelAlive/settings.json` 的 `cli_config.baseline_items`，不能只依赖用户记住 `.bak` 路径。

`settings.json` 中的 `cli_config.baseline_items` 是一键还原的唯一依据。它按目标文件保存本应用首次修改前的状态，后续应用不能覆盖已有 item。每次应用仍可写入 `cli-config-apply-history.json`，用于展示本次备份路径和排查问题，但不能作为“一键还原到原始配置”的依据。

`baseline_id` 在 `settings.json.cli_config` 首次创建或规范化时生成，当前格式为 `baseline-<unix_timestamp_nanos>`。`apply_id` 每次应用生成一次，当前格式为 `apply-<unix_timestamp_nanos>`。

基线 item 建议结构：

以下路径仅为 Windows 示例。实际写入必须使用 `paths.rs` 返回的平台绝对路径。

```json
{
  "target": "codex",
  "file_id": "codex-auth",
  "path": "C:\\Users\\me\\.codex\\auth.json",
  "existed_before": true,
  "backup_path": "C:\\Users\\me\\.TestModelAlive\\cli-config-backups\\codex\\auth.json.20260703153045.bak",
  "created_at": "1780000000000000000"
}
```

字段语义：

- `path`：真实 CLI 配置路径。
- `existed_before`：本应用首次修改该目标前，目标文件是否存在。
- `backup_path`：本应用首次修改该目标前，目标文件存在时的备份路径；不存在时为 `null`。
- `target`：目标工具，取值为 `codex`、`claude`、`opencode`。
- `file_id`：稳定文件标识，用于展示和恢复结果，例如 `codex-auth`、`codex-config`、`claude-settings`、`claude-state`、`opencode-config`。
- `created_at`：该基线项创建时间，当前为 Unix timestamp nanos 字符串。

写入顺序必须是：

1. 解析本次会修改的目标文件列表。
2. 读取或创建 `~/.TestModelAlive/settings.json`。
3. 对每个目标文件检查 `cli_config.baseline_items` 中是否已有相同 `target + file_id + path` 的 item。
4. 如果没有 item，说明这是本应用第一次修改该目标文件：先备份当前目标文件到 `cli-config-backups/<target>/`，再将 `target`、`file_id`、`path`、`existed_before`、`backup_path`、`created_at` 写入 `settings.json`。
5. 如果已有 item，说明原始配置已记录，不能覆盖该 item；本次应用只创建普通操作备份并写入 `cli-config-apply-history.json`。
6. 保存 `settings.json`。
7. 执行 Codex/Claude/OpenCode 配置修改。

这样即使后续在应用内多次修改，也能通过 `settings.json` 找到本应用首次修改前的最原始状态并还原。

### 还原配置

主界面新增“还原配置”入口。按钮行为：

- 读取 `~/.TestModelAlive/settings.json`。
- 从 `cli_config.baseline_items` 生成可勾选列表。
- 列表按目标工具分组：Codex、Claude、OpenCode。
- Codex 分组包含 `codex-auth` 和 `codex-config`。
- Claude 分组包含 `claude-settings` 和 `claude-state`。
- OpenCode 分组包含 `opencode-config`。
- 用户勾选哪些配置，就把哪些 item 作为 `selected_items` 传给 `restore_original_cli_config`。
- 如果 `existed_before == true`，用 `backup_path` 覆盖 `path`。
- 如果 `existed_before == false`，说明该文件是本应用首次修改时新创建的，还原时删除 `path`。
- 还原前也应把当前 `path` 备份为 `*.pre-restore.<timestamp>.bak`，避免用户在应用后手动修改的内容被永久覆盖。
- 如果某个 `backup_path` 不存在，返回该项失败，不继续覆盖该文件。
- 单项失败不阻断其他项还原，最终展示每个目标的恢复结果。

还原成功后不删除 `settings.json` 中的基线 item，允许用户重复还原到同一个原始状态。重复还原时，如果 `existed_before == false` 且 `path` 已不存在，应返回 `skipped` 并视为成功。

## CLI 写入通用规则

通用规则：

- 后端生成预览时尽量读取原配置并做结构化合并。
- 能解析的配置只修改目标字段，保留未知字段。
- 不能解析的配置不自动覆盖，返回解析错误，让用户先修复或手动处理。
- 用户在编辑确认框中确认后的 `edited_config` 是最终写入内容。
- 后端仍必须校验 `edited_config.files[*].path` 是允许的目标路径，避免前端传入任意路径。

## Codex 写入方案

仅当当前端点 `type` 为 `codex` 时执行。

目标文件：

- `~/.codex/auth.json`
- `~/.codex/config.toml`

写入策略：`config.toml` 在原有配置基础上修改必要字段；`auth.json` 当前生成基础 JSON。用户可编辑预览内容；`apply_cli_config` 写入用户确认后的内容。

### Codex auth.json

生成预览时：

- 当前实现生成基础 JSON，只包含并设置 `OPENAI_API_KEY` 为当前端点的 `api_key`。
- 当前实现不会读取并保留 `auth.json` 中其他未知字段。

如果原文件不存在：创建基础 JSON。

如果原文件存在但不是合法 JSON：当前实现不会解析旧 `auth.json`，仍会生成基础 JSON 预览；用户需要在编辑确认框中自行补回要保留的字段。

`auth.json`：

```json
{
  "OPENAI_API_KEY": "sk-xxx"
}
```

注意：如果用户需要保留 `auth.json` 中其他字段，应在预览编辑框中手动补回。

### Codex config.toml

如果原文件存在且是合法 TOML：

- 保留所有未知顶层字段和未知表。
- 设置或覆盖顶层 `model` 为用户选择的单个模型 ID。
- 设置或覆盖顶层 `model_provider` 为当前端点名称 `endpoint.name`。
- 新增或覆盖 `[model_providers.<endpoint.name>]` 表。
- 当前真实 CLI 配置写入预览不会设置 `disable_response_storage`；该字段只存在于临时 Codex 测试配置中。
- 不删除其他 `[model_providers.*]`，避免破坏用户已有 provider。

如果原文件不存在：创建基础 TOML。

如果原文件存在但不是合法 TOML：不自动覆盖，提示用户修复或在编辑确认框中手动处理。

`config.toml`：

```toml
model = "用户选择的模型 ID"
model_provider = "MyEndpoint"

[model_providers.MyEndpoint]
name = "MyEndpoint"
base_url = "https://example.com/v1"
wire_api = "responses"
```

字段处理：

- 修改：`model`、`model_provider`、`model_providers.<endpoint.name>`。
- 保留：其他顶层字段、其他 provider、sandbox、approval、history 等用户配置。
- 删除：初版不主动删除任何字段。
- Codex 应用配置时必须且只能选择一个模型，用该模型填充顶层 `model`。
- 最终写入内容以用户在编辑确认框中确认的 `edited_config` 为准。

## Claude 写入方案

仅当当前端点 `type` 为 `claude` 时执行。

目标文件：

- `~/.claude/settings.json`
- `~/.claude.json`

写入策略：在原有 `settings.json` 和 `~/.claude.json` 基础上修改并补齐 Claude Code 基础字段，不完整重写用户 Claude 设置。`build_cli_config_preview` 读取现有 JSON 并生成合并后的预览；用户可编辑预览内容；`apply_cli_config` 写入用户确认后的内容。

### Claude settings.json

如果原文件存在且是合法 JSON：

- 保留所有未知顶层字段。
- 保留 `env` 中除目标字段之外的其他环境变量。
- 如果缺少顶层 `$schema`，补齐为 `https://json.schemastore.org/claude-code-settings.json`。
- 如果缺少顶层 `includeGitInstructions`，补齐为 `false`。
- 设置或覆盖 `env.ANTHROPIC_BASE_URL` 为当前端点 `base_url`。
- 设置或覆盖 `env.ANTHROPIC_API_KEY` 为当前端点 `api_key`。
- 如果原配置已存在 `env.ANTHROPIC_AUTH_TOKEN`，同步覆盖为当前端点 `api_key`；如果原配置没有该字段，不主动新增。
- 设置或覆盖模型 ID 字段为用户当前选择的 Claude 模型，或用户确认追加 `[1m]` 后的模型名：`ANTHROPIC_MODEL`、`ANTHROPIC_DEFAULT_SONNET_MODEL`、`ANTHROPIC_DEFAULT_OPUS_MODEL`、`ANTHROPIC_DEFAULT_HAIKU_MODEL`、`ANTHROPIC_DEFAULT_FABLE_MODEL`。
- 如果原配置已存在模型显示名字段，则同步覆盖为同一个模型名：`ANTHROPIC_DEFAULT_SONNET_MODEL_NAME`、`ANTHROPIC_DEFAULT_OPUS_MODEL_NAME`、`ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME`、`ANTHROPIC_DEFAULT_FABLE_MODEL_NAME`；如果原配置没有这些字段，不主动新增。
- 如果缺少 Claude Code 行为控制字段，按基础格式补齐。
- 已存在的 `ANTHROPIC_AUTH_TOKEN` 会同步覆盖，避免旧 token 在 Claude 运行时继续优先生效；未存在时保持不写入，默认使用 `ANTHROPIC_API_KEY`。

如果原文件不存在：创建基础 JSON。

如果原文件存在但不是合法 JSON：不自动覆盖，提示用户修复或在编辑确认框中手动处理。

默认预览结构：

```json
{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "includeGitInstructions": false,
  "env": {
    "ANTHROPIC_BASE_URL": "https://example.com/v1",
    "ANTHROPIC_API_KEY": "sk-xxx",
    "ANTHROPIC_MODEL": "用户选择的模型 ID",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "用户选择的模型 ID",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "用户选择的模型 ID",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "用户选择的模型 ID",
    "ANTHROPIC_DEFAULT_FABLE_MODEL": "用户选择的模型 ID",
    "CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS": "1",
    "ENABLE_PROMPT_CACHING_1H": "1",
    "CLAUDE_CODE_ATTRIBUTION_HEADER": "0",
    "DISABLE_TELEMETRY": "1",
    "DISABLE_ERROR_REPORTING": "1",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
    "CLAUDE_CODE_ENABLE_BACKGROUND_PLUGIN_REFRESH": "0"
  }
}
```

说明：

- 修改：`env.ANTHROPIC_BASE_URL`、`env.ANTHROPIC_API_KEY`；若 `env.ANTHROPIC_AUTH_TOKEN` 已存在，也会同步覆盖。
- 补齐缺失字段：`$schema`、`includeGitInstructions`、Claude Code 行为控制 env；模型 ID env 会覆盖为当前选择的模型，模型显示名 env 仅在已存在时覆盖。
- 保留：其他 settings 字段、permissions、hooks、mcpServers、env 中其他变量等。
- 删除：初版不主动删除任何字段。
- 模型字段使用用户当前选择的 Claude 模型 ID，或用户确认追加 `[1m]` 后的模型名，填充 `ANTHROPIC_MODEL` 以及默认 Sonnet、Opus、Haiku、Fable 的 `*_MODEL` 字段；`*_MODEL_NAME` 字段仅在原配置已有时覆盖。
- Claude 应用配置时必须且只能选择一个模型。
- 如果当前没有选择模型，或选择了多个模型，预览生成失败并提示用户只选择一个模型。
- 最终写入内容以用户在编辑确认框中确认的 `edited_config` 为准。

### Claude ~/.claude.json

`~/.claude.json` 用于标记 Claude Code 已完成初始化，避免配置完成后仍要求登录或走 onboarding。

如果原文件存在且是合法 JSON：

- 保留所有未知字段。
- 设置或覆盖 `hasCompletedOnboarding = true`。

如果原文件不存在：创建基础 JSON。

如果原文件存在但不是合法 JSON：不自动覆盖，提示用户修复或在编辑确认框中手动处理。

默认预览结构：

```json
{
  "hasCompletedOnboarding": true
}
```

说明：

- 修改：`hasCompletedOnboarding`。
- 保留：其他 Claude Code 状态字段。
- 删除：初版不主动删除任何字段。
- 该文件也要纳入原始基线和还原列表，`file_id` 为 `claude-state`。

## OpenCode 写入方案

仅当当前端点 `type` 为 `opencode` 时执行。OpenCode 使用 OpenAI 兼容 provider，但端点类型独立，不复用 `codex` 类型凭证；`claude` 类型凭证不写入 OpenCode。

目标文件：

- 由 `paths.rs` 按平台解析的 OpenCode 配置文件，Linux/XDG 示例为 `~/.config/opencode/opencode.json`

OpenCode 的目标是“新增入口”，因此不能重写整个配置。推荐策略：

- 如果文件不存在，创建基础 JSON 配置。
- 如果文件存在，先解析原配置并追加入口。
- 新入口 key 使用当前端点名称 `endpoint.name`。
- 如果同名 provider 已存在，预览中覆盖该入口，并返回 `OpenCodeProviderOverwrite` warning。
- 前端必须在展示写入预览前要求用户确认覆盖；用户取消则不继续应用。

建议写入结构：

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "provider": {
    "MyEndpoint": {
      "npm": "@ai-sdk/openai-compatible",
      "options": {
        "baseURL": "https://example.com/v1",
        "apiKey": "sk-xxx"
      },
      "models": {
        "model-a": {
          "name": "model-a"
        },
        "model-b": {
          "name": "model-b"
        }
      }
    }
  }
}
```

说明：

- 上述结构是默认预览内容，最终写入内容以用户在编辑确认框中确认的 `edited_config` 为准。
- OpenCode 应用配置时允许选择多个模型。默认预览会把用户选择的每个模型写入 `provider.<endpoint.name>.models`。
- OpenCode 可选择默认模型；若选择，预览会写入顶层 `model` 和 `small_model`，值为 `<endpoint.name>/<model>`。
- 当前实现内置轻量 JSONC 处理：移除注释和尾逗号后用 `serde_json` 解析，最终写回 pretty JSON。
- 文件存在但解析后不是对象，或无法解析为 JSON/JSONC 时，预览失败且不写入。

字段处理：

- 修改：`provider` 对象，新增 `<endpoint.name>` entry，并写入用户选择的多个模型；可选修改顶层 `model` 和 `small_model`。
- 保留：除新增 entry 外的所有字段，包括已有 provider、model、theme、mcp、formatter 等。
- 删除：应用 OpenCode 配置时不主动删除任何字段，不覆盖已有同名 entry。“从 OpenCode 移除”会按当前端点的 URL 和 API Key 匹配 provider 并删除，若顶层 `model` 或 `small_model` 指向该 provider，也会一并移除。

## 真实 CLI 配置测试方案

该测试用于验证“应用到 Codex / Claude / OpenCode”后真实 CLI 配置是否可用。它与现有模型测试不同：

- 不写入 `~/.TestModelAlive/codex-home/`。
- 不设置 `CODEX_HOME`。
- 不生成 `~/.TestModelAlive/claude-settings.json`。
- Claude 不传 `--settings`。
- OpenCode 使用默认配置目录。
- 不修改任何真实 CLI 配置，只运行命令并收集输出。

入口：测试模型页控制栏中的“测试当前配置”按钮，以及应用成功结果弹窗中的“用当前配置测试”按钮。

### Codex 真实配置测试

适用条件：`target == codex`。

模型规则：必须且只能选择一个模型。该模型已经写入 `~/.codex/config.toml` 顶层 `model`，测试命令不再额外传模型参数。

建议命令：

```text
codex exec --skip-git-repo-check "{prompt}"
```

运行环境：

- 不设置 `CODEX_HOME`。
- 继承当前进程环境变量。
- 依赖 Codex 默认读取 `~/.codex/config.toml` 和 `~/.codex/auth.json`。

### Claude 真实配置测试

适用条件：`target == claude`。

模型规则：必须且只能选择一个模型。该模型已经写入 `~/.claude/settings.json` 的 Claude 模型 env 字段，测试命令不再额外传 `--model`，避免与真实配置不一致。

建议命令：

```text
claude --debug --verbose -p "{prompt}"
```

运行环境：

- 不传 `--settings`。
- 继承当前进程环境变量。
- 依赖 Claude 默认读取 `~/.claude/settings.json` 和 `~/.claude.json`。

### OpenCode 真实配置测试

适用条件：`target == opencode`。

模型规则：允许一个或多个模型。对每个选中模型分别运行一次测试。

当前实现命令：

```text
opencode run --model "{endpoint_name}/{model}" "{prompt}"
```

OpenCode 真实配置测试要求请求携带 `endpoint_name`，通常为当前端点名称。

运行环境：

- 不设置临时配置目录。
- 继承当前进程环境变量。
- 依赖 OpenCode 默认读取 `~/.config/opencode/opencode.json`。

### 输出与成功判断

- 通过 `Channel<TestMessage>` 复用现有日志和结果事件。
- 每个模型生成一条 `TestResult` 结果事件。
- 成功判断复用现有 `success_keyword` 匹配逻辑。
- 超时使用测试弹窗的超时时间输入值，默认 120 秒。
- 日志输出 `starting real CLI config test: target=<target>`，避免和临时配置测试混淆。

## 校验规则

后端写入前校验：

- `endpoint.base_url.trim()` 不能为空。
- `endpoint.api_key.trim()` 不能为空。
- `base_url` 去掉尾部 `/`。
- `api_key` 去掉首尾空白。
- `endpoint.type` 只接受当前已有的 `codex` / `claude` / `opencode`。
- `target` 只接受 `codex`、`claude`、`opencode`。
- `target == "codex"` 时要求 `endpoint.type == "codex"`。
- `target == "claude"` 时要求 `endpoint.type == "claude"`。
- `target == "opencode"` 时要求 `endpoint.type == "opencode"`。
- `target == "codex"` 时要求 `selected_models.len() == 1`。
- `target == "claude"` 时要求 `selected_models.len() == 1`。
- `target == "opencode"` 时要求 `selected_models.len() >= 1`，允许多个模型。
- `edited_config.files` 不能为空。
- `edited_config.files[*].path` 必须匹配后端为该 target 计算出的允许目标路径，不能由前端改写到任意路径。
- `edited_config.files[*].content` 不能为空。

前端也复用现有校验和脱敏展示。

## 错误处理

常见错误提示：

- 无当前端点：`请先打开一个测试端点。`
- URL 为空：`端点 URL 不能为空。`
- Key 为空：`API Key 不能为空。`
- 端点类型不支持：`仅支持 codex、claude 或 opencode 类型端点。`
- 目标不支持：`仅支持 Codex、Claude 或 OpenCode。`
- 端点类型与目标不匹配：`当前端点不能应用到所选 CLI。`
- 未选择模型：`请至少选择一个模型。`
- Codex/Claude 多选模型：`应用到 Codex 或 Claude 时只能选择一个模型。`
- 编辑后的配置为空：`配置内容不能为空。`
- 编辑后的目标路径不合法：`配置目标路径不合法。`
- 未选择还原项：`请选择需要还原的配置。`
- 创建目录失败：返回系统错误。
- 备份失败：停止写入该目标，返回错误。
- 原始基线写入失败：停止本次应用，不修改真实 CLI 配置。
- 读取或写入 `settings.json` 失败：停止本次应用，不修改真实 CLI 配置。
- 写入失败：返回目标路径和系统错误。
- Codex `auth.json` 无法解析：返回 `Codex auth.json 不是合法 JSON，未修改，请先修复或手动编辑。`
- Codex `config.toml` 无法解析：返回 `Codex config.toml 不是合法 TOML，未修改，请先修复或手动编辑。`
- Claude `settings.json` 无法解析：返回 `Claude settings.json 不是合法 JSON，未修改，请先修复或手动编辑。`
- Claude `~/.claude.json` 无法解析：返回 `Claude ~/.claude.json 不是合法 JSON，未修改，请先修复或手动编辑。`
- OpenCode 配置无法解析：返回 `OpenCode 配置不是合法 JSON，已备份但未修改，请手动合并。`
- 无原始基线：`没有可还原的 CLI 原始配置。`
- 备份文件不存在：`备份文件不存在，无法还原该配置。`

部分成功时返回错误会隐藏成功结果。为提升可用性，`apply_cli_config` 内部对当前 `target` 对应文件分别执行并返回每项状态：

```rust
struct CliConfigWriteResult {
    target: String,
    path: String,
    backup_paths: Vec<String>,
    action: String,
    ok: bool,
    error: Option<String>,
}
```

整体命令只在请求参数无效、目标不支持、端点类型不匹配、编辑后的配置不合法、`settings.json` 无法读写时返回 `Err`；单个文件写入失败时返回 `Ok(result)` 并由前端展示部分失败。

`restore_original_cli_config` 只在无法读取或解析 `settings.json`、未选择还原项、选择项不在基线中时返回 `Err`；单个文件还原失败时返回 `Ok(result)` 并由前端展示部分失败。

## 安全与用户确认

- 确认弹窗必须说明 API Key 会写入本机 CLI 明文配置。
- 确认弹窗展示 Key 时使用现有 `maskKey()`。
- 应用确认弹窗必须展示可编辑配置内容，并说明“确认后将按编辑框中的内容写入真实 CLI 配置”。
- 成功结果中不返回明文 Key。
- 成功结果展示 `baseline_path`，提示用户可通过“一键还原”恢复到本应用首次修改前的原始配置。
- 还原确认弹窗必须说明会覆盖当前 CLI 配置，并会先备份当前配置为 `*.pre-restore.<timestamp>.bak`。
- 文档和 README 后续需要补充该功能会修改真实 CLI 配置文件。

## 源码文件规划

当前实现已经完成后端和前端的模块拆分，`src-tauri/src/lib.rs` 只保留 Tauri command 注册，`frontend/main.ts` 只负责启动应用。

### 后端 Rust

当前后端结构：

```text
src-tauri/src/
  main.rs
  lib.rs
  models.rs
  paths.rs
  settings.rs
  endpoints.rs
  test_runner/
    mod.rs
    codex.rs
    claude.rs
    opencode.rs
    process.rs
  cli_config/
    mod.rs
    types.rs
    commands.rs
    backup.rs
    preview.rs
    restore.rs
    codex.rs
    claude.rs
    opencode.rs
```

职责划分：

- `models.rs`：共享数据结构，例如 `SavedEndpoint`、`TestSettings`、`TestResult`、通用返回结构。
- `paths.rs`：`~/.TestModelAlive`、真实 CLI 配置路径、备份目录、路径展开等。
- `settings.rs`：`~/.TestModelAlive/settings.json` 的读写、默认值、版本迁移、原子写入。
- `endpoints.rs`：端点增删改查，数据写入 `settings.json`。
- `test_runner/mod.rs`：测试模型的公共调度。
- `test_runner/codex.rs`：Codex 测试临时配置和命令执行。
- `test_runner/claude.rs`：Claude 测试临时 settings 和命令执行。
- `test_runner/opencode.rs`：OpenCode 真实配置测试命令执行。
- `test_runner/process.rs`：子进程启动、输出流、超时和取消控制。
- `cli_config/types.rs`：`CliConfigTargetKind`、`CliConfigPreview`、`EditedCliConfig`、`RestoreSelection`、写入/还原结果类型。
- `cli_config/commands.rs`：Tauri command 薄封装，只做参数校验入口和调用业务函数。
- `cli_config/backup.rs`：原始基线、普通备份、pre-restore 备份、备份路径生成。
- `cli_config/preview.rs`：按 target 调用具体 CLI 预览生成器。
- `cli_config/restore.rs`：按勾选项从 `settings.json.cli_config.baseline_items` 还原。
- `cli_config/codex.rs`：Codex `auth.json` 和 `config.toml` 的解析、合并、预览、写入。
- `cli_config/claude.rs`：Claude `settings.json` 的解析、合并、预览、写入。
- `cli_config/opencode.rs`：OpenCode `opencode.json` 的解析、合并、预览、写入。

拆分原则：

- `lib.rs` 不放业务细节，只注册 commands 和初始化状态。
- 每个 CLI 的配置格式处理放在自己的文件里。
- 文件系统备份和 settings 索引更新不要散落在 Codex/Claude/OpenCode 文件里，由 `backup.rs` 和 `settings.rs` 统一处理。
- command 函数保持薄层，方便单元测试直接测业务函数。
- 原有代码允许重构，但重构时保持现有 command 名称和前端行为，除非本设计明确新增或替换。

### 前端 TypeScript

当前前端结构：

```text
frontend/
  main.ts
  i18n.ts
  styles.css
  types.ts
  state.ts
  api/
    tauri.ts
    endpoints.ts
    testModels.ts
    cliConfig.ts
  ui/
    renderApp.ts
    modal.ts
    endpointForm.ts
    endpointList.ts
    modelList.ts
    testDialog.ts
    testSettingsDialog.ts
    cliConfigDialog.ts
    restoreConfigDialog.ts
    logPanel.ts
    elements.ts
    app.ts
  utils/
    mask.ts
    dom.ts
```

职责划分：

- `main.ts`：应用启动。
- `ui/app.ts`：初始化状态、绑定顶层事件、协调各 UI 模块。
- `types.ts`：前端共享类型，例如 `SavedEndpoint`、`CliConfigPreview`、`RestoreSelection`。
- `state.ts`：当前端点、模型选择、测试状态、语言等前端状态。
- `api/tauri.ts`：`invoke` 包装、错误格式化。
- `api/endpoints.ts`：端点读写相关 API。
- `api/testModels.ts`：拉取模型和测试模型 API。
- `api/cliConfig.ts`：`build_cli_config_preview`、`apply_cli_config`、`restore_original_cli_config` API。
- `ui/renderApp.ts`：主界面骨架渲染。
- `ui/modal.ts`：通用确认框/编辑框基础能力。
- `ui/testDialog.ts`：测试模型弹窗；根据端点类型展示应用按钮。
- `ui/cliConfigDialog.ts`：应用前的配置预览和编辑确认弹窗、应用结果弹窗。
- `ui/restoreConfigDialog.ts`：主界面“还原配置”勾选弹窗。
- `utils/mask.ts`：API Key 脱敏。
- `utils/dom.ts`：DOM 查询和事件辅助函数。

拆分原则：

- `main.ts` 不继续增长成所有 UI 和业务逻辑的集合。
- Tauri API 调用集中到 `api/`，UI 组件不直接拼 `invoke` 参数细节。
- 弹窗独立文件实现，尤其是配置预览编辑和还原勾选弹窗。
- `i18n.ts` 继续保留文案，但新增文案按功能分组排列。
- 样式可以先保留在 `styles.css`，如果继续增长，再拆成 `frontend/styles/*.css`。

### 已完成重构顺序

1. 后端已抽出 `models.rs`、`paths.rs`、`settings.rs`。
2. 后端已抽出 `test_runner/`，Codex/Claude/OpenCode 测试逻辑从 `lib.rs` 移出。
3. 后端已新增 `cli_config/` 模块，实现预览、应用、备份、还原和 OpenCode 移除预览。
4. 前端已抽出 `types.ts`、`state.ts`、`api/`。
5. 前端已抽出测试弹窗、设置弹窗、日志、端点表单/列表、模型列表。
6. 前端已新增配置预览编辑弹窗和还原勾选弹窗。

这样可以把“结构性重构”和“功能实现”分阶段提交，降低一次性改动风险。

## 实施步骤

1. 已完成源码文件拆分，把 `src-tauri/src/lib.rs` 和 `frontend/main.ts` 中与本功能相关的逻辑迁移到独立模块。
2. 已新增返回结构体、`build_cli_config_preview`、`build_remove_opencode_config_preview`、`load_cli_config_baseline_items`、`apply_cli_config`、`restore_original_cli_config` 和 `test_cli_with_real_config` command。
3. 已实现 `settings.json` 统一读写、原始基线写入、真实 CLI 路径解析、Codex 写入、Claude 写入、OpenCode 新增入口。
4. 已实现按 `selected_items` 读取原始基线并恢复配置。
5. 已注册 command 到 `tauri::generate_handler!`。
6. 已在前端 i18n 增加按钮、确认、结果和错误文案。
7. 已在前端测试控制栏按端点类型展示应用按钮：`codex` 展示 Codex，`claude` 展示 Claude，`opencode` 展示 OpenCode 和从 OpenCode 移除。
8. 已在前端主界面增加“还原配置”按钮和勾选式还原弹窗。
9. 已实现点击应用按钮后调用 `build_cli_config_preview`，展示可编辑配置内容。
10. 已实现在用户确认后调用 `invoke("apply_cli_config", { endpoint: testEndpoint, target, editedConfig })`。
11. 已在测试模型页和应用成功结果弹窗中提供“用当前配置测试”入口，调用 `test_cli_with_real_config`。
12. README 已补充存储/安全说明。
13. 仍建议手动验证：`codex` 端点只展示并应用 Codex、`claude` 端点只展示并应用 Claude、`opencode` 端点只展示并应用 OpenCode/移除 OpenCode、应用前可编辑配置内容、目标文件不存在、目标文件存在、OpenCode 配置无法 JSON/JSONC 解析、勾选还原到本应用首次修改前的原始配置、应用后用真实 CLI 配置测试。

## 测试建议

- Rust 单元测试覆盖路径无关的内容生成函数。
- 使用临时目录测试备份命名和写入逻辑。
- 测试 `codex` 端点只展示 Codex 应用按钮。
- 测试 `claude` 端点只展示 Claude 应用按钮。
- 测试 `opencode` 端点只展示 OpenCode 应用按钮。
- 测试 `build_cli_config_preview` 只生成预览，不写文件、不备份。
- 测试 `apply_cli_config` 写入用户编辑后的配置内容。
- 测试 `test_cli_with_real_config` 不设置 `CODEX_HOME`，Claude 不传 `--settings`，OpenCode 不使用临时配置目录。
- 测试应用前先写入 `settings.json.cli_config.baseline_items`，且后续多次应用不会覆盖已有基线 item。
- 测试还原弹窗勾选哪些 item 就只还原哪些 item。
- 测试目标文件原本不存在时，还原会删除本应用新建文件。
- 测试目标文件原本存在时，还原会用基线中的 `backup_path` 覆盖回去。
- 测试还原前当前配置会生成 `*.pre-restore.<timestamp>.bak`。
- 前端手动测试：未选择端点、取消确认、确认成功、部分失败展示。
- 在 Windows 下验证 home 解析为 `%USERPROFILE%`，OpenCode 路径按实际 CLI 位置解析。
- 在 macOS 下验证 home、`Application Support` 或 OpenCode 实际配置路径解析。
- 在 Linux 下验证 `${XDG_CONFIG_HOME:-$HOME/.config}` 解析。
- 在 Windows 下验证配置 replace 逻辑能覆盖已存在文件。
- 在三端验证命令执行不依赖 shell 转义，prompt 中包含引号和换行也能作为单个参数传入。

## 当前实现仍需关注

- OpenCode 当前实现使用 `provider.<endpoint.name>` 配置结构和 `@ai-sdk/openai-compatible` provider；如果 OpenCode CLI 后续配置格式变化，需要同步更新 `cli_config/opencode.rs` 和 `test_runner/opencode.rs`。
- OpenCode 路径当前由 `paths.rs` 集中解析：Windows 为 `%USERPROFILE%\.config\opencode\opencode.json`，macOS 为 `$HOME/Library/Application Support/opencode/opencode.json`，Linux 为 `${XDG_CONFIG_HOME:-$HOME/.config}/opencode/opencode.json`。如果 CLI 文档调整默认路径，应只改 `paths.rs`。
- OpenCode 真实配置测试当前命令为 `opencode run --model "<endpoint_name>/<model>" "<prompt>"`。如果 CLI 命令格式变化，应只改 `test_runner/opencode.rs`。
