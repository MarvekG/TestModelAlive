import type { EndpointType, SavedEndpoint } from "../types";
import { escapeAttr, escapeHtml } from "../utils/dom";
import { maskKey } from "../utils/mask";

export function endpointTypeLabel(type: EndpointType) {
  if (type === "codex") return "Codex";
  if (type === "claude") return "Claude";
  return "OpenCode";
}

export function renderEndpointRows(options: {
  root: HTMLTableSectionElement;
  endpoints: SavedEndpoint[];
  selectedEndpointId: string;
  checkedEndpointIds: Set<string>;
  onSelect: (id: string) => void;
  onOpenTest: () => void;
}) {
  const { root, endpoints, selectedEndpointId, checkedEndpointIds, onSelect, onOpenTest } = options;
  root.innerHTML = "";
  for (const endpoint of endpoints) {
    const row = document.createElement("tr");
    row.dataset.id = endpoint.id;
    row.classList.toggle("selected", endpoint.id === selectedEndpointId);
    row.innerHTML = `
      <td class="check-column"></td>
      <td>${escapeHtml(endpoint.name)}</td>
      <td>${endpointTypeLabel(endpoint.type)}${endpoint.type === "opencode" ? `<span class="endpoint-sdk">${escapeHtml(endpoint.opencode_sdk_package)}</span>` : ""}</td>
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
    row.addEventListener("click", () => onSelect(endpoint.id));
    row.addEventListener("dblclick", onOpenTest);
    root.append(row);
  }
}

export function setEndpointChecks(endpoints: SavedEndpoint[], checkedEndpointIds: Set<string>, checked: boolean, render: () => void) {
  checkedEndpointIds.clear();
  if (checked) {
    for (const endpoint of endpoints) checkedEndpointIds.add(endpoint.id);
  }
  render();
}
