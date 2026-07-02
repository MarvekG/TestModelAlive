# TSA

TSA 是一个用于管理 Codex / Claude API 端点并测试模型可用性的桌面工具。

当前 GUI 已重构为 Tauri 技术栈：

- 前端：TypeScript + Vite，源码在 `frontend/`。
- 桌面壳与本地能力：Tauri + Rust，源码在 `src-tauri/`。
- 数据文件：继续使用当前目录的 `tsa_endpoints.json`，兼容旧版 GUI 数据格式。

## 开发

安装依赖：

```bash
npm install
```

启动 Tauri 开发模式：

```bash
npm run tauri dev
```

构建前端：

```bash
npm run build
```

构建本地二进制：

```bash
npm run tauri build
```

构建后的 Linux 二进制位于：

```text
src-tauri/target/release/tsa
```

## 功能

- 添加 `codex` 或 `claude` 端点。
- 调用端点 `/models` 接口拉取模型列表。
- 保存勾选的模型到 `tsa_endpoints.json`。
- 管理已保存端点：测试、删除、加载、复制 URL、复制 KEY。
- 测试模型时临时写入 CLI 配置，并在测试结束后恢复。
- Claude 测试支持给模型 ID 追加 `[1m]`。

## 安全提醒

`tsa_endpoints.json` 包含明文 API Key，已被 `.gitignore` 忽略。不要提交或公开这个文件。

旧版 PyQt6 GUI 和 Python CLI 脚本已移动到 `lagacy/`。
