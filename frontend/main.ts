import { Channel, invoke } from "@tauri-apps/api/core";
import { type Language, translate } from "./i18n";
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

interface TestSettings {
  prompt: string;
  success_keyword: string;
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
let language: Language = (localStorage.getItem("language") === "en" ? "en" : "zh");
document.documentElement.lang = language === "zh" ? "zh-CN" : "en";

function t(key: string, values: Record<string, string | number> = {}) {
  return translate(language, key, values);
}

app.innerHTML = `
  <main class="shell">
    <header class="app-bar">
      <h1>TestModelAlive</h1>
      <span id="subtitle">${t("subtitle")}</span>
      <button id="language-toggle" class="secondary language-toggle">${language === "zh" ? "English" : "中文"}</button>
    </header>

    <section class="workspace">
      <div class="card form-card">
        <div class="card-title">
          <h2 id="add-endpoint-title">${t("addEndpoint")}</h2>
          <button id="clear-input" class="secondary">${t("clear")}</button>
        </div>
        <label><span id="endpoint-type-label">${t("type")}</span>
          <select id="endpoint-type">
            <option value="codex">codex</option>
            <option value="claude">claude</option>
          </select>
        </label>
        <label><span id="base-url-label">${t("url")}</span>
          <input id="base-url" placeholder="https://example.com/v1" />
        </label>
        <label><span id="api-key-label">${t("sk")}</span>
          <input id="api-key" type="password" placeholder="sk-..." />
        </label>
        <label><span id="fetch-timeout-label">${t("fetchTimeout")}</span>
          <input id="fetch-timeout" type="number" min="1" max="3600" value="30" />
        </label>
        <div class="actions">
          <button id="fetch-models">${t("fetchModels")}</button>
          <button id="save-endpoint">${t("saveEndpoint")}</button>
        </div>
      </div>

      <div class="card table-card">
        <div class="card-title">
          <h2 id="saved-endpoints-title">${t("savedEndpoints")}</h2>
          <button id="reload-endpoints" class="secondary">${t("refresh")}</button>
        </div>
        <div class="table-wrap">
          <table>
            <thead>
              <tr><th id="endpoint-check-header" class="check-column">${t("select")}</th><th id="endpoint-type-header">${t("type")}</th><th>URL</th><th id="endpoint-key-header">${t("sk")}</th><th id="endpoint-model-count-header">${t("modelCount")}</th></tr>
            </thead>
            <tbody id="endpoint-rows"></tbody>
          </table>
        </div>
        <div class="actions wrap">
          <button id="open-test">${t("test")}</button>
          <button id="delete-endpoint" class="danger">${t("delete")}</button>
          <button id="delete-checked" class="danger">${t("batchDelete")}</button>
          <button id="load-endpoint" class="secondary">${t("load")}</button>
          <button id="copy-url" class="secondary">${t("copyUrl")}</button>
          <button id="copy-key" class="secondary">${t("copyKey")}</button>
          <button id="check-endpoints-all" class="secondary">${t("selectAll")}</button>
          <button id="check-endpoints-none" class="secondary">${t("selectNone")}</button>
        </div>
      </div>

      <div class="card models-card">
        <div class="card-title">
          <h2 id="fetched-models-title">${t("fetchedModels")}</h2>
          <div class="actions compact">
            <button id="models-all" class="secondary">${t("selectAll")}</button>
            <button id="models-none" class="secondary">${t("selectNone")}</button>
            <button id="models-invert" class="secondary">${t("invert")}</button>
          </div>
        </div>
        <div id="fetched-models" class="check-list empty">${t("noModels")}</div>
      </div>
    </section>

    <section id="test-panel" class="test-modal hidden" aria-modal="true" role="dialog">
      <div class="test-dialog">
        <div class="modal-title">
          <div>
            <h2 id="test-models-title">${t("testModels")}</h2>
          </div>
          <button id="close-test" class="secondary">${t("close")}</button>
        </div>
        <div class="test-endpoint-box">
          <div><span id="test-type-label">${t("type")}</span><strong id="test-type"></strong></div>
          <div><span>URL</span><strong id="test-url"></strong><button id="test-copy-url" class="secondary">${t("copyUrl")}</button></div>
          <div><span>${t("sk")}</span><strong id="test-key"></strong><button id="test-copy-key" class="secondary">${t("copyKey")}</button></div>
        </div>
        <div class="test-controls-bar">
          <label><span id="test-timeout-label">${t("timeout")}</span>
            <input id="test-timeout" type="number" min="1" max="3600" value="120" />
          </label>
          <label id="append-1m-label" class="inline-check">
            <input id="append-1m" type="checkbox" />
            ${t("append1m")}
          </label>
          <button id="start-test">${t("startTest")}</button>
          <button id="stop-test" class="danger" disabled>${t("stop")}</button>
          <button id="open-test-settings" class="secondary">${t("testSettings")}</button>
          <span id="test-status" class="test-status">${t("notStarted")}</span>
        </div>
        <div class="test-layout">
          <div class="test-box test-left">
          <h3 id="choose-models-title">${t("chooseModels")}</h3>
            <div class="actions compact">
              <button id="test-all" class="secondary">${t("selectAll")}</button>
              <button id="test-none" class="secondary">${t("selectNone")}</button>
              <button id="test-invert" class="secondary">${t("invert")}</button>
            </div>
            <div id="test-models" class="check-list test-models"></div>
          </div>
          <div class="test-box test-right">
            <h3 id="results-title">${t("results")}</h3>
            <div class="table-wrap results">
              <table>
                <thead><tr><th id="result-model-header">${t("model")}</th><th id="result-status-header">${t("status")}</th><th id="result-elapsed-header">${t("elapsed")}</th></tr></thead>
                <tbody id="result-rows"></tbody>
              </table>
            </div>
          </div>
        </div>
        <div class="test-box test-log-box">
          <div class="test-log-title">
            <h3 id="log-title">${t("log")}</h3>
            <div class="actions compact">
              <button id="copy-test-log" class="secondary">${t("copy")}</button>
              <button id="clear-test-log" class="secondary">${t("clear")}</button>
            </div>
          </div>
          <pre id="test-log-output"></pre>
        </div>
      </div>
    </section>

    <section id="test-settings-panel" class="settings-modal hidden" aria-modal="true" role="dialog">
      <div class="settings-dialog">
        <div class="modal-title">
          <h2 id="settings-title">${t("testSettings")}</h2>
          <button id="close-test-settings" class="secondary">${t("close")}</button>
        </div>
        <p id="settings-hint" class="settings-hint">${t("successHint")}</p>
        <label><span id="success-keyword-label">${t("successKeyword")}</span>
          <input id="success-keyword" placeholder="OKK" />
        </label>
        <label><span id="test-prompt-label">${t("testPrompt")}</span>
          <textarea id="test-prompt" rows="6"></textarea>
        </label>
        <div class="actions">
          <button id="save-test-settings">${t("saveSettings")}</button>
          <button id="reset-test-settings" class="secondary">${t("resetDefault")}</button>
        </div>
      </div>
    </section>
  </main>
`;

const endpointType = byId<HTMLSelectElement>("endpoint-type");
const languageToggle = byId<HTMLButtonElement>("language-toggle");
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
bind("language-toggle", "click", () => {
  localStorage.setItem("language", language === "zh" ? "en" : "zh");
  window.location.reload();
});
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
testPanel.addEventListener("click", (event) => {
  if (event.target === testPanel) closeTestPanel();
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && !testPanel.classList.contains("hidden")) {
    closeTestPanel();
  }
});

void loadEndpoints();
void loadTestSettings();

async function loadEndpoints() {
  try {
    endpoints = await invoke<SavedEndpoint[]>("load_endpoints");
    checkedEndpointIds = new Set([...checkedEndpointIds].filter((id) => endpoints.some((endpoint) => endpoint.id === id)));
    renderEndpoints();
  } catch (error) {
    alertError(tr("readEndpointsFailed"), error);
  }
}

async function loadTestSettings() {
  try {
    const settings = await invoke<TestSettings>("load_test_settings");
    testPrompt = settings.prompt;
    successKeyword = settings.success_keyword;
  } catch (error) {
    alertError(tr("readSettingsFailed"), error);
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
    alertError(tr("fetchFailed"), error);
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
    alert(tr("selectAtLeastOneFetchedModel"));
    return;
  }
  const duplicate = endpoints.find((endpoint) => endpoint.type === request.type && endpoint.base_url === request.base_url);
  let overwrite = false;
  if (duplicate) {
    const action = await confirmDuplicateEndpointAction(request.base_url);
    if (action === "cancel") return;
    overwrite = action === "overwrite";
  }
  try {
    const savedEndpoint = await invoke<SavedEndpoint>("add_endpoint", { request: { ...request, models, overwrite } });
    selectedEndpointId = savedEndpoint.id;
    log(`${overwrite ? "overwrote" : "saved"} endpoint: type=${request.type} url=${request.base_url} models=${models.length}`);
    await loadEndpoints();
  } catch (error) {
    alertError(tr("saveFailed"), error);
  }
}

async function deleteSelectedEndpoint() {
  const endpoint = selectedEndpoint();
  if (!endpoint) return;
  if (!confirm(`${tr("confirmDeleteEndpoint")}\n${endpoint.base_url}`)) return;
  try {
    await invoke("delete_endpoint", { endpointId: endpoint.id });
    log(`deleted endpoint: ${endpoint.base_url}`);
    selectedEndpointId = "";
    await loadEndpoints();
  } catch (error) {
    alertError(tr("deleteFailed"), error);
  }
}

async function deleteCheckedEndpoints() {
  const selected = endpoints.filter((endpoint) => checkedEndpointIds.has(endpoint.id));
  if (selected.length === 0) {
    alert(tr("checkEndpointsFirst"));
    return;
  }
  if (!confirm(tr("confirmDeleteChecked", { count: selected.length }))) return;
  try {
    for (const endpoint of selected) {
      await invoke("delete_endpoint", { endpointId: endpoint.id });
      log(`deleted endpoint: ${endpoint.base_url}`);
    }
    checkedEndpointIds.clear();
    if (selected.some((endpoint) => endpoint.id === selectedEndpointId)) selectedEndpointId = "";
    await loadEndpoints();
  } catch (error) {
    alertError(tr("batchDeleteFailed"), error);
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
  testStatus.textContent = tr("notStarted");
  append1mLabel.classList.toggle("hidden", endpoint.type !== "claude");
  renderTestModels();
  renderResults();
  renderTestLogs();
}

function closeTestPanel() {
  if (testRunning && !confirm(tr("testStillRunning"))) return;
  if (testRunning) void stopTests();
  testPanel.classList.add("hidden");
  document.body.classList.remove("modal-open");
}

async function runTests() {
  if (!testEndpoint) return;
  const models = testEndpoint.models.filter((model) => testSelection.has(model));
  if (models.length === 0) {
    alert(tr("selectAtLeastOneModel"));
    return;
  }
  testResults = [];
  renderResults();
  testRunning = true;
  startTest.disabled = true;
  stopTest.disabled = false;
  testStatus.textContent = `${tr("running")}: ${models.length}`;
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
      testStatus.textContent = tr("ended");
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
    testStatus.textContent = tr("launchFailed");
    alertError(tr("startTestFailed"), error);
  }
}

async function stopTests() {
  try {
    await invoke("stop_test");
    log("stopping test...");
  } catch (error) {
    alertError(tr("stopFailed"), error);
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

async function saveTestSettings() {
  const keyword = successKeywordInput.value.trim();
  const prompt = testPromptInput.value.trim();
  if (!keyword) {
    alert(tr("missingKeyword"));
    return;
  }
  if (!prompt) {
    alert(tr("missingPrompt"));
    return;
  }
  if (!prompt.includes(keyword)) {
    alert(tr("promptMustContainKeyword"));
    return;
  }
  try {
    await invoke("save_test_settings", { settings: { prompt, success_keyword: keyword } });
    successKeyword = keyword;
    testPrompt = prompt;
    testLog(`saved test settings: success keyword=${successKeyword}`);
    closeTestSettings();
  } catch (error) {
    alertError(tr("saveSettingsFailed"), error);
  }
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
    root.textContent = tr("noModels");
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

function tr(key: string, values: Record<string, string | number> = {}) {
  return translate(language, key, values);
}

function formRequest() {
  const request = {
    type: endpointType.value as EndpointType,
    base_url: baseUrl.value.trim().replace(/\/+$/, ""),
    api_key: apiKey.value.trim(),
    timeout: Number(fetchTimeout.value || 30),
  };
  if (!request.base_url) {
    alert(tr("missingEndpointUrl"));
    return null;
  }
  if (!request.api_key) {
    alert(tr("missingApiKey"));
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
  if (!endpoint) alert(tr("selectEndpointFirst"));
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

function confirmDuplicateEndpointAction(url: string): Promise<"add" | "overwrite" | "cancel"> {
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.className = "choice-modal";
    overlay.innerHTML = `
      <div class="choice-dialog" role="dialog" aria-modal="true" aria-labelledby="duplicate-endpoint-title">
        <h2 id="duplicate-endpoint-title">${escapeHtml(tr("duplicateEndpointTitle"))}</h2>
        <p>${escapeHtml(tr("duplicateEndpointMessage"))}</p>
        <div class="choice-url" title="${escapeAttr(url)}">${escapeHtml(url)}</div>
        <div class="actions choice-actions">
          <button data-action="add">${escapeHtml(tr("addNew"))}</button>
          <button data-action="overwrite" class="danger">${escapeHtml(tr("overwrite"))}</button>
          <button data-action="cancel" class="secondary">${escapeHtml(tr("cancel"))}</button>
        </div>
      </div>
    `;

    const finish = (action: "add" | "overwrite" | "cancel") => {
      document.removeEventListener("keydown", onKeyDown);
      overlay.remove();
      document.body.classList.toggle("modal-open", isTestPanelOpen());
      resolve(action);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") finish("cancel");
    };

    overlay.addEventListener("click", (event) => {
      if (event.target === overlay) finish("cancel");
    });
    overlay.querySelectorAll<HTMLButtonElement>("button[data-action]").forEach((button) => {
      button.addEventListener("click", () => finish(button.dataset.action as "add" | "overwrite" | "cancel"));
    });

    document.body.classList.add("modal-open");
    document.addEventListener("keydown", onKeyDown);
    document.body.append(overlay);
    overlay.querySelector<HTMLButtonElement>('button[data-action="add"]')?.focus();
  });
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
