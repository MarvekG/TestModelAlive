# TSA

用于检查 Codex CLI 和 Claude CLI 是否可以正常调用指定 API 端点和模型 ID 的
Python 小工具。

## 文件

- `test_codex_models.py`：通过 `codex exec` 测试 OpenAI 兼容端点。
- `test_claude_models.py`：通过 `claude -p` 测试 Anthropic 兼容端点。
- `tsa_gui.py`：PyQt6 GUI，用于管理端点、拉取模型并测活。
- `codex.txt`：本地 Codex 端点列表，已被 git 忽略。
- `claude.txt`：本地 Claude 端点列表，已被 git 忽略。

## GUI 用法

GUI 依赖 PyQt6：

```bash
python3 -m pip install PyQt6
```

启动 GUI：

```bash
python3 tsa_gui.py
```

GUI 会在当前目录读写 `tsa_endpoints.json`，用于保存端点 URL、API Key 和已选择模型。
该文件包含明文 API Key，已被 `.gitignore` 忽略，请不要提交或公开。

如果 Linux 桌面环境中中文显示为方块或乱码，请先安装中文字体，例如：

```bash
sudo apt install fonts-noto-cjk
```

或：

```bash
sudo apt install fonts-wqy-microhei
```

PyQt6 会使用 Qt/Fontconfig 字体系统，通常能正确使用系统中的中文字体。

基本流程：

1. 选择 `codex` 或 `claude`。
2. 填写端点 URL 和 SK。
3. 点击“拉取模型”。
4. 勾选要保存的模型，可使用“全选 / 全不选 / 反选”。
5. 点击“保存端点”。
6. 在已保存端点列表中选择端点，可执行“测试 / 删除 / 刷新 / 加载 / 复制 URL / 复制 KEY”。
7. 点击“加载”会把端点类型、URL、SK 和保存的模型回填到左侧输入区。
8. 点击“测试”打开测试弹窗，选择模型并开始测活。
9. Claude 端点测试时可勾选“模型后追加 1M 上下文 [1m]”，测试时会把模型 ID 后缀追加为 `[1m]`，不会修改保存数据。

GUI 拉取模型时支持普通 JSON、`gzip` 和 `deflate` 压缩响应。即使服务端漏写 `Content-Encoding: gzip`，只要响应体是 gzip 格式也会自动解压。

## 输入格式

端点文件每行一个端点(URL; API-kEY; 模型1,模型2)：

```text
https://example.com/v1;sk-...;model-1,model-2
```

第三列模型列表是可选的。如果省略，脚本会使用 `--models` 传入的模型列表。
字段分隔符和模型分隔符同时支持英文和中文标点。

## 用法

常用测试案例：

```bash
python3 test_codex_models.py --domain kimi
```

```bash
python3 test_claude_models.py --domain kimi
```

## Codex 脚本参数

- `--api-file PATH`：端点文件路径，默认 `codex.txt`。
- `--codex-dir PATH`：Codex 配置目录，默认 `~/.codex`。
- `--fetch-timeout SECONDS`：请求 `/models` 的超时时间，默认 30 秒。
- `--codex-timeout SECONDS`：每次执行 `codex exec` 的超时时间，默认 120 秒。
- `--models MODEL1,MODEL2`：当端点行没有第三列时使用的模型列表，默认 `gpt-5.5`。
- `--models-check`：先请求 `/models`，只测试请求模型和服务端模型列表的交集。
- `--domain TEXT`：只测试 URL 中包含 `TEXT` 的端点。
- `--last-only`：只测试输入文件中的最后一个有效端点。
- `--limit N`：每个端点最多测试 `N` 个模型。
- `--list-only`：只拉取并打印模型列表，不修改 Codex 配置。
- `--list-models`：同 `--list-only`。
- `-v, --verbose`：启用详细日志。

示例：

```bash
python3 test_codex_models.py --domain kimi --models gpt-5.5
```

## Claude 脚本参数

- `--claude-file PATH`：端点文件路径，默认 `claude.txt`。
- `--timeout SECONDS`：每次请求 `/models` 或执行 `claude -p` 的超时时间，默认 120 秒。
- `--models MODEL1,MODEL2`：当端点行没有第三列时使用的模型列表，默认 `claude-opus-4-6`。
- `--models-check`：先请求 `/models`，只测试请求模型和服务端模型列表的交集。
- `--domain TEXT`：只测试 URL 中包含 `TEXT` 的端点。
- `--last-only`：只测试输入文件中的最后一个有效端点。
- `--limit N`：每个端点最多测试 `N` 个模型。
- `--list-models`：只拉取并打印模型列表，不执行 `claude -p`。
- `-v, --verbose`：启用详细日志。

示例：

```bash
python3 test_claude_models.py --domain kimi --models claude-opus-4-6
```

## 注意事项

脚本和 GUI 运行测试时会临时写入 CLI 配置，并在测试结束、异常、超时或点击停止后恢复原始文件。
Codex 会临时修改 `~/.codex/auth.json` 和 `~/.codex/config.toml`；Claude 会临时修改当前目录的 `claude-settings.json`。
端点文件和 `tsa_endpoints.json` 可能包含 API Key，请不要提交或公开这些文件。

## 友情链接

- [Linux Do](https://linux.do/)

## 许可证

MIT。详见 [LICENSE](LICENSE)。
