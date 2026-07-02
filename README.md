# TestModelAlive

[English](README.en.md)

TestModelAlive 是一个 Tauri 桌面应用，用于管理 Codex / Claude 兼容 API 端点，并通过本机 CLI 工具测试已保存模型是否可用。

应用支持中英双语界面，默认中文。

## 功能

- 添加并保存 `codex` / `claude` API 端点。
- 从端点 `/models` API 拉取模型列表。
- 勾选模型并随端点保存。
- 管理已保存端点，支持单个删除和批量删除。
- 复制端点 URL / API Key。
- 通过本机 `codex` 或 `claude` CLI 测试已保存模型。
- 在测试弹窗中实时显示 CLI 输出。
- 自定义测试提示词和成功匹配关键词。
- 将端点数据和测试设置持久化到用户目录。
- 支持中文 / English 界面切换。

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

- `tsa_endpoints.json`：已保存端点和模型列表。
- `test_settings.json`：测试提示词和成功匹配关键词。
- `claude-settings.json`：测试 Claude CLI 时生成的临时 settings 文件。
- `codex-home/`：测试 Codex CLI 时使用的独立 Codex home。

如果当前工作目录存在旧版 `tsa_endpoints.json` 或 `test_settings.json`，应用首次使用时会复制到 `~/.TestModelAlive/`。

## 模型测试

模型测试通过本机 CLI 执行：

- Codex 测试使用独立 `CODEX_HOME`，路径为 `~/.TestModelAlive/codex-home`。
- Claude 测试使用 `~/.TestModelAlive/claude-settings.json` 作为 settings 文件。

测试弹窗会实时显示 CLI 输出。后端不会再把测试日志镜像输出到终端。

成功判定可配置：

- 设置测试提示词。
- 设置成功匹配关键词。
- 提示词必须明确包含成功关键词，并要求模型输出它。
- 命令输出中包含成功关键词时，模型会被标记为可用。

## 跨平台说明

- Linux 是当前仓库的主要开发环境。
- Windows 支持 `.cmd` / `.bat` CLI shim，并通过 `taskkill /T /F` 终止进程树。
- macOS 和 Linux 会额外查找 `/usr/local/bin`、`/opt/homebrew/bin`、`~/.local/bin` 等常见 CLI 路径。
- Tauri 跨平台打包可能需要对应平台的 SDK、资源编译器或系统依赖，仅安装 Rust target 不一定足够。

## 安全提醒

API Key 会以明文形式保存在 `~/.TestModelAlive/tsa_endpoints.json`。

不要提交或公开运行时数据文件。相关本地数据文件已加入 `.gitignore`。
