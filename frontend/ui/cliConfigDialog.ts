import type { ApplyCliConfigResult, CliConfigPreview, CliConfigTargetKind, EditedCliConfig } from "../types";
import { escapeHtml } from "../utils/dom";
import { showChoiceDialog } from "./modal";

export function cliTargetLabel(target: CliConfigTargetKind) {
  if (target === "codex") return "Codex";
  if (target === "claude") return "Claude";
  if (target === "deepseek") return "DeepSeek Harness";
  return "OpenCode";
}

export function showCliConfigPreviewDialog(options: {
  preview: CliConfigPreview;
  models: string[];
  t: (key: string, values?: Record<string, string | number>) => string;
  isModalOpen: () => boolean;
  showAlert: (title: string, message: string) => Promise<void>;
  title?: string;
}): Promise<EditedCliConfig | null> {
  const { preview, models, t, isModalOpen, showAlert } = options;
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.className = "choice-modal";
    overlay.innerHTML = `
      <div class="cli-config-dialog" role="dialog" aria-modal="true">
        <div class="modal-title">
          <h2>${escapeHtml(options.title ?? t("cliConfigPreviewTitle", { target: cliTargetLabel(preview.target) }))}</h2>
          <button data-action="cancel" class="secondary">${escapeHtml(t("cancel"))}</button>
        </div>
        <p class="settings-hint warning-box">${escapeHtml(t("cliConfigWarning"))}</p>
        <div class="cli-config-meta">
          <div><strong>${escapeHtml(t("type"))}</strong><span>${escapeHtml(preview.endpoint_type)}</span></div>
          <div><strong>${escapeHtml(t("model"))}</strong><span>${escapeHtml(models.join(", "))}</span></div>
        </div>
        <div class="cli-config-files">
          ${preview.files
            .map(
              (file, index) => `
                <label class="cli-config-file">
                  <span>${escapeHtml(file.file_id)} · ${escapeHtml(file.path)}</span>
                  <textarea class="cli-config-editor" data-index="${index}" spellcheck="false">${escapeHtml(file.content)}</textarea>
                </label>`
            )
            .join("")}
        </div>
        <div class="actions choice-actions">
          <button data-action="apply" class="danger">${escapeHtml(t("confirmApplyConfig"))}</button>
          <button data-action="cancel" class="secondary">${escapeHtml(t("cancel"))}</button>
        </div>
      </div>
    `;
    const finish = (value: EditedCliConfig | null) => {
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
      button.addEventListener("click", async () => {
        if (button.dataset.action !== "apply") {
          finish(null);
          return;
        }
        const files = preview.files.map((file, index) => ({
          file_id: file.file_id,
          path: file.path,
          content: overlay.querySelector<HTMLTextAreaElement>(`textarea[data-index="${index}"]`)?.value ?? "",
        }));
        if (files.some((file) => file.content.trim().length === 0)) {
          await showAlert(t("applyCliConfigFailed"), t("emptyEditedConfig"));
          return;
        }
        finish({ selected_models: models, files });
      });
    });
    document.body.classList.add("modal-open");
    document.addEventListener("keydown", onKeyDown);
    document.body.append(overlay);
    overlay.querySelector<HTMLTextAreaElement>("textarea")?.focus();
  });
}

export function showApplyCliConfigResultDialog(result: ApplyCliConfigResult, t: (key: string, values?: Record<string, string | number>) => string, isModalOpen: () => boolean) {
  const detail = result.results.map((item) => `${item.ok ? "OK" : "ERR"} ${item.action} ${item.path}${item.error ? `: ${item.error}` : ""}`).join("\n");
  return showChoiceDialog<"test" | "close">({
    title: t("applyCliConfigResultTitle"),
    message: t("appliedToTarget", { target: cliTargetLabel(result.target) }),
    detail,
    buttons: [
      { action: "test", label: t("testCurrentConfig"), className: "" },
      { action: "close", label: t("close"), className: "secondary" },
    ],
    initialAction: "test",
    cancelAction: "close",
    isModalOpen,
    resolve: (action) => (action === "test" ? "test" : "close"),
  });
}
