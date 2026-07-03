import type { TestResult } from "../types";
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
