import type { Language } from "../i18n";

export type Translator = (key: string, values?: Record<string, string | number>) => string;

export function renderApp(root: HTMLElement, language: Language, t: Translator) {
  root.innerHTML = `
  <main class="shell">
    <header class="app-bar">
      <h1>TestModelAlive</h1>
      <span id="subtitle">${t("subtitle")}</span>
      <button id="language-toggle" class="secondary language-toggle">${language === "zh" ? "English" : "中文"}</button>
    </header>
    <div id="toast" class="toast" role="status" aria-live="polite"></div>

    <section class="workspace">
      <div class="card form-card">
        <div class="card-title">
          <h2 id="add-endpoint-title">${t("addEndpoint")}</h2>
          <button id="clear-input" class="secondary">${t("clear")}</button>
        </div>
        <label><span id="endpoint-name-label">${t("name")}</span>
          <input id="endpoint-name" placeholder="MyEndpoint" />
        </label>
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
              <tr><th id="endpoint-check-header" class="check-column">${t("select")}</th><th id="endpoint-name-header">${t("name")}</th><th id="endpoint-type-header">${t("type")}</th><th>URL</th><th id="endpoint-key-header">${t("sk")}</th><th id="endpoint-model-count-header">${t("modelCount")}</th></tr>
            </thead>
            <tbody id="endpoint-rows"></tbody>
          </table>
        </div>
        <div class="actions wrap">
          <button id="open-test">${t("test")}</button>
          <button id="delete-endpoint" class="danger">${t("delete")}</button>
          <button id="delete-checked" class="danger">${t("batchDelete")}</button>
          <button id="restore-config" class="secondary">${t("restoreConfig")}</button>
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
          <button id="start-real-config-test" class="secondary">${t("startRealConfigTest")}</button>
          <button id="stop-test" class="danger" disabled>${t("stop")}</button>
          <button id="open-test-settings" class="secondary">${t("testSettings")}</button>
          <button id="apply-codex" class="secondary hidden">${t("applyToCodex")}</button>
          <button id="apply-opencode" class="secondary hidden">${t("applyToOpenCode")}</button>
          <button id="apply-claude" class="secondary hidden">${t("applyToClaude")}</button>
          <span id="test-status" class="test-status">${t("notStarted")}</span>
        </div>
        <div class="test-layout">
          <div class="test-box test-left">
          <h3 id="choose-models-title">${t("chooseModels")}</h3>
            <div class="actions compact wrap">
              <button id="test-fetch-models" class="secondary">${t("fetchModels")}</button>
              <button id="test-save-models" class="secondary">${t("saveModels")}</button>
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
}
