import { invokeCommand } from "./tauri";
import type { ApplyCliConfigResult, CliConfigBaselineItem, CliConfigPreview, CliConfigTargetKind, EditedCliConfig, RestoreCliConfigResult, RestoreSelection, SavedEndpoint } from "../types";

export function buildCliConfigPreviewApi(endpoint: SavedEndpoint, target: CliConfigTargetKind, selectedModels: string[]) {
  return invokeCommand<CliConfigPreview>("build_cli_config_preview", { endpoint, target, selectedModels });
}

export function applyCliConfigApi(endpoint: SavedEndpoint, target: CliConfigTargetKind, editedConfig: EditedCliConfig) {
  return invokeCommand<ApplyCliConfigResult>("apply_cli_config", { endpoint, target, editedConfig });
}

export function loadCliConfigBaselineItemsApi() {
  return invokeCommand<CliConfigBaselineItem[]>("load_cli_config_baseline_items");
}

export function restoreOriginalCliConfigApi(selectedItems: RestoreSelection[]) {
  return invokeCommand<RestoreCliConfigResult>("restore_original_cli_config", { selectedItems });
}
