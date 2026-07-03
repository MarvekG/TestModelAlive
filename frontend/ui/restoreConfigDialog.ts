import type { CliConfigBaselineItem, CliConfigTargetKind, RestoreCliConfigResult, RestoreSelection } from "../types";
import { escapeAttr, escapeHtml } from "../utils/dom";
import { cliTargetLabel } from "./cliConfigDialog";

export function showRestoreConfigDialog(items: CliConfigBaselineItem[], t: (key: string) => string, isModalOpen: () => boolean): Promise<RestoreSelection[] | null> {
  return new Promise((resolve) => {
    const selected = new Set(items.map((item) => `${item.target}|${item.file_id}|${item.path}`));
    const overlay = document.createElement("div");
    overlay.className = "choice-modal";
    overlay.innerHTML = `
      <div class="restore-config-dialog" role="dialog" aria-modal="true">
        <div class="modal-title">
          <h2>${escapeHtml(t("restoreConfigTitle"))}</h2>
          <button data-action="cancel" class="secondary">${escapeHtml(t("cancel"))}</button>
        </div>
        <p class="settings-hint warning-box">${escapeHtml(t("restoreConfigWarning"))}</p>
        <div class="restore-list">
          ${items
            .map((item, index) => {
              const key = `${item.target}|${item.file_id}|${item.path}`;
              return `<label class="model-check checked restore-item">
                <input type="checkbox" data-index="${index}" data-key="${escapeAttr(key)}" checked />
                <span>${escapeHtml(cliTargetLabel(item.target))} · ${escapeHtml(item.file_id)} · ${escapeHtml(item.path)}</span>
              </label>`;
            })
            .join("")}
        </div>
        <div class="actions choice-actions">
          <button data-action="restore" class="danger">${escapeHtml(t("restoreConfig"))}</button>
          <button data-action="cancel" class="secondary">${escapeHtml(t("cancel"))}</button>
        </div>
      </div>
    `;
    const finish = (value: RestoreSelection[] | null) => {
      document.removeEventListener("keydown", onKeyDown);
      overlay.remove();
      document.body.classList.toggle("modal-open", isModalOpen());
      resolve(value);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") finish(null);
    };
    overlay.querySelectorAll<HTMLInputElement>('input[type="checkbox"]').forEach((checkbox) => {
      checkbox.addEventListener("change", () => {
        const key = checkbox.dataset.key ?? "";
        if (checkbox.checked) selected.add(key);
        else selected.delete(key);
        checkbox.closest("label")?.classList.toggle("checked", checkbox.checked);
      });
    });
    overlay.querySelectorAll<HTMLButtonElement>("button[data-action]").forEach((button) => {
      button.addEventListener("click", () => {
        if (button.dataset.action !== "restore") {
          finish(null);
          return;
        }
        finish(items.filter((item) => selected.has(`${item.target}|${item.file_id}|${item.path}`)).map((item) => ({ target: item.target, file_id: item.file_id, path: item.path })));
      });
    });
    overlay.addEventListener("click", (event) => {
      if (event.target === overlay) finish(null);
    });
    document.body.classList.add("modal-open");
    document.addEventListener("keydown", onKeyDown);
    document.body.append(overlay);
  });
}

export function restoreResultDetail(result: RestoreCliConfigResult) {
  return result.results.map((item) => `${item.ok ? "OK" : "ERR"} ${item.action} ${item.path}${item.error ? `: ${item.error}` : ""}`).join("\n");
}

export type RestoreTarget = CliConfigTargetKind;
