import { Channel, invoke } from "@tauri-apps/api/core";
import "./styles.css";

type EndpointType = "codex" | "claude";

interface SavedEndpoint {
  id: string;
  type: EndpointType;
  base_url: string;
  api_key: string;
  models: string[];
}

interface TestResult {
  model: string;
  status: string;
  seconds: number;
  detail: string;
}

interface TestMessage {
  kind: "log" | "result" | "finished";
  message?: string;
  stream?: boolean;
  result?: TestResult;
}

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("#app not found");
}

let endpoints: SavedEndpoint[] = [];
let fetchedModels: string[] = [];
let fetchedSelection = new Set<string>();
let selectedEndpointId = "";
let checkedEndpointIds = new Set<string>();
let testEndpoint: SavedEndpoint | null = null;
let testSelection = new Set<string>();
let testResults: TestResult[] = [];
let testRunning = false;
let testLogChunks: string[] = [];
let testPrompt = "You must output exactly OKK and nothing else. Do not explain. Do not add punctuation.";
let successKeyword = "OKK";

app.innerHTML = `
  <main class="shell">
    <header class="app-bar">
      <h1>TestModelAlive</h1>
      <span>端点管理 / CLI 测试</span>
    </header>

    <section class="workspace">
      <div class="card form-card">
        <div class="card-title">
          <h2>添加端点</h2>
          <button id="clear-input" class="secondary">清空</button>
        </div>
        <label>类型
          <select id="endpoint-type">
            <option value="codex">codex</option>
            <option value="claude">claude</option>
          </select>
        </label>
        <label>URL
          <input id="base-url" placeholder="https://example.com/v1" />
        </label>
        <label>SK
          <input id="api-key" type="password" placeholder="sk-..." />
        </label>
        <label>拉取超时
          <input id="fetch-timeout" type="number" min="1" max="3600" value="30" />
        </label>
        <div class="actions">
          <button id="fetch-models">拉取模型</button>
          <button id="save-endpoint">保存端点</button>
        </div>
      </div>

      <div class="card table-card">
        <div class="card-title">
          <h2>已保存端点</h2>
          <button id="reload-endpoints" class="secondary">刷新</button>
        </div>
        <div class="table-wrap">
          <table>
            <thead>
              <tr><th class="check-column">选</th><th>类型</th><th>URL</th><th>SK</th><th>模型数</th></tr>
            </thead>
            <tbody id="endpoint-rows"></tbody>
          </table>
        </div>
        <div class="actions wrap">
          <button id="open-test">测试</button>
          <button id="delete-endpoint" class="danger">删除</button>
          <button id="delete-checked" class="danger">批量删除</button>
          <button id="load-endpoint" class="secondary">加载</button>
          <button id="copy-url" class="secondary">复制 URL</button>
          <button id="copy-key" class="secondary">复制 KEY</button>
          <button id="check-endpoints-all" class="secondary">全选</button>
          <button id="check-endpoints-none" class="secondary">全不选</button>
        </div>
      </div>

      <div class="card models-card">
        <div class="card-title">
          <h2>已拉取模型</h2>
          <div class="actions compact">
            <button id="models-all" class="secondary">全选</button>
            <button id="models-none" class="secondary">全不选</button>
            <button id="models-invert" class="secondary">反选</button>
          </div>
        </div>
        <div id="fetched-models" class="check-list empty">暂无模型</div>
      </div>
    </section>

    <section id="test-panel" class="test-modal hidden" aria-modal="true" role="dialog">
      <div class="test-dialog">
        <div class="modal-title">
          <div>
            <h2>测试模型</h2>
          </div>
          <button id="close-test" class="secondary">关闭</button>
        </div>
        <div class="test-endpoint-box">
          <div><span>类型</span><strong id="test-type"></strong></div>
          <div><span>URL</span><strong id="test-url"></strong><button id="test-copy-url" class="secondary">复制 URL</button></div>
          <div><span>SK</span><strong id="test-key"></strong><button id="test-copy-key" class="secondary">复制 SK</button></div>
        </div>
        <div class="test-controls-bar">
          <label>超时时间
            <input id="test-timeout" type="number" min="1" max="3600" value="120" />
          </label>
          <label id="append-1m-label" class="inline-check">
            <input id="append-1m" type="checkbox" />
            模型后追加 1M 上下文 [1m]
          </label>
          <button id="start-test">开始测试</button>
          <button id="stop-test" class="danger" disabled>停止</button>
          <button id="open-test-settings" class="secondary">测试设置</button>
          <span id="test-status" class="test-status">未开始</span>
        </div>
        <div class="test-layout">
          <div class="test-box test-left">
            <h3>选择模型</h3>
            <div class="actions compact">
              <button id="test-all" class="secondary">全选</button>
              <button id="test-none" class="secondary">全不选</button>
              <button id="test-invert" class="secondary">反选</button>
            </div>
            <div id="test-models" class="check-list test-models"></div>
          </div>
          <div class="test-box test-right">
            <h3>结果</h3>
            <div class="table-wrap results">
              <table>
                <thead><tr><th>模型</th><th>状态</th><th>耗时</th></tr></thead>
                <tbody id="result-rows"></tbody>
              </table>
            </div>
          </div>
        </div>
        <div class="test-box test-log-box">
          <div class="test-log-title">
            <h3>日志</h3>
            <div class="actions compact">
              <button id="copy-test-log" class="secondary">复制</button>
              <button id="clear-test-log" class="secondary">清空</button>
            </div>
          </div>
          <pre id="test-log-output"></pre>
        </div>
      </div>
    </section>

    <section id="test-settings-panel" class="settings-modal hidden" aria-modal="true" role="dialog">
      <div class="settings-dialog">
        <div class="modal-title">
          <h2>测试设置</h2>
          <button id="close-test-settings" class="secondary">关闭</button>
        </div>
        <p class="settings-hint">成功关键词必须明确写进提示词里，要求模型输出它；测试会在命令输出中匹配这个关键词。</p>
        <label>匹配成功关键词
          <input id="success-keyword" placeholder="OKK" />
        </label>
        <label>测试提示词
          <textarea id="test-prompt" rows="6"></textarea>
        </label>
        <div class="actions">
          <button id="save-test-settings">保存设置</button>
          <button id="reset-test-settings" class="secondary">恢复默认</button>
        </div>
      </div>
    </section>
  </main>
`;

const endpointType = byId<HTMLSelectElement>("endpoint-type");
const baseUrl = byId<HTMLInputElement>("base-url");
const apiKey = byId<HTMLInputElement>("api-key");
const fetchTimeout = byId<HTMLInputElement>("fetch-timeout");
const endpointRows = byId<HTMLTableSectionElement>("endpoint-rows");
const fetchedModelsEl = byId<HTMLDivElement>("fetched-models");
const testPanel = byId<HTMLElement>("test-panel");
const testType = byId<HTMLElement>("test-type");
const testUrl = byId<HTMLElement>("test-url");
const testKey = byId<HTMLElement>("test-key");
const append1mLabel = byId<HTMLLabelElement>("append-1m-label");
const append1m = byId<HTMLInputElement>("append-1m");
const testModelsEl = byId<HTMLDivElement>("test-models");
const testTimeout = byId<HTMLInputElement>("test-timeout");
const resultRows = byId<HTMLTableSectionElement>("result-rows");
const startTest = byId<HTMLButtonElement>("start-test");
const stopTest = byId<HTMLButtonElement>("stop-test");
const testStatus = byId<HTMLSpanElement>("test-status");
const testLogOutput = byId<HTMLPreElement>("test-log-output");
const testSettingsPanel = byId<HTMLElement>("test-settings-panel");
const successKeywordInput = byId<HTMLInputElement>("success-keyword");
const testPromptInput = byId<HTMLTextAreaElement>("test-prompt");

bind("fetch-models", "click", fetchModels);
bind("save-endpoint", "click", saveEndpoint);
bind("clear-input", "click", clearInput);
bind("reload-endpoints", "click", loadEndpoints);
bind("open-test", "click", openTestPanel);
bind("delete-endpoint", "click", deleteSelectedEndpoint);
bind("delete-checked", "click", deleteCheckedEndpoints);
bind("load-endpoint", "click", loadSelectedEndpointToForm);
bind("copy-url", "click", () => copyFromSelected("URL", (endpoint) => endpoint.base_url));
bind("copy-key", "click", () => copyFromSelected("KEY", (endpoint) => endpoint.api_key));
bind("check-endpoints-all", "click", () => setEndpointChecks(true));
bind("check-endpoints-none", "click", () => setEndpointChecks(false));
bind("models-all", "click", () => setSelection(fetchedSelection, fetchedModels, true, renderFetchedModels));
bind("models-none", "click", () => setSelection(fetchedSelection, fetchedModels, false, renderFetchedModels));
bind("models-invert", "click", () => invertSelection(fetchedSelection, fetchedModels, renderFetchedModels));
bind("close-test", "click", closeTestPanel);
bind("test-copy-url", "click", () => copyFromTest("URL", (endpoint) => endpoint.base_url));
bind("test-copy-key", "click", () => copyFromTest("SK", (endpoint) => endpoint.api_key));
bind("test-all", "click", () => setSelection(testSelection, testEndpoint?.models ?? [], true, renderTestModels));
bind("test-none", "click", () => setSelection(testSelection, testEndpoint?.models ?? [], false, renderTestModels));
bind("test-invert", "click", () => invertSelection(testSelection, testEndpoint?.models ?? [], renderTestModels));
bind("start-test", "click", runTests);
bind("stop-test", "click", stopTests);
bind("open-test-settings", "click", openTestSettings);
bind("close-test-settings", "click", closeTestSettings);
bind("save-test-settings", "click", saveTestSettings);
bind("reset-test-settings", "click", resetTestSettings);
bind("copy-test-log", "click", copyTestLog);
bind("clear-test-log", "click", () => {
  testLogChunks = [];
  renderTestLogs();
});

void loadEndpoints();

async function loadEndpoints() {
  try {
    endpoints = await invoke<SavedEndpoint[]>("load_endpoints");
    checkedEndpointIds = new Set([...checkedEndpointIds].filter((id) => endpoints.some((endpoint) => endpoint.id === id)));
    renderEndpoints();
  } catch (error) {
    alertError("读取端点失败", error);
  }
}

async function fetchModels() {
  const request = formRequest();
  if (!request) return;
  log(`fetching models: type=${request.type} url=${request.base_url}`);
  setBusy("fetch-models", true);
  try {
    fetchedModels = await invoke<string[]>("fetch_models", { request });
    fetchedSelection = new Set(fetchedModels);
    renderFetchedModels();
    log(`fetched ${fetchedModels.length} models`);
  } catch (error) {
    alertError("拉取模型失败", error);
    log(`fetch failed: ${String(error)}`);
  } finally {
    setBusy("fetch-models", false);
  }
}

async function saveEndpoint() {
  const request = formRequest();
  if (!request) return;
  const models = fetchedModels.filter((model) => fetchedSelection.has(model));
  if (models.length === 0) {
    alert("请先拉取模型并选择至少一个模型。");
    return;
  }
  try {
    await invoke("add_endpoint", { request: { ...request, models } });
    log(`saved endpoint: type=${request.type} url=${request.base_url} models=${models.length}`);
    await loadEndpoints();
  } catch (error) {
    alertError("保存失败", error);
  }
}

async function deleteSelectedEndpoint() {
  const endpoint = selectedEndpoint();
  if (!endpoint) return;
  if (!confirm(`确定删除端点？\n${endpoint.base_url}`)) return;
  try {
    await invoke("delete_endpoint", { endpointId: endpoint.id });
    log(`deleted endpoint: ${endpoint.base_url}`);
    selectedEndpointId = "";
    await loadEndpoints();
  } catch (error) {
    alertError("删除失败", error);
  }
}

async function deleteCheckedEndpoints() {
  const selected = endpoints.filter((endpoint) => checkedEndpointIds.has(endpoint.id));
  if (selected.length === 0) {
    alert("请先勾选要删除的端点。");
    return;
  }
  if (!confirm(`确定删除已勾选的 ${selected.length} 个端点？`)) return;
  try {
    for (const endpoint of selected) {
      await invoke("delete_endpoint", { endpointId: endpoint.id });
      log(`deleted endpoint: ${endpoint.base_url}`);
    }
    checkedEndpointIds.clear();
    if (selected.some((endpoint) => endpoint.id === selectedEndpointId)) selectedEndpointId = "";
    await loadEndpoints();
  } catch (error) {
    alertError("批量删除失败", error);
  }
}

function loadSelectedEndpointToForm() {
  const endpoint = selectedEndpoint();
  if (!endpoint) return;
  endpointType.value = endpoint.type;
  baseUrl.value = endpoint.base_url;
  apiKey.value = endpoint.api_key;
  fetchedModels = [...endpoint.models];
  fetchedSelection = new Set(fetchedModels);
  renderFetchedModels();
  log(`loaded endpoint into form: ${endpoint.base_url}`);
}

function openTestPanel() {
  const endpoint = selectedEndpoint();
  if (!endpoint) return;
  testEndpoint = endpoint;
  testSelection = new Set(endpoint.models);
  testResults = [];
  testLogChunks = [];
  append1m.checked = false;
  testPanel.classList.remove("hidden");
  document.body.classList.add("modal-open");
  testType.textContent = label(endpoint.type);
  testUrl.textContent = endpoint.base_url;
  testKey.textContent = maskKey(endpoint.api_key);
  testStatus.textContent = "未开始";
  append1mLabel.classList.toggle("hidden", endpoint.type !== "claude");
  renderTestModels();
  renderResults();
  renderTestLogs();
}

function closeTestPanel() {
  if (testRunning && !confirm("测试仍在运行，是否停止并关闭？")) return;
  if (testRunning) void stopTests();
  testPanel.classList.add("hidden");
  document.body.classList.remove("modal-open");
}

async function runTests() {
  if (!testEndpoint) return;
  const models = testEndpoint.models.filter((model) => testSelection.has(model));
  if (models.length === 0) {
    alert("请选择至少一个模型。");
    return;
  }
  testResults = [];
  renderResults();
  testRunning = true;
  startTest.disabled = true;
  stopTest.disabled = false;
  testStatus.textContent = `运行中：${models.length} 个模型`;
  log(`starting CLI test request: type=${testEndpoint.type} url=${testEndpoint.base_url} models=${models.length} timeout=${Number(testTimeout.value || 120)}s`);
  const onEvent = new Channel<TestMessage>((message) => {
    if (message.kind === "log" && message.message !== undefined) {
      if (message.stream) appendStreamLog(message.message);
      else log(message.message);
    } else if (message.kind === "result" && message.result) {
      testResults.push(message.result);
      renderResults();
    } else if (message.kind === "finished") {
      testRunning = false;
      startTest.disabled = false;
      stopTest.disabled = true;
      testStatus.textContent = "已结束";
    }
  });
  try {
    await invoke("test_models", {
      request: {
        endpoint: testEndpoint,
        models,
        timeout: Number(testTimeout.value || 120),
        append_1m: append1m.checked,
        prompt: testPrompt,
        success_keyword: successKeyword,
      },
      onEvent,
    });
  } catch (error) {
    testRunning = false;
    startTest.disabled = false;
    stopTest.disabled = true;
    testStatus.textContent = "启动失败";
    alertError("启动测试失败", error);
  }
}

async function stopTests() {
  try {
    await invoke("stop_test");
    log("stopping test...");
  } catch (error) {
    alertError("停止失败", error);
  }
}

function openTestSettings() {
  successKeywordInput.value = successKeyword;
  testPromptInput.value = testPrompt;
  testSettingsPanel.classList.remove("hidden");
}

function closeTestSettings() {
  testSettingsPanel.classList.add("hidden");
}

function saveTestSettings() {
  const keyword = successKeywordInput.value.trim();
  const prompt = testPromptInput.value.trim();
  if (!keyword) {
    alert("请填写匹配成功关键词。");
    return;
  }
  if (!prompt) {
    alert("请填写测试提示词。");
    return;
  }
  if (!prompt.includes(keyword)) {
    alert("测试提示词必须包含匹配成功关键词，并要求模型输出它。");
    return;
  }
  successKeyword = keyword;
  testPrompt = prompt;
  testLog(`saved test settings: success keyword=${successKeyword}`);
  closeTestSettings();
}

function resetTestSettings() {
  successKeywordInput.value = "OKK";
  testPromptInput.value = "You must output exactly OKK and nothing else. Do not explain. Do not add punctuation.";
}

function renderEndpoints() {
  endpointRows.innerHTML = "";
  for (const endpoint of endpoints) {
    const row = document.createElement("tr");
    row.dataset.id = endpoint.id;
    row.classList.toggle("selected", endpoint.id === selectedEndpointId);
    row.innerHTML = `
      <td class="check-column"></td>
      <td>${label(endpoint.type)}</td>
      <td title="${escapeAttr(endpoint.base_url)}">${escapeHtml(endpoint.base_url)}</td>
      <td>${escapeHtml(maskKey(endpoint.api_key))}</td>
      <td>${endpoint.models.length}</td>
    `;
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.checked = checkedEndpointIds.has(endpoint.id);
    const checkCell = row.querySelector<HTMLTableCellElement>("td");
    checkCell?.addEventListener("click", (event) => {
      event.stopPropagation();
      if (event.target !== checkbox) {
        checkbox.checked = !checkbox.checked;
        checkbox.dispatchEvent(new Event("change"));
      }
    });
    checkCell?.addEventListener("dblclick", (event) => event.stopPropagation());
    checkbox.addEventListener("click", (event) => event.stopPropagation());
    checkbox.addEventListener("change", () => {
      if (checkbox.checked) checkedEndpointIds.add(endpoint.id);
      else checkedEndpointIds.delete(endpoint.id);
    });
    checkCell?.append(checkbox);
    row.addEventListener("click", () => {
      selectedEndpointId = endpoint.id;
      renderEndpoints();
    });
    row.addEventListener("dblclick", openTestPanel);
    endpointRows.append(row);
  }
}

function setEndpointChecks(checked: boolean) {
  checkedEndpointIds.clear();
  if (checked) {
    for (const endpoint of endpoints) checkedEndpointIds.add(endpoint.id);
  }
  renderEndpoints();
}

function renderFetchedModels() {
  renderCheckList(fetchedModelsEl, fetchedModels, fetchedSelection, "fetched");
}

function renderTestModels() {
  renderCheckList(testModelsEl, testEndpoint?.models ?? [], testSelection, "test");
}

function renderCheckList(root: HTMLElement, models: string[], selection: Set<string>, prefix: string) {
  root.innerHTML = "";
  root.classList.toggle("empty", models.length === 0);
  if (models.length === 0) {
    root.textContent = "暂无模型";
    return;
  }
  for (const model of models) {
    const item = document.createElement("label");
    item.className = "model-check";
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.checked = selection.has(model);
    item.classList.toggle("checked", checkbox.checked);
    checkbox.id = `${prefix}-${model}`;
    checkbox.addEventListener("change", () => {
      if (checkbox.checked) selection.add(model);
      else selection.delete(model);
      item.classList.toggle("checked", checkbox.checked);
    });
    const text = document.createElement("span");
    text.textContent = model;
    item.append(checkbox, text);
    root.append(item);
  }
}

function renderResults() {
  resultRows.innerHTML = "";
  for (const result of testResults) {
    const row = document.createElement("tr");
    row.innerHTML = `
      <td>${escapeHtml(result.model)}</td>
      <td><span class="status ${result.status.toLowerCase()}">${escapeHtml(result.status)}</span></td>
      <td>${result.seconds.toFixed(1)}s</td>
    `;
    resultRows.append(row);
  }
}

function renderTestLogs() {
  testLogOutput.textContent = testLogChunks.join("");
  testLogOutput.scrollTop = testLogOutput.scrollHeight;
}

function formRequest() {
  const request = {
    type: endpointType.value as EndpointType,
    base_url: baseUrl.value.trim().replace(/\/+$/, ""),
    api_key: apiKey.value.trim(),
    timeout: Number(fetchTimeout.value || 30),
  };
  if (!request.base_url) {
    alert("请填写端点 URL。");
    return null;
  }
  if (!request.api_key) {
    alert("请填写 API Key。");
    return null;
  }
  return request;
}

function clearInput() {
  baseUrl.value = "";
  apiKey.value = "";
  fetchedModels = [];
  fetchedSelection.clear();
  renderFetchedModels();
}

function selectedEndpoint() {
  const endpoint = endpoints.find((item) => item.id === selectedEndpointId);
  if (!endpoint) alert("请先选择一个已保存端点。");
  return endpoint;
}

async function copyFromSelected(labelText: string, getter: (endpoint: SavedEndpoint) => string) {
  const endpoint = selectedEndpoint();
  if (!endpoint) return;
  await navigator.clipboard.writeText(getter(endpoint));
  log(`copied ${labelText}: ${labelText === "URL" ? endpoint.base_url : endpoint.base_url}`);
}

async function copyFromTest(labelText: string, getter: (endpoint: SavedEndpoint) => string) {
  if (!testEndpoint) return;
  await navigator.clipboard.writeText(getter(testEndpoint));
  testLog(`copied ${labelText}`);
}

async function copyTestLog() {
  await navigator.clipboard.writeText(testLogChunks.join(""));
  testLog("copied log");
}

function setSelection(selection: Set<string>, models: string[], checked: boolean, render: () => void) {
  selection.clear();
  if (checked) {
    for (const model of models) selection.add(model);
  }
  render();
}

function invertSelection(selection: Set<string>, models: string[], render: () => void) {
  for (const model of models) {
    if (selection.has(model)) selection.delete(model);
    else selection.add(model);
  }
  render();
}

function log(message: string) {
  const stamp = new Date().toLocaleTimeString("zh-CN", { hour12: false });
  const line = `${stamp} ${message}\n`;
  if (isTestPanelOpen()) testLogChunks.push(line);
  renderTestLogs();
}

function appendStreamLog(message: string) {
  if (isTestPanelOpen()) testLogChunks.push(message);
  renderTestLogs();
}

function testLog(message: string) {
  const stamp = new Date().toLocaleTimeString("zh-CN", { hour12: false });
  testLogChunks.push(`${stamp} ${message}\n`);
  renderTestLogs();
}

function isTestPanelOpen() {
  return !testPanel.classList.contains("hidden");
}

function label(type: EndpointType) {
  return type === "codex" ? "Codex" : "Claude";
}

function maskKey(value: string) {
  if (value.length <= 10) return "*".repeat(value.length);
  return `${value.slice(0, 7)}...${value.slice(-4)}`;
}

function byId<T extends HTMLElement>(id: string) {
  const element = document.getElementById(id);
  if (!element) throw new Error(`#${id} not found`);
  return element as T;
}

function bind(id: string, event: string, handler: EventListener) {
  byId(id).addEventListener(event, handler);
}

function setBusy(id: string, busy: boolean) {
  const button = byId<HTMLButtonElement>(id);
  button.disabled = busy;
}

function alertError(title: string, error: unknown) {
  alert(`${title}:\n${String(error)}`);
}

function escapeHtml(value: string) {
  return value.replace(/[&<>"]/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[char] ?? char);
}

function escapeAttr(value: string) {
  return escapeHtml(value).replace(/'/g, "&#39;");
}
