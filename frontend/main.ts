import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("#app not found");
}

let endpoints: SavedEndpoint[] = [];
let fetchedModels: string[] = [];
let fetchedSelection = new Set<string>();
let selectedEndpointId = "";
let testEndpoint: SavedEndpoint | null = null;
let testSelection = new Set<string>();
let testResults: TestResult[] = [];
let testRunning = false;
let logs: string[] = [];

app.innerHTML = `
  <main class="shell">
    <section class="hero">
      <div>
        <p class="eyebrow">TSA Tauri</p>
        <h1>端点管理与模型测活</h1>
        <p>管理 Codex / Claude API 端点，拉取模型，调用本机 CLI 验证模型可用性。</p>
      </div>
      <div class="badge">Rust + TypeScript + Tauri</div>
    </section>

    <section class="grid">
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
              <tr><th>类型</th><th>URL</th><th>SK</th><th>模型数</th></tr>
            </thead>
            <tbody id="endpoint-rows"></tbody>
          </table>
        </div>
        <div class="actions wrap">
          <button id="open-test">测试</button>
          <button id="delete-endpoint" class="danger">删除</button>
          <button id="load-endpoint" class="secondary">加载</button>
          <button id="copy-url" class="secondary">复制 URL</button>
          <button id="copy-key" class="secondary">复制 KEY</button>
        </div>
      </div>
    </section>

    <section class="card models-card">
      <div class="card-title">
        <h2>已拉取模型</h2>
        <div class="actions compact">
          <button id="models-all" class="secondary">全选</button>
          <button id="models-none" class="secondary">全不选</button>
          <button id="models-invert" class="secondary">反选</button>
        </div>
      </div>
      <div id="fetched-models" class="check-list empty">暂无模型</div>
    </section>

    <section id="test-panel" class="card test-card hidden">
      <div class="card-title">
        <div>
          <h2>测试端点</h2>
          <p id="test-meta"></p>
        </div>
        <button id="close-test" class="secondary">关闭</button>
      </div>
      <div class="test-layout">
        <div>
          <div class="test-controls">
            <label>超时时间
              <input id="test-timeout" type="number" min="1" max="3600" value="120" />
            </label>
            <label id="append-1m-label" class="inline-check">
              <input id="append-1m" type="checkbox" />
              模型后追加 1M 上下文 [1m]
            </label>
          </div>
          <div class="actions compact">
            <button id="test-all" class="secondary">全选</button>
            <button id="test-none" class="secondary">全不选</button>
            <button id="test-invert" class="secondary">反选</button>
          </div>
          <div id="test-models" class="check-list test-models"></div>
        </div>
        <div>
          <div class="actions">
            <button id="start-test">开始测试</button>
            <button id="stop-test" class="danger" disabled>停止</button>
          </div>
          <div class="table-wrap results">
            <table>
              <thead><tr><th>模型</th><th>状态</th><th>耗时</th></tr></thead>
              <tbody id="result-rows"></tbody>
            </table>
          </div>
        </div>
      </div>
    </section>

    <section class="card log-card">
      <div class="card-title">
        <h2>日志</h2>
        <button id="clear-log" class="secondary">清空日志</button>
      </div>
      <pre id="log-output"></pre>
    </section>
  </main>
`;

const endpointType = byId<HTMLSelectElement>("endpoint-type");
const baseUrl = byId<HTMLInputElement>("base-url");
const apiKey = byId<HTMLInputElement>("api-key");
const fetchTimeout = byId<HTMLInputElement>("fetch-timeout");
const endpointRows = byId<HTMLTableSectionElement>("endpoint-rows");
const fetchedModelsEl = byId<HTMLDivElement>("fetched-models");
const logOutput = byId<HTMLPreElement>("log-output");
const testPanel = byId<HTMLElement>("test-panel");
const testMeta = byId<HTMLParagraphElement>("test-meta");
const append1mLabel = byId<HTMLLabelElement>("append-1m-label");
const append1m = byId<HTMLInputElement>("append-1m");
const testModelsEl = byId<HTMLDivElement>("test-models");
const testTimeout = byId<HTMLInputElement>("test-timeout");
const resultRows = byId<HTMLTableSectionElement>("result-rows");
const startTest = byId<HTMLButtonElement>("start-test");
const stopTest = byId<HTMLButtonElement>("stop-test");

bind("fetch-models", "click", fetchModels);
bind("save-endpoint", "click", saveEndpoint);
bind("clear-input", "click", clearInput);
bind("reload-endpoints", "click", loadEndpoints);
bind("open-test", "click", openTestPanel);
bind("delete-endpoint", "click", deleteSelectedEndpoint);
bind("load-endpoint", "click", loadSelectedEndpointToForm);
bind("copy-url", "click", () => copyFromSelected("URL", (endpoint) => endpoint.base_url));
bind("copy-key", "click", () => copyFromSelected("KEY", (endpoint) => endpoint.api_key));
bind("models-all", "click", () => setSelection(fetchedSelection, fetchedModels, true, renderFetchedModels));
bind("models-none", "click", () => setSelection(fetchedSelection, fetchedModels, false, renderFetchedModels));
bind("models-invert", "click", () => invertSelection(fetchedSelection, fetchedModels, renderFetchedModels));
bind("close-test", "click", closeTestPanel);
bind("test-all", "click", () => setSelection(testSelection, testEndpoint?.models ?? [], true, renderTestModels));
bind("test-none", "click", () => setSelection(testSelection, testEndpoint?.models ?? [], false, renderTestModels));
bind("test-invert", "click", () => invertSelection(testSelection, testEndpoint?.models ?? [], renderTestModels));
bind("start-test", "click", runTests);
bind("stop-test", "click", stopTests);
bind("clear-log", "click", () => {
  logs = [];
  renderLogs();
});

void listen<{ message: string }>("test-log", (event) => log(event.payload.message));
void listen<{ result: TestResult }>("test-result", (event) => {
  testResults.push(event.payload.result);
  renderResults();
});
void listen("test-finished", () => {
  testRunning = false;
  startTest.disabled = false;
  stopTest.disabled = true;
});

void loadEndpoints();

async function loadEndpoints() {
  try {
    endpoints = await invoke<SavedEndpoint[]>("load_endpoints");
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
  append1m.checked = false;
  testPanel.classList.remove("hidden");
  testMeta.textContent = `${label(endpoint.type)} · ${endpoint.base_url} · ${maskKey(endpoint.api_key)}`;
  append1mLabel.classList.toggle("hidden", endpoint.type !== "claude");
  renderTestModels();
  renderResults();
}

function closeTestPanel() {
  if (testRunning && !confirm("测试仍在运行，是否停止并关闭？")) return;
  if (testRunning) void stopTests();
  testPanel.classList.add("hidden");
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
  try {
    await invoke("test_models", {
      request: {
        endpoint: testEndpoint,
        models,
        timeout: Number(testTimeout.value || 120),
        append_1m: append1m.checked,
      },
    });
  } catch (error) {
    testRunning = false;
    startTest.disabled = false;
    stopTest.disabled = true;
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

function renderEndpoints() {
  endpointRows.innerHTML = "";
  for (const endpoint of endpoints) {
    const row = document.createElement("tr");
    row.dataset.id = endpoint.id;
    row.classList.toggle("selected", endpoint.id === selectedEndpointId);
    row.innerHTML = `
      <td>${label(endpoint.type)}</td>
      <td title="${escapeAttr(endpoint.base_url)}">${escapeHtml(endpoint.base_url)}</td>
      <td>${escapeHtml(maskKey(endpoint.api_key))}</td>
      <td>${endpoint.models.length}</td>
    `;
    row.addEventListener("click", () => {
      selectedEndpointId = endpoint.id;
      renderEndpoints();
    });
    row.addEventListener("dblclick", openTestPanel);
    endpointRows.append(row);
  }
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
    checkbox.id = `${prefix}-${model}`;
    checkbox.addEventListener("change", () => {
      if (checkbox.checked) selection.add(model);
      else selection.delete(model);
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

function renderLogs() {
  logOutput.textContent = logs.join("\n");
  logOutput.scrollTop = logOutput.scrollHeight;
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
  logs.push(`${stamp} ${message}`);
  renderLogs();
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
