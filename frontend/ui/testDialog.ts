import type { OpenCodeTimeoutOptions, TestResult } from "../types";
import { escapeHtml } from "../utils/dom";
import { renderCheckList, setSelection, invertSelection } from "./modelList";

export function renderResults(root: HTMLTableSectionElement, results: TestResult[]) {
  root.innerHTML = "";
  for (const result of results) {
    const row = document.createElement("tr");
    row.innerHTML = `
      <td>${escapeHtml(result.model)}</td>
      <td><span class="status ${result.status.toLowerCase()}">${escapeHtml(result.status)}</span></td>
      <td>${result.seconds.toFixed(1)}s</td>
    `;
    root.append(row);
  }
}

export function chooseFetchedTestModels(options: {
  models: string[];
  t: (key: string) => string;
  emptyText: string;
  isModalOpen: () => boolean;
  showAlert: (title: string, message: string) => Promise<void>;
}): Promise<string[] | null> {
  const { models, t, emptyText, isModalOpen, showAlert } = options;
  return new Promise((resolve) => {
    const selection = new Set(models);
    const overlay = document.createElement("div");
    overlay.className = "choice-modal";
    overlay.innerHTML = `
      <div class="model-picker-dialog" role="dialog" aria-modal="true" aria-labelledby="fetched-test-models-title">
        <div class="modal-title">
          <h2 id="fetched-test-models-title">${escapeHtml(t("fetchedModels"))}</h2>
          <button data-action="cancel" class="secondary">${escapeHtml(t("cancel"))}</button>
        </div>
        <div class="actions compact wrap">
          <button data-action="all" class="secondary">${escapeHtml(t("selectAll"))}</button>
          <button data-action="none" class="secondary">${escapeHtml(t("selectNone"))}</button>
          <button data-action="invert" class="secondary">${escapeHtml(t("invert"))}</button>
          <button data-action="save">${escapeHtml(t("saveModels"))}</button>
        </div>
        <div class="check-list model-picker-list"></div>
      </div>
    `;
    const list = overlay.querySelector<HTMLDivElement>(".model-picker-list");
    if (!list) {
      resolve(null);
      return;
    }

    const render = () => renderCheckList(list, models, selection, "fetched-test", emptyText);
    const finish = (selectedModels: string[] | null) => {
      document.removeEventListener("keydown", onKeyDown);
      overlay.remove();
      document.body.classList.toggle("modal-open", isModalOpen());
      resolve(selectedModels);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") finish(null);
    };

    overlay.addEventListener("click", (event) => {
      if (event.target === overlay) finish(null);
    });
    overlay.querySelectorAll<HTMLButtonElement>("button[data-action]").forEach((button) => {
      button.addEventListener("click", async () => {
        const action = button.dataset.action;
        if (action === "cancel") finish(null);
        else if (action === "all") setSelection(selection, models, true, render);
        else if (action === "none") setSelection(selection, models, false, render);
        else if (action === "invert") invertSelection(selection, models, render);
        else if (action === "save") {
          const selectedModels = models.filter((model) => selection.has(model));
          if (selectedModels.length === 0) {
            await showAlert(t("testModels"), t("selectAtLeastOneModel"));
            return;
          }
          finish(selectedModels);
        }
      });
    });

    document.body.classList.add("modal-open");
    document.addEventListener("keydown", onKeyDown);
    render();
    document.body.append(overlay);
    overlay.querySelector<HTMLButtonElement>('button[data-action="save"]')?.focus();
  });
}

export function chooseOpenCodeApplyOptions(options: {
  models: string[];
  defaultTimeoutSeconds: number;
  t: (key: string) => string;
  isModalOpen: () => boolean;
  showAlert: (title: string, message: string) => Promise<void>;
}): Promise<{ defaultModel: string | null; timeouts: OpenCodeTimeoutOptions | null } | null> {
  const { models, defaultTimeoutSeconds, t, isModalOpen, showAlert } = options;
  const defaults = {
    timeoutSeconds: defaultTimeoutSeconds,
    headerTimeoutSeconds: 120,
    chunkTimeoutSeconds: 120,
  };
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.className = "choice-modal";
    overlay.innerHTML = `
      <div class="choice-dialog opencode-options-dialog" role="dialog" aria-modal="true">
        <h2>${escapeHtml(t("openCodeApplyOptionsTitle"))}</h2>
        <section class="opencode-option-card">
          <label class="inline-check option-toggle">
            <input data-field="setDefaultModel" type="checkbox" />
            ${escapeHtml(t("openCodeDefaultModelOption"))}
          </label>
          <small>${escapeHtml(t("openCodeDefaultModelHint"))}</small>
          <select class="model-select" disabled>
            ${models.map((model) => `<option value="${escapeHtml(model)}">${escapeHtml(model)}</option>`).join("")}
          </select>
        </section>
        <section class="opencode-option-card">
          <label class="inline-check option-toggle">
            <input data-field="setTimeouts" type="checkbox" checked />
            ${escapeHtml(t("openCodeTimeoutOption"))}
          </label>
          <small>${escapeHtml(t("openCodeTimeoutHint"))}</small>
          <div class="timeout-grid">
            <label>
              <span>${escapeHtml(t("openCodeTimeout"))} (s)</span>
              <input data-field="timeout" class="timeout-input" type="number" min="1" max="86400" value="${defaults.timeoutSeconds}" disabled />
            </label>
            <label>
              <span>${escapeHtml(t("openCodeHeaderTimeout"))} (s)</span>
              <input data-field="headerTimeout" class="timeout-input" type="number" min="1" max="86400" value="${defaults.headerTimeoutSeconds}" disabled />
            </label>
            <label>
              <span>${escapeHtml(t("openCodeChunkTimeout"))} (s)</span>
              <input data-field="chunkTimeout" class="timeout-input" type="number" min="1" max="86400" value="${defaults.chunkTimeoutSeconds}" disabled />
            </label>
          </div>
        </section>
        <div class="actions choice-actions">
          <button data-action="confirm">${escapeHtml(t("confirm"))}</button>
          <button data-action="cancel" class="secondary">${escapeHtml(t("cancel"))}</button>
        </div>
      </div>
    `;
    const setDefaultModel = overlay.querySelector<HTMLInputElement>('input[data-field="setDefaultModel"]');
    const setTimeouts = overlay.querySelector<HTMLInputElement>('input[data-field="setTimeouts"]');
    const modelSelect = overlay.querySelector<HTMLSelectElement>("select.model-select");
    const timeoutInputs = [...overlay.querySelectorAll<HTMLInputElement>("input.timeout-input")];
    const finish = (value: { defaultModel: string | null; timeouts: OpenCodeTimeoutOptions | null } | null) => {
      document.removeEventListener("keydown", onKeyDown);
      overlay.remove();
      document.body.classList.toggle("modal-open", isModalOpen());
      resolve(value);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") finish(null);
    };
    const readSeconds = (field: string) => {
      const value = Number(overlay.querySelector<HTMLInputElement>(`input[data-field="${field}"]`)?.value || 0);
      if (!Number.isFinite(value) || !Number.isInteger(value) || value < 1 || value > 86400) return null;
      return value;
    };
    const updateEnabled = () => {
      if (modelSelect) modelSelect.disabled = !setDefaultModel?.checked;
      timeoutInputs.forEach((input) => (input.disabled = !setTimeouts?.checked));
    };
    overlay.addEventListener("click", (event) => {
      if (event.target === overlay) finish(null);
    });
    setDefaultModel?.addEventListener("change", updateEnabled);
    setTimeouts?.addEventListener("change", updateEnabled);
    overlay.querySelectorAll<HTMLButtonElement>("button[data-action]").forEach((button) => {
      button.addEventListener("click", async () => {
        if (button.dataset.action !== "confirm") {
          finish(null);
          return;
        }
        const defaultModel = setDefaultModel?.checked ? modelSelect?.value ?? null : null;
        let timeouts: OpenCodeTimeoutOptions | null = null;
        if (setTimeouts?.checked) {
          const timeoutSeconds = readSeconds("timeout");
          const headerTimeoutSeconds = readSeconds("headerTimeout");
          const chunkTimeoutSeconds = readSeconds("chunkTimeout");
          if (timeoutSeconds == null || headerTimeoutSeconds == null || chunkTimeoutSeconds == null) {
            await showAlert(t("openCodeApplyOptionsTitle"), t("invalidOpenCodeTimeout"));
            return;
          }
          timeouts = {
            timeout_ms: timeoutSeconds * 1000,
            header_timeout_ms: headerTimeoutSeconds * 1000,
            chunk_timeout_ms: chunkTimeoutSeconds * 1000,
          };
        }
        finish({ defaultModel, timeouts });
      });
    });
    document.body.classList.add("modal-open");
    document.addEventListener("keydown", onKeyDown);
    document.body.append(overlay);
    updateEnabled();
    setDefaultModel?.focus();
  });
}

export function chooseDeepSeekApplyOptions(options: {
  models: string[];
  t: (key: string) => string;
  isModalOpen: () => boolean;
}): Promise<string | null> {
  const { models, t, isModalOpen } = options;
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.className = "choice-modal";
    overlay.innerHTML = `
      <div class="choice-dialog opencode-options-dialog" role="dialog" aria-modal="true">
        <h2>${escapeHtml(t("deepSeekApplyOptionsTitle"))}</h2>
        <section class="opencode-option-card">
          <label>
            <span>${escapeHtml(t("deepSeekDefaultModel"))}</span>
            <select class="model-select">
              ${models.map((model) => `<option value="${escapeHtml(model)}">${escapeHtml(model)}</option>`).join("")}
            </select>
          </label>
          <small>${escapeHtml(t("deepSeekDefaultModelHint"))}</small>
        </section>
        <div class="actions choice-actions">
          <button data-action="confirm">${escapeHtml(t("confirm"))}</button>
          <button data-action="cancel" class="secondary">${escapeHtml(t("cancel"))}</button>
        </div>
      </div>
    `;
    const modelSelect = overlay.querySelector<HTMLSelectElement>("select.model-select");
    const finish = (value: string | null) => {
      document.removeEventListener("keydown", onKeyDown);
      overlay.remove();
      document.body.classList.toggle("modal-open", isModalOpen());
      resolve(value);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") finish(null);
    };
    overlay.addEventListener("click", (event) => {
      if (event.target === overlay) finish(null);
    });
    overlay.querySelectorAll<HTMLButtonElement>("button[data-action]").forEach((button) => {
      button.addEventListener("click", () => {
        if (button.dataset.action === "confirm") finish(modelSelect?.value ?? null);
        else finish(null);
      });
    });
    document.body.classList.add("modal-open");
    document.addEventListener("keydown", onKeyDown);
    document.body.append(overlay);
    modelSelect?.focus();
  });
}
