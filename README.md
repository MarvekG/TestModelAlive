# TSA

用于检查 Codex CLI 和 Claude CLI 是否可以正常调用指定 API 端点和模型 ID 的
Python 小工具。

## 文件

- `test_codex_models.py`：通过 `codex exec` 测试 OpenAI 兼容端点。
- `test_claude_models.py`：通过 `claude -p` 测试 Anthropic 兼容端点。
- `codex.txt`：本地 Codex 端点列表，已被 git 忽略。
- `claude.txt`：本地 Claude 端点列表，已被 git 忽略。

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

脚本运行测试时会临时写入 CLI 配置，并在退出前恢复原始文件。端点文件可能包含
API Key，请不要提交或公开这些文件。

## 友情链接

- [Linux Do](https://linux.do/)

## 许可证

MIT。详见 [LICENSE](LICENSE)。
