import { cssEscape, escapeAttr, escapeHtml } from "../utils/dom";

export function showChoiceDialog<T>(options: {
  title: string;
  message: string;
  detail?: string;
  buttons: { action: string; label: string; className: string }[];
  initialAction: string;
  cancelAction: string;
  isModalOpen: () => boolean;
  resolve: (action: string) => T;
}): Promise<T> {
  return new Promise((resolve) => {
    const titleId = `choice-title-${Date.now().toString(36)}`;
    const overlay = document.createElement("div");
    overlay.className = "choice-modal";
    overlay.innerHTML = `
      <div class="choice-dialog" role="dialog" aria-modal="true" aria-labelledby="${titleId}">
        <h2 id="${titleId}">${escapeHtml(options.title)}</h2>
        <p>${escapeHtml(options.message)}</p>
        ${options.detail ? `<div class="choice-url" title="${escapeAttr(options.detail)}">${escapeHtml(options.detail)}</div>` : ""}
        <div class="actions choice-actions">
          ${options.buttons
            .map((button) => `<button data-action="${escapeAttr(button.action)}" class="${escapeAttr(button.className)}">${escapeHtml(button.label)}</button>`)
            .join("")}
        </div>
      </div>
    `;

    const finish = (action: string) => {
      document.removeEventListener("keydown", onKeyDown);
      overlay.remove();
      document.body.classList.toggle("modal-open", options.isModalOpen());
      resolve(options.resolve(action));
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") finish(options.cancelAction);
    };

    overlay.addEventListener("click", (event) => {
      if (event.target === overlay) finish(options.cancelAction);
    });
    overlay.querySelectorAll<HTMLButtonElement>("button[data-action]").forEach((button) => {
      button.addEventListener("click", () => finish(button.dataset.action ?? options.cancelAction));
    });

    document.body.classList.add("modal-open");
    document.addEventListener("keydown", onKeyDown);
    document.body.append(overlay);
    overlay.querySelector<HTMLButtonElement>(`button[data-action="${cssEscape(options.initialAction)}"]`)?.focus();
  });
}

export function showAlert(title: string, message: string, t: (key: string) => string, isModalOpen: () => boolean, detail = ""): Promise<void> {
  return showChoiceDialog<void>({
    title,
    message,
    detail,
    buttons: [{ action: "ok", label: t("ok"), className: "" }],
    initialAction: "ok",
    cancelAction: "ok",
    isModalOpen,
    resolve: () => undefined,
  });
}

export function showConfirm(
  title: string,
  message: string,
  t: (key: string) => string,
  isModalOpen: () => boolean,
  confirmLabel = t("confirm"),
  detail = "",
  confirmClassName = ""
): Promise<boolean> {
  return showChoiceDialog<boolean>({
    title,
    message,
    detail,
    buttons: [
      { action: "confirm", label: confirmLabel, className: confirmClassName },
      { action: "cancel", label: t("cancel"), className: "secondary" },
    ],
    initialAction: "cancel",
    cancelAction: "cancel",
    isModalOpen,
    resolve: (action) => action === "confirm",
  });
}

export function confirmDuplicateEndpointAction(url: string, t: (key: string) => string, isModalOpen: () => boolean): Promise<"overwrite" | "cancel"> {
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.className = "choice-modal";
    overlay.innerHTML = `
      <div class="choice-dialog" role="dialog" aria-modal="true" aria-labelledby="duplicate-endpoint-title">
        <h2 id="duplicate-endpoint-title">${escapeHtml(t("duplicateEndpointTitle"))}</h2>
        <p>${escapeHtml(t("duplicateEndpointMessage"))}</p>
        <div class="choice-url" title="${escapeAttr(url)}">${escapeHtml(url)}</div>
        <div class="actions choice-actions">
          <button data-action="overwrite" class="danger">${escapeHtml(t("overwrite"))}</button>
          <button data-action="cancel" class="secondary">${escapeHtml(t("cancel"))}</button>
        </div>
      </div>
    `;

    const finish = (action: "overwrite" | "cancel") => {
      document.removeEventListener("keydown", onKeyDown);
      overlay.remove();
      document.body.classList.toggle("modal-open", isModalOpen());
      resolve(action);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") finish("cancel");
    };

    overlay.addEventListener("click", (event) => {
      if (event.target === overlay) finish("cancel");
    });
    overlay.querySelectorAll<HTMLButtonElement>("button[data-action]").forEach((button) => {
      button.addEventListener("click", () => finish(button.dataset.action as "overwrite" | "cancel"));
    });

    document.body.classList.add("modal-open");
    document.addEventListener("keydown", onKeyDown);
    document.body.append(overlay);
    overlay.querySelector<HTMLButtonElement>('button[data-action="cancel"]')?.focus();
  });
}
