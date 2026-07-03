import { Channel } from "@tauri-apps/api/core";
import { translate, type Language } from "../i18n";
import { addEndpointApi, loadEndpointsApi, deleteEndpointApi } from "../api/endpoints";
import { fetchModelsApi, loadTestSettingsApi, saveTestSettingsApi, stopTestApi, testCliWithRealConfigApi, testModelsApi } from "../api/testModels";
import { applyCliConfigApi, buildCliConfigPreviewApi, buildRemoveOpenCodeConfigPreviewApi, loadCliConfigBaselineItemsApi, restoreOriginalCliConfigApi } from "../api/cliConfig";
import type { CliConfigTargetKind, SavedEndpoint, TestMessage, TestResult, TestSettings } from "../types";
import { createInitialState } from "../state";
import { bind, setBusy } from "../utils/dom";
import { maskKey } from "../utils/mask";
import { renderApp } from "./renderApp";
import { getElements } from "./elements";
import { clearEndpointForm, readEndpointForm } from "./endpointForm";
import { endpointTypeLabel, renderEndpointRows, setEndpointChecks as updateEndpointChecks } from "./endpointList";
import { invertSelection, renderCheckList, setSelection } from "./modelList";
import { appendTimestampedLog, renderTestLogs as renderLogPanel } from "./logPanel";
import { chooseFetchedTestModels, chooseSingleModel, renderResults as renderResultRows } from "./testDialog";
import { closeTestSettingsDialog, openTestSettingsDialog, resetTestSettingsDialog } from "./testSettingsDialog";
import { cliTargetLabel, showApplyCliConfigResultDialog, showCliConfigPreviewDialog } from "./cliConfigDialog";
import { restoreResultDetail, showRestoreConfigDialog } from "./restoreConfigDialog";
import { confirmDuplicateEndpointAction, showAlert as showModalAlert, showConfirm as showModalConfirm } from "./modal";

export function initApp() {
  const app = document.querySelector<HTMLDivElement>("#app");
  if (!app) throw new Error("#app not found");

  const state = createInitialState();
  let endpoints: SavedEndpoint[] = state.endpoints;
  let fetchedModels: string[] = state.fetchedModels;
  let fetchedSelection = state.fetchedSelection;
  let selectedEndpointId = state.selectedEndpointId;
  let checkedEndpointIds = state.checkedEndpointIds;
  let testEndpoint: SavedEndpoint | null = state.testEndpoint;
  let testSelection = state.testSelection;
  let testResults: TestResult[] = state.testResults;
  let testRunning = state.testRunning;
  let testLogChunks: string[] = state.testLogChunks;
  let testPrompt = state.testPrompt;
  let successKeyword = state.successKeyword;
  let language: Language = state.language;
  let toastTimer: number | undefined;

  document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
  renderApp(app, language, t);
  const elements = getElements();

  bindControls();
  void loadEndpoints();
  void loadTestSettings();

  function t(key: string, values: Record<string, string | number> = {}) {
    return translate(language, key, values);
  }

  function bindControls() {
    bind("fetch-models", "click", fetchModels);
    bind("language-toggle", "click", () => {
      localStorage.setItem("language", language === "zh" ? "en" : "zh");
      window.location.reload();
    });
    bind("save-endpoint", "click", saveEndpoint);
    bind("clear-input", "click", clearInput);
    bind("reload-endpoints", "click", loadEndpoints);
    bind("endpoint-filter-text", "input", renderEndpoints);
    bind("endpoint-filter-type", "change", renderEndpoints);
    bind("open-test", "click", openTestPanel);
    bind("delete-endpoint", "click", deleteSelectedEndpoint);
    bind("delete-checked", "click", deleteCheckedEndpoints);
    bind("restore-config", "click", openRestoreConfigDialog);
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
    bind("test-fetch-models", "click", fetchTestModels);
    bind("test-save-models", "click", saveTestModels);
    bind("test-all", "click", () => setSelection(testSelection, testEndpoint?.models ?? [], true, renderTestModels));
    bind("test-none", "click", () => setSelection(testSelection, testEndpoint?.models ?? [], false, renderTestModels));
    bind("test-invert", "click", () => invertSelection(testSelection, testEndpoint?.models ?? [], renderTestModels));
    bind("start-test", "click", runTests);
    bind("start-real-config-test", "click", runRealConfigTests);
    bind("stop-test", "click", stopTests);
    bind("open-test-settings", "click", () => openTestSettingsDialog(elements, successKeyword, testPrompt));
    bind("apply-codex", "click", () => applyCliConfig("codex"));
    bind("apply-opencode", "click", () => applyCliConfig("opencode"));
    bind("remove-opencode", "click", removeOpenCodeConfig);
    bind("apply-claude", "click", () => applyCliConfig("claude"));
    bind("close-test-settings", "click", () => closeTestSettingsDialog(elements));
    bind("save-test-settings", "click", saveTestSettings);
    bind("reset-test-settings", "click", () => resetTestSettingsDialog(elements));
    bind("copy-test-log", "click", copyTestLog);
    bind("clear-test-log", "click", () => {
      testLogChunks = [];
      renderTestLogs();
    });
  }

  async function loadEndpoints() {
    try {
      endpoints = await loadEndpointsApi();
      checkedEndpointIds = new Set([...checkedEndpointIds].filter((id) => endpoints.some((endpoint) => endpoint.id === id)));
      renderEndpoints();
    } catch (error) {
      alertError(t("readEndpointsFailed"), error);
    }
  }

  async function loadTestSettings() {
    try {
      const settings = await loadTestSettingsApi();
      testPrompt = settings.prompt;
      successKeyword = settings.success_keyword;
    } catch (error) {
      alertError(t("readSettingsFailed"), error);
    }
  }

  async function fetchModels() {
    const request = await formRequest(t("fetchModels"));
    if (!request) return;
    log(`fetching models: type=${request.type} url=${request.base_url}`);
    setBusy("fetch-models", true);
    try {
      fetchedModels = await fetchModelsApi(request);
      fetchedSelection = new Set(fetchedModels);
      renderFetchedModels();
      log(`fetched ${fetchedModels.length} models`);
    } catch (error) {
      alertError(t("fetchFailed"), error);
      log(`fetch failed: ${String(error)}`);
    } finally {
      setBusy("fetch-models", false);
    }
  }

  async function saveEndpoint() {
    const request = await formRequest(t("saveEndpoint"));
    if (!request) return;
    const models = fetchedModels.filter((model) => fetchedSelection.has(model));
    if (models.length === 0) {
      await showAlert(t("saveEndpoint"), t("selectAtLeastOneFetchedModel"));
      return;
    }
    const duplicate = endpoints.find((endpoint) => endpoint.type === request.type && endpoint.base_url === request.base_url);
    let overwrite = false;
    if (duplicate) {
      const action = await confirmDuplicateEndpointAction(request.base_url, t, isTestPanelOpen);
      if (action === "cancel") return;
      overwrite = action === "overwrite";
    }
    try {
      const savedEndpoint = await addEndpointApi({ ...request, models, overwrite });
      selectedEndpointId = savedEndpoint.id;
      log(`${overwrite ? "overwrote" : "saved"} endpoint: type=${request.type} url=${request.base_url} models=${models.length}`);
      await loadEndpoints();
    } catch (error) {
      alertError(t("saveFailed"), error);
    }
  }

  async function deleteSelectedEndpoint() {
    const endpoint = selectedEndpoint();
    if (!endpoint) return;
    if (!(await confirmDeleteAction(t("deleteEndpointTitle"), t("confirmDeleteEndpoint"), endpoint.base_url))) return;
    try {
      await deleteEndpointApi(endpoint.id);
      log(`deleted endpoint: ${endpoint.base_url}`);
      selectedEndpointId = "";
      await loadEndpoints();
    } catch (error) {
      alertError(t("deleteFailed"), error);
    }
  }

  async function deleteCheckedEndpoints() {
    const selected = endpoints.filter((endpoint) => checkedEndpointIds.has(endpoint.id));
    if (selected.length === 0) {
      await showAlert(t("batchDeleteEndpointTitle"), t("checkEndpointsFirst"));
      return;
    }
    if (!(await confirmDeleteAction(t("batchDeleteEndpointTitle"), t("confirmDeleteChecked", { count: selected.length })))) return;
    try {
      for (const endpoint of selected) {
        await deleteEndpointApi(endpoint.id);
        log(`deleted endpoint: ${endpoint.base_url}`);
      }
      checkedEndpointIds.clear();
      if (selected.some((endpoint) => endpoint.id === selectedEndpointId)) selectedEndpointId = "";
      await loadEndpoints();
    } catch (error) {
      alertError(t("batchDeleteFailed"), error);
    }
  }

  function loadSelectedEndpointToForm() {
    const endpoint = selectedEndpoint();
    if (!endpoint) return;
    elements.endpointType.value = endpoint.type;
    elements.endpointName.value = endpoint.name;
    elements.baseUrl.value = endpoint.base_url;
    elements.apiKey.value = endpoint.api_key;
    fetchedModels = [...endpoint.models];
    fetchedSelection = new Set(fetchedModels);
    renderFetchedModels();
    log(`loaded endpoint into form: ${endpoint.name} ${endpoint.base_url}`);
  }

  function openTestPanel() {
    const endpoint = selectedEndpoint();
    if (!endpoint) return;
    testEndpoint = endpoint;
    testSelection = new Set(endpoint.models);
    testResults = [];
    testLogChunks = [];
    elements.append1m.checked = false;
    elements.testPanel.classList.remove("hidden");
    document.body.classList.add("modal-open");
    elements.testType.textContent = endpointTypeLabel(endpoint.type);
    elements.testUrl.textContent = endpoint.base_url;
    elements.testKey.textContent = maskKey(endpoint.api_key);
    elements.testStatus.textContent = t("notStarted");
    elements.append1mLabel.classList.toggle("hidden", endpoint.type !== "claude");
    elements.applyCodex.classList.toggle("hidden", endpoint.type !== "codex");
    elements.applyOpenCode.classList.toggle("hidden", endpoint.type !== "opencode");
    elements.removeOpenCode.classList.toggle("hidden", endpoint.type !== "opencode");
    elements.applyClaude.classList.toggle("hidden", endpoint.type !== "claude");
    renderTestModels();
    renderResults();
    renderTestLogs();
  }

  async function applyCliConfig(target: CliConfigTargetKind) {
    if (!testEndpoint) return;
    if (testRunning) {
      await showAlert(t("testModels"), t("testStillRunning"));
      return;
    }
    const models = selectedTestModels();
    if (target === "opencode" && models.length === 0) {
      await showAlert(cliTargetLabel(target), t("selectAtLeastOneModelForOpenCode"));
      return;
    }
    if (target !== "opencode" && models.length !== 1) {
      await showAlert(cliTargetLabel(target), t("selectExactlyOneModelForCli"));
      return;
    }
    setApplyBusy(true);
    try {
      const defaultModel = await chooseOpenCodeDefaultModel(target, models);
      const preview = await buildCliConfigPreviewApi(testEndpoint, target, models, defaultModel);
      const editedConfig = await showCliConfigPreviewDialog({ preview, models, t, isModalOpen: isTestPanelOpen, showAlert });
      if (!editedConfig) return;
      const result = await applyCliConfigApi(testEndpoint, target, editedConfig);
      const action = await showApplyCliConfigResultDialog(result, t, isTestPanelOpen);
      if (action === "test") await testCliWithRealConfig(target, models);
    } catch (error) {
      alertError(t("applyCliConfigFailed"), error);
    } finally {
      setApplyBusy(false);
    }
  }

  async function testCliWithRealConfig(target: CliConfigTargetKind, models: string[]) {
    if (!testEndpoint) return;
    testResults = [];
    renderResults();
    setTestRunning(true);
    setApplyBusy(true);
    elements.testStatus.textContent = `${t("running")}: ${models.length}`;
    testLog(`starting real CLI config test: target=${target} models=${models.length}`);
    const onEvent = createTestEventChannel({ logToPanel: true, onFinished: () => setApplyBusy(false) });
    try {
      await testCliWithRealConfigApi(
        { target, endpoint_name: testEndpoint.name, models, timeout: Number(elements.testTimeout.value || 120), prompt: testPrompt, success_keyword: successKeyword },
        onEvent,
      );
    } catch (error) {
      setTestRunning(false);
      setApplyBusy(false);
      alertError(t("startTestFailed"), error);
    }
  }

  async function chooseOpenCodeDefaultModel(target: CliConfigTargetKind, models: string[]) {
    if (target !== "opencode") return null;
    if (!(await showConfirm(cliTargetLabel(target), t("setOpenCodeDefaultModel")))) return null;
    return chooseSingleModel({ title: t("chooseDefaultModel"), models, t, isModalOpen: isTestPanelOpen });
  }

  async function runRealConfigTests() {
    if (!testEndpoint) return;
    if (testRunning) {
      await showAlert(t("testModels"), t("testStillRunning"));
      return;
    }
    const target: CliConfigTargetKind = testEndpoint.type;
    const models = selectedTestModels();
    if (target === "opencode" && models.length === 0) {
      await showAlert(cliTargetLabel(target), t("selectAtLeastOneModelForOpenCode"));
      return;
    }
    if (target !== "opencode" && models.length !== 1) {
      await showAlert(cliTargetLabel(target), t("selectExactlyOneModelForCli"));
      return;
    }
    await testCliWithRealConfig(target, models);
  }

  async function openRestoreConfigDialog() {
    try {
      const items = await loadCliConfigBaselineItemsApi();
      if (items.length === 0) {
        await showAlert(t("restoreConfig"), t("noRestorableConfig"));
        return;
      }
      const selectedItems = await showRestoreConfigDialog(items, t, isTestPanelOpen);
      if (!selectedItems) return;
      if (selectedItems.length === 0) {
        await showAlert(t("restoreConfig"), t("selectRestoreItems"));
        return;
      }
      const result = await restoreOriginalCliConfigApi(selectedItems);
      await showAlert(t("restoreConfigResultTitle"), t("restoredConfig"), restoreResultDetail(result));
    } catch (error) {
      alertError(t("restoreConfigFailed"), error);
    }
  }

  async function removeOpenCodeConfig() {
    if (!testEndpoint) return;
    if (testRunning) {
      await showAlert(t("testModels"), t("testStillRunning"));
      return;
    }
    if (testEndpoint.type !== "opencode") return;
    setApplyBusy(true);
    try {
      const preview = await buildRemoveOpenCodeConfigPreviewApi(testEndpoint);
      const models = testEndpoint.models.length > 0 ? testEndpoint.models : selectedTestModels();
      const editedConfig = await showCliConfigPreviewDialog({ preview, models, t, isModalOpen: isTestPanelOpen, showAlert, title: t("removeFromOpenCode") });
      if (!editedConfig) return;
      const result = await applyCliConfigApi(testEndpoint, "opencode", editedConfig);
      await showAlert(t("removeFromOpenCode"), t("removedFromOpenCode"), result.results.map((item) => item.path).join("\n"));
    } catch (error) {
      alertError(t("removeOpenCodeFailed"), error);
    } finally {
      setApplyBusy(false);
    }
  }

  async function closeTestPanel() {
    if (testRunning) {
      await showAlert(t("testModels"), t("testStillRunning"));
      return;
    }
    elements.testPanel.classList.add("hidden");
    document.body.classList.remove("modal-open");
  }

  async function runTests() {
    if (!testEndpoint) return;
    const models = selectedTestModels();
    if (models.length === 0) {
      await showAlert(t("testModels"), t("selectAtLeastOneModel"));
      return;
    }
    testResults = [];
    renderResults();
    setTestRunning(true);
    elements.testStatus.textContent = `${t("running")}: ${models.length}`;
    log(`starting CLI test request: type=${testEndpoint.type} url=${testEndpoint.base_url} models=${models.length} timeout=${Number(elements.testTimeout.value || 120)}s`);
    const onEvent = createTestEventChannel({ logToPanel: false });
    try {
      await testModelsApi(
        {
          endpoint: testEndpoint,
          models,
          timeout: Number(elements.testTimeout.value || 120),
          append_1m: elements.append1m.checked,
          prompt: testPrompt,
          success_keyword: successKeyword,
        },
        onEvent,
      );
    } catch (error) {
      setTestRunning(false);
      elements.testStatus.textContent = t("launchFailed");
      alertError(t("startTestFailed"), error);
    }
  }

  async function fetchTestModels() {
    if (!testEndpoint) return;
    if (testRunning) {
      await showAlert(t("testModels"), t("testStillRunning"));
      return;
    }
    const request = { type: testEndpoint.type, base_url: testEndpoint.base_url, api_key: testEndpoint.api_key, timeout: Number(elements.fetchTimeout.value || 30) };
    testLog(`fetching latest models: type=${request.type} url=${request.base_url}`);
    setBusy("test-fetch-models", true);
    try {
      const models = await fetchModelsApi(request);
      testLog(`fetched latest models: ${models.length}`);
      const selectedModels = await chooseFetchedTestModels({ models, t, emptyText: t("noModels"), isModalOpen: isTestPanelOpen, showAlert });
      if (!selectedModels) return;
      await saveTestModelSelection(selectedModels);
    } catch (error) {
      alertError(t("fetchFailed"), error);
      testLog(`fetch latest models failed: ${String(error)}`);
    } finally {
      setBusy("test-fetch-models", false);
    }
  }

  async function saveTestModels() {
    await saveTestModelSelection(selectedTestModels());
  }

  async function saveTestModelSelection(models: string[]) {
    if (!testEndpoint) return;
    if (models.length === 0) {
      await showAlert(t("testModels"), t("selectAtLeastOneModel"));
      return;
    }
    setBusy("test-save-models", true);
    try {
      const savedEndpoint = await addEndpointApi({ name: testEndpoint.name, type: testEndpoint.type, base_url: testEndpoint.base_url, api_key: testEndpoint.api_key, models, overwrite: true });
      testEndpoint = savedEndpoint;
      testSelection = new Set(models);
      selectedEndpointId = savedEndpoint.id;
      testLog(`saved selected models: ${models.length}`);
      await loadEndpoints();
      renderTestModels();
    } catch (error) {
      alertError(t("saveFailed"), error);
    } finally {
      setBusy("test-save-models", false);
    }
  }

  async function stopTests() {
    try {
      await stopTestApi();
      log("stopping test...");
    } catch (error) {
      alertError(t("stopFailed"), error);
    }
  }

  async function saveTestSettings() {
    const keyword = elements.successKeywordInput.value.trim();
    const prompt = elements.testPromptInput.value.trim();
    if (!keyword) {
      await showAlert(t("testSettings"), t("missingKeyword"));
      return;
    }
    if (!prompt) {
      await showAlert(t("testSettings"), t("missingPrompt"));
      return;
    }
    if (!prompt.includes(keyword)) {
      await showAlert(t("testSettings"), t("promptMustContainKeyword"));
      return;
    }
    try {
      const settings: TestSettings = { prompt, success_keyword: keyword };
      await saveTestSettingsApi(settings);
      successKeyword = keyword;
      testPrompt = prompt;
      testLog(`saved test settings: success keyword=${successKeyword}`);
      closeTestSettingsDialog(elements);
    } catch (error) {
      alertError(t("saveSettingsFailed"), error);
    }
  }

  function createTestEventChannel(options: { logToPanel: boolean; onFinished?: () => void }) {
    return new Channel<TestMessage>((message) => {
      if (message.kind === "log" && message.message !== undefined) {
        if (message.stream) appendStreamLog(message.message);
        else if (options.logToPanel) testLog(message.message);
        else log(message.message);
      } else if (message.kind === "result" && message.result) {
        testResults.push(message.result);
        renderResults();
      } else if (message.kind === "finished") {
        setTestRunning(false);
        options.onFinished?.();
        elements.testStatus.textContent = t("ended");
      }
    });
  }

  function renderEndpoints() {
    const visibleEndpoints = filteredEndpoints();
    renderEndpointRows({
      root: elements.endpointRows,
      endpoints: visibleEndpoints,
      selectedEndpointId,
      checkedEndpointIds,
      onSelect: (id) => {
        selectedEndpointId = id;
        renderEndpoints();
      },
      onOpenTest: openTestPanel,
    });
  }

  function setEndpointChecks(checked: boolean) {
    updateEndpointChecks(filteredEndpoints(), checkedEndpointIds, checked, renderEndpoints);
  }

  function filteredEndpoints() {
    const query = elements.endpointFilterText.value.trim().toLowerCase();
    const type = elements.endpointFilterType.value;
    return endpoints.filter((endpoint) => {
      if (type !== "all" && endpoint.type !== type) return false;
      if (!query) return true;
      return [endpoint.name, endpoint.type, endpoint.base_url, ...endpoint.models]
        .some((value) => value.toLowerCase().includes(query));
    });
  }

  function renderFetchedModels() {
    renderCheckList(elements.fetchedModelsEl, fetchedModels, fetchedSelection, "fetched", t("noModels"));
  }

  function renderTestModels() {
    renderCheckList(elements.testModelsEl, testEndpoint?.models ?? [], testSelection, "test", t("noModels"));
  }

  function renderResults() {
    renderResultRows(elements.resultRows, testResults);
  }

  function renderTestLogs() {
    renderLogPanel(elements.testLogOutput, testLogChunks);
  }

  function formRequest(title: string) {
    return readEndpointForm(elements, title, showAlert, t);
  }

  function clearInput() {
    clearEndpointForm(elements);
    fetchedModels = [];
    fetchedSelection.clear();
    renderFetchedModels();
  }

  function selectedEndpoint() {
    const endpoint = endpoints.find((item) => item.id === selectedEndpointId);
    if (!endpoint) void showAlert(t("savedEndpoints"), t("selectEndpointFirst"));
    return endpoint;
  }

  async function copyFromSelected(labelText: string, getter: (endpoint: SavedEndpoint) => string) {
    const endpoint = selectedEndpoint();
    if (!endpoint) return;
    await navigator.clipboard.writeText(getter(endpoint));
    showToast(t("copied", { label: labelText }));
    log(`copied ${labelText}: ${endpoint.base_url}`);
  }

  async function copyFromTest(labelText: string, getter: (endpoint: SavedEndpoint) => string) {
    if (!testEndpoint) return;
    await navigator.clipboard.writeText(getter(testEndpoint));
    showToast(t("copied", { label: labelText }));
    testLog(`copied ${labelText}`);
  }

  async function copyTestLog() {
    await navigator.clipboard.writeText(testLogChunks.join(""));
    testLog("copied log");
  }

  function selectedTestModels() {
    return testEndpoint?.models.filter((model) => testSelection.has(model)) ?? [];
  }

  function setApplyBusy(busy: boolean) {
    elements.applyCodex.disabled = busy;
    elements.applyOpenCode.disabled = busy;
    elements.removeOpenCode.disabled = busy;
    elements.applyClaude.disabled = busy;
  }

  function setTestRunning(running: boolean) {
    testRunning = running;
    elements.startTest.disabled = running;
    elements.startRealConfigTest.disabled = running;
    elements.stopTest.disabled = !running;
  }

  function log(message: string) {
    if (isTestPanelOpen()) appendTimestampedLog(testLogChunks, message);
    renderTestLogs();
  }

  function appendStreamLog(message: string) {
    if (isTestPanelOpen()) testLogChunks.push(message);
    renderTestLogs();
  }

  function testLog(message: string) {
    appendTimestampedLog(testLogChunks, message);
    renderTestLogs();
  }

  function showToast(message: string) {
    elements.toast.textContent = message;
    elements.toast.classList.add("visible");
    window.clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => {
      elements.toast.classList.remove("visible");
    }, 1600);
  }

  function isTestPanelOpen() {
    return !elements.testPanel.classList.contains("hidden");
  }

  function confirmDeleteAction(title: string, message: string, detail = "") {
    return showConfirm(title, message, t("delete"), detail, "danger");
  }

  function showAlert(title: string, message: string, detail = "") {
    return showModalAlert(title, message, t, isTestPanelOpen, detail);
  }

  function showConfirm(title: string, message: string, confirmLabel = t("confirm"), detail = "", confirmClassName = "") {
    return showModalConfirm(title, message, t, isTestPanelOpen, confirmLabel, detail, confirmClassName);
  }

  function alertError(title: string, error: unknown) {
    void showAlert(title, String(error));
  }
}
