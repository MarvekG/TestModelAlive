# TestModelAlive

[English](README.en.md)

TestModelAlive 是一个 Tauri 桌面应用，用于管理 Codex / Claude / OpenCode / DeepSeek Harness 兼容 API 端点，并通过本机 CLI 工具测试已保存模型是否可用。

应用支持中英双语界面，默认中文。

## 总体设计思路

1. 主页：填写模型 API 信息，拉取模型列表，保存 API 和对应的模型。
2. 模型测试页：为测活生成临时配置文件，按需替换 Codex / Claude / OpenCode / DeepSeek Harness 配置，再使用替换后的配置进行测活。

测活指测试模型是否可用。测试时会向模型发送指定提示词，并根据命令输出中是否包含成功关键词判断模型是否通过。

## 主页使用步骤

1. 填写端点名称、类型、URL 和 API Key。
2. 点击“拉取模型”，确认模型列表后勾选需要保存的模型。
3. 点击“保存端点”，在“已保存端点”中选择、加载、复制或删除端点。

![主页](https://github.com/user-attachments/assets/3d0cca52-9309-4e4d-8375-2eb44c27cde1)

## 模型测试页使用步骤

1. 在主页选择一个已保存端点，点击“测试”进入模型测试页；页面会展示端点类型、URL 和脱敏后的 API Key。
2. 先确认要测试的模型列表，可重新“拉取模型”、保存模型，也可使用全选、全不选、反选快速调整范围。
3. 按需设置超时时间；Claude 端点可勾选“模型后追加 1M 上下文 [1m]”测试长上下文模型名。
4. 点击“测试设置”可修改测试提示词和成功关键词；提示词必须要求模型输出该关键词，系统会用它判断测试是否通过。
5. 点击“开始测试”测试当前端点，或点击“测试当前配置”验证本机 CLI 现有配置；测试过程中可停止，并通过结果区和日志区查看状态、耗时、错误输出。测试通过后再按需应用到 Codex、Claude、OpenCode 或 DeepSeek Harness。

测试模型是否可用：

![模型测活](https://github.com/user-attachments/assets/1a769d4a-210c-42fd-8851-869d46eaf66c)

替换 CLI 配置：

![替换 CLI 配置](https://github.com/user-attachments/assets/92beed45-5e0f-4279-b5dc-fe2794b87370)

## 技术栈

- 前端：TypeScript + Vite，源码在 `frontend/`。
- 桌面运行时：Tauri 2 + Rust，源码在 `src-tauri/`。
- 旧版 Python/PyQt 文件保留在 `lagacy/`。

## 环境要求

- Node.js 和 npm。
- Rust stable toolchain。
- 当前平台所需的 Tauri 系统依赖。
- 根据测试类型安装本机 CLI：
   - Codex 端点需要 `codex`。
   - Claude 端点需要 `claude`。
   - OpenCode 端点需要 `opencode`。
   - DeepSeek Harness 端点需要 `dsh`，可通过 `npm install -g @deepseek-ai/dsh` 安装。

应用会在 `PATH` 和常见安装位置中查找 CLI，包括 Windows 的 npm 全局路径和 macOS 的 Homebrew 路径。

## 开发

安装依赖：

```bash
npm install
```

启动 Tauri 开发模式：

```bash
npm run tauri dev
```

只构建前端：

```bash
npm run build
```

构建桌面应用：

```bash
npm run tauri build
```

## 数据目录

运行时数据保存在用户目录下：

```text
~/.TestModelAlive/
```

Windows 下对应：

```text
%USERPROFILE%\.TestModelAlive\
```

目录内主要文件包括：

- `settings.json`：已保存端点、模型列表、测试设置和 CLI 配置还原基线。
- `cli-config-apply-history.json`：CLI 配置应用历史。
- `claude-settings.json`：测试 Claude CLI 时生成的临时 settings 文件，每次测试会直接覆盖，保留用于排查问题。
- `codex-home/`：测试 Codex CLI 时使用的独立 Codex home。
- `opencode-home/`：测试 OpenCode CLI 时使用的独立 home。
- `dsh-home/`：测试 DeepSeek Harness 时使用的隔离 `DSH_HOME`。
- `cli-config-backups/`：应用或还原真实 CLI 配置前生成的备份。

## 模型测试

模型测试通过本机 CLI 执行：

- Codex 测试使用独立 `CODEX_HOME`，路径为 `~/.TestModelAlive/codex-home`。
- Claude 测试使用 `~/.TestModelAlive/claude-settings.json` 作为 settings 文件。
- OpenCode 测试使用独立 home，路径为 `~/.TestModelAlive/opencode-home`。
- DeepSeek Harness 测试使用隔离 `DSH_HOME`，路径为 `~/.TestModelAlive/dsh-home`，通过 `dsh --profile headless` 运行；API Key 仅作为子进程环境变量注入。

测试弹窗会实时显示 CLI 输出。后端不会再把测试日志镜像输出到终端。

成功判定可配置：

- 设置测试提示词。
- 设置成功匹配关键词。
- 提示词必须明确包含成功关键词，并要求模型输出它。
- 命令输出中包含成功关键词时，模型会被标记为可用。

## DeepSeek Harness 配置

- 在 DeepSeek Harness 类型端点中选择一个或多个模型后，可点击“应用到 DeepSeek Harness”，并选择其中一个默认模型。
- 应用会合并 DSH 的 `settings.yaml` 和 `.credentials.yaml`，路径为 `DSH_HOME`（如已设置）或默认的 `~/.dsh/`。
- `deepseek-v4-flash` 和 `deepseek-v4-pro` 默认与其他模型一起写入 OpenAI 兼容的 `tma-<端点名称>` provider。`llm-pi-ai` 尚未适配第三方端点的 DeepSeek 模型；要使用它们作为默认模型，需在应用选项中勾选“拆分 DeepSeek V4 模型到 llm-deepseek”，将它们写入原生 `llm-deepseek` 配置，并使用 `deepseek-official` 作为 `agent-default-model` provider。未勾选时，DeepSeek V4 模型不能作为默认模型。
- **拆分会覆盖 `llm-deepseek` 中已有的 DeepSeek 官方配置。**
- 拆分时可选择最大输出：官方端点默认 `384000`，第三方端点可选 `131072`。隔离测活固定使用 `131072`，不读取已有 DSH 配置。
- 会复用内嵌的模型上下文和最大输出值，`llm-pi-ai` 模型还会转换 OpenCode 推理档位，未知模型保留 DSH 默认能力。
- 模型元数据内嵌在应用的 `src-tauri/src/model_metadata.json`，不会写入 `~/.TestModelAlive/settings.json`；启动时会自动移除旧的 `opencode_model_variants` 字段。
- DeepSeek Harness 目前处于开发预览阶段，配置格式可能会有破坏性变更；升级 DSH 后请先执行一次“测试当前配置”。

## 平台说明

- Windows 支持 `.cmd` / `.bat` CLI shim，并通过 `taskkill /T /F` 终止进程树。
- macOS 和 Linux 会额外查找 `/usr/local/bin`、`/opt/homebrew/bin`、`~/.local/bin` 等常见 CLI 路径。
- Tauri 跨平台打包可能需要对应平台的 SDK、资源编译器或系统依赖，仅安装 Rust target 不一定足够。

## 友链

- [LINUX DO](https://linux.do/)

## 安全提醒

API Key 会以明文形式保存在 `~/.TestModelAlive/settings.json`，应用到本机 CLI 后也会写入对应 CLI 配置。

不要提交或公开运行时数据文件。相关本地数据文件已加入 `.gitignore`。
