# TSA GUI 设计文档

## 目标

为现有 TSA 脚本新增一个 Python 原生 GUI，用于管理 Codex/Claude API 端点、拉取模型、保存模型列表，并对已保存模型执行测活。

第一版使用 `PyQt6` 实现。

## 范围

第一版包含：

- 选择端点类型：`Codex` 或 `Claude`。
- 输入端点 URL 和 API Key。
- 调用端点 `/models` 接口拉取模型列表。
- 勾选模型并保存到当前目录 JSON 文件。
- 模型列表支持“全选 / 全不选 / 反选”。
- 启动时自动加载已保存端点。
- 主界面展示已保存端点列表。
- 每个端点提供“测试”按钮。
- 已保存端点支持“加载 / 复制 URL / 复制 KEY”。
- 点击“测试”后弹出测试窗口，列出该端点已保存模型。
- 在测试窗口中选择模型并测活。
- Claude 测试支持勾选“模型后追加 1M 上下文 [1m]”。
- 实时显示日志和测试结果。

第一版不包含：

- API Key 加密存储。
- 多端点并发测试。
- 测试历史数据库。
- 自定义测试 prompt。
- 端点分组、搜索和复杂过滤。
- 打包成可执行文件。

## 文件设计

新增文件：

```text
tsa_gui.py
```

运行依赖：

```bash
python3 -m pip install PyQt6
```

数据文件：

```text
tsa_endpoints.json
```

`tsa_endpoints.json` 位于当前工作目录。GUI 启动时读取该文件；保存、删除或更新端点后写回该文件。两个 CLI 脚本也默认读取该文件，并按 `type` 过滤出各自端点。

## 数据结构

`tsa_endpoints.json` 格式：

```json
{
  "version": 1,
  "endpoints": [
    {
      "id": "codex-20260702120000-001",
      "type": "codex",
      "base_url": "https://example.com/v1",
      "api_key": "sk-xxx",
      "models": [
        "gpt-5.5",
        "gpt-5.5-mini"
      ]
    }
  ]
}
```

字段说明：

- `id`：端点唯一 ID，用于列表操作和更新。
- `type`：端点类型，取值为 `codex` 或 `claude`。
- `base_url`：用户输入的端点 URL，保存前去除尾部 `/`。
- `api_key`：用户输入的 API Key，第一版明文保存。
- `models`：用户从拉取结果中勾选并保存的模型 ID 列表。

界面展示 API Key 时必须脱敏，例如：

```text
sk-1234...abcd
```

## 主界面布局

主窗口分为四个区域：端点输入区、模型选择区、已保存端点区、日志区。

```text
┌──────────────────────────────────────────────────────────────┐
│ TSA GUI                                                       │
├───────────────────────────────┬──────────────────────────────┤
│ 添加端点                       │ 已保存端点                   │
│                               │                              │
│ 类型: [ Codex ▼ ]             │ 类型    URL      SK   模型 操作│
│ URL:  [ https://...       ]    │ Codex   ...      ...   2  测试 │
│ SK:   [ sk-...            ]    │ Claude  ...      ...   1  测试 │
│                               │                              │
│ [拉取模型] [保存端点] [清空]   │ [测试] [删除] [刷新] [加载]    │
│                               │ [复制 URL] [复制 KEY]          │
├───────────────────────────────┴──────────────────────────────┤
│ 已拉取模型                                                     │
│ [ ] gpt-5.5                                                    │
│ [ ] gpt-5.5-mini                                               │
│ [ ] claude-opus-4-6                                            │
│ [全选] [全不选] [反选]                                          │
├──────────────────────────────────────────────────────────────┤
│ 日志                                                           │
│ 12:00:01 fetching models from https://example.com/v1            │
│ 12:00:02 fetched 23 models                                      │
└──────────────────────────────────────────────────────────────┘
```

推荐布局：

- 顶部左侧：添加端点表单。
- 顶部右侧：已保存端点列表。
- 中部：已拉取模型多选列表。
- 底部：日志输出框。

## 添加端点流程

1. 用户选择端点类型：`Codex` 或 `Claude`。
2. 用户填写 `URL`。
3. 用户填写 `SK`。
4. 用户点击“拉取模型”。
5. GUI 根据类型调用对应模型列表接口。
6. 成功后在“已拉取模型”区域展示模型复选框。
7. 用户勾选需要保存的模型。
8. 用户点击“保存端点”。
9. GUI 将端点写入 `tsa_endpoints.json`。
10. 已保存端点列表刷新。

## CLI 读取规则

`test_codex_models.py` 默认读取 `tsa_endpoints.json`，只使用 `type` 为 `codex` 的端点。

`test_claude_models.py` 默认读取 `tsa_endpoints.json`，只使用 `type` 为 `claude` 的端点。

两个脚本仍保留参数名：

- Codex：`--api-file PATH`
- Claude：`--claude-file PATH`

这两个参数现在只接受同结构的 JSON 文件，不再读取旧的 `codex.txt` / `claude.txt` 文本格式。

如果端点 JSON 中的 `models` 为空，CLI 会使用 `--models` 参数传入的模型列表。

Claude CLI 额外支持 `-1m` 参数。启用后，会在最终测试的模型 ID 后追加 `[1m]`，例如 `claude-opus-4-6[1m]`。追加发生在 `/models` 交集和 `--limit` 之后，不影响模型存在性检查。

已拉取模型区域提供：

- `全选`：勾选全部模型。
- `全不选`：取消勾选全部模型。
- `反选`：反转当前勾选状态。

## 模型拉取规则

### Codex

Codex 使用 OpenAI 兼容格式：

```text
GET {base_url}/models
Authorization: Bearer {api_key}
Accept: application/json
```

返回体要求包含：

```json
{
  "data": [
    {"id": "gpt-5.5"}
  ]
}
```

响应体支持普通 JSON、`gzip` 和 `deflate` 压缩。即使服务端漏写 `Content-Encoding: gzip`，只要响应体是 gzip 格式也会自动解压。

### Claude

Claude 使用 Anthropic 兼容格式。

如果 `base_url` 以 `/v1` 结尾：

```text
GET {base_url}/models
```

否则：

```text
GET {base_url}/v1/models
```

请求头：

```text
Authorization: Bearer {api_key}
X-Api-Key: {api_key}
Anthropic-Version: 2023-06-01
Accept: application/json
```

返回体支持两种格式：

```json
{
  "data": [
    {"id": "claude-opus-4-6"}
  ]
}
```

或：

```json
{
  "models": [
    "claude-opus-4-6"
  ]
}
```

响应体支持普通 JSON、`gzip` 和 `deflate` 压缩。即使服务端漏写 `Content-Encoding: gzip`，只要响应体是 gzip 格式也会自动解压。

## 已保存端点列表

列表字段：

- 类型：`Codex` 或 `Claude`。
- URL：端点 URL。
- SK：脱敏 API Key。
- 模型数：已保存模型数量。
- 操作：`测试`、`删除`、`刷新`、`加载`、`复制 URL`、`复制 KEY`。

第一版必须实现：

- `测试`
- `删除`
- `刷新`
- `加载`
- `复制 URL`
- `复制 KEY`

第一版可以暂缓：

- `编辑`

删除端点前需要二次确认。

## 测试弹窗

点击端点行的“测试”按钮后，打开独立弹窗。

```text
┌──────────────────────────────────────────────┐
│ 测试端点                                      │
├──────────────────────────────────────────────┤
│ 类型: Codex                                   │
│ URL:  https://example.com/v1        [复制 URL] │
│ SK:   sk-1234...abcd                [复制 SK]  │
├──────────────────────────────────────────────┤
│ 选择模型                                      │
│ [x] gpt-5.5                                   │
│ [ ] gpt-5.5-mini                              │
│ [ ] gpt-4.1                                   │
│ [全选] [全不选] [反选]                         │
├──────────────────────────────────────────────┤
│ 超时时间: [120] 秒                            │
│ Claude: [ ] 模型后追加 1M 上下文 [1m]          │
│ [开始测试] [停止] [关闭]                      │
├──────────────────────────────────────────────┤
│ 结果                                          │
│ 模型              状态          耗时           │
│ gpt-5.5           AVAILABLE     8.4s           │
│ gpt-5.5-mini      UNAVAILABLE   timeout        │
├──────────────────────────────────────────────┤
│ 日志                                          │
│ running codex exec ...                         │
└──────────────────────────────────────────────┘
```

测试弹窗行为：

- 默认勾选该端点保存的全部模型。
- 用户可以取消部分模型。
- 模型列表支持“全选 / 全不选 / 反选”。
- 点击“开始测试”后不禁用模型选择框；当前测试批次使用点击开始时选中的模型。
- Claude 端点可勾选“模型后追加 1M 上下文 [1m]”，勾选后本次测试使用 `模型ID[1m]`，不修改保存数据。
- URL 和 SK 提供复制按钮。界面显示 SK 时脱敏，复制 SK 时复制完整明文值。
- 测试过程在后台线程运行，避免卡住 GUI。
- 每个模型测完后立即更新结果表。
- 点击“停止”后终止当前测试进程，并停止后续模型测试。
- 测试结束后恢复按钮状态。

## 测活规则

测活 prompt 固定为现有脚本使用的 prompt：

```text
You must output exactly OKK and nothing else. Do not explain. Do not add punctuation.
```

如果 CLI 退出码为 0，且输出中存在单独一行 `OKK`，则判定为可用。

状态定义：

- `AVAILABLE`：CLI 正常退出，并返回预期输出。
- `UNAVAILABLE`：CLI 失败、超时、输出不符合预期或进程异常。
- `STOPPED`：用户手动停止。

## Codex 测试实现

Codex 测试复用现有脚本逻辑：

- 写入临时 Codex 配置：
  - `~/.codex/auth.json`
  - `~/.codex/config.toml`
- 执行：

```text
codex exec --skip-git-repo-check "{prompt}"
```

- 测试结束后恢复原配置文件。

GUI 启动测试前需要检查：

```text
codex
```

是否存在于 `PATH`。

## Claude 测试实现

Claude 测试复用现有脚本逻辑：

- 写入临时设置文件：
  - `claude-settings.json`
- 执行：

```text
claude --debug --verbose --settings claude-settings.json --model {model} -p "{prompt}"
```

- 测试结束后恢复原 `claude-settings.json`。

GUI 启动测试前需要检查：

```text
claude
```

是否存在于 `PATH`。

## 线程模型

GUI 主线程只负责界面更新。

耗时任务放入后台线程：

- 拉取模型。
- 执行模型测试。

后台线程不能直接更新 Qt 控件。后台线程通过 `queue.Queue` 发送事件给主线程，主线程使用 `QTimer` 定时消费事件。

事件类型建议：

```text
log
models_fetched
fetch_failed
test_started
test_result
test_finished
test_failed
```

## 错误处理

需要明确提示的错误：

- URL 为空。
- API Key 为空。
- 未选择任何模型。
- 模型接口请求失败。
- 模型接口返回格式不符合预期。
- `codex` 命令不存在。
- `claude` 命令不存在。
- JSON 文件读取失败。
- JSON 文件写入失败。

JSON 文件读取失败时，GUI 应弹窗提示，并允许用户选择：

- 退出。
- 忽略损坏文件并创建新的空数据。

## 安全说明

第一版会把 API Key 明文写入当前目录的 `tsa_endpoints.json`。

要求：

- `tsa_endpoints.json` 必须加入 `.gitignore`。
- GUI 列表中不显示完整 API Key。
- 日志中默认不打印完整 API Key。

## 实现步骤

1. 新增 `tsa_gui.py`。
2. 实现 `EndpointStore`，负责读写 `tsa_endpoints.json`。
3. 实现主窗口布局。
4. 实现模型拉取。
5. 实现模型复选列表。
6. 实现保存端点和刷新列表。
7. 实现删除端点。
8. 实现测试弹窗。
9. 接入 Codex 测试逻辑。
10. 接入 Claude 测试逻辑。
11. 加入后台线程和队列事件。
12. 运行语法检查：

```bash
python3 -m py_compile tsa_gui.py
```

## 后续增强

后续版本可以考虑：

- 编辑已有端点。
- 重新拉取某个端点的模型并更新。
- 导出可用模型配置。
- 保存测试历史。
- 端点搜索和标签。
- API Key 加密存储。
- 自定义测试 prompt。
- 并发测试多个模型。
- 支持打包为桌面应用。
