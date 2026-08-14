import { invokeCommand } from "./tauri";
import type {
  ApplyCliConfigResult,
  CliConfigBaselineItem,
  CliConfigPreview,
  CliConfigTargetKind,
  EditedCliConfig,
  OpenCodeTimeoutOptions,
  RestoreCliConfigResult,
  RestoreSelection,
  SavedEndpoint,
} from "../types";

export function buildCliConfigPreviewApi(
  endpoint: SavedEndpoint,
  target: CliConfigTargetKind,
  selectedModels: string[],
  defaultModel: string | null = null,
  timeouts: OpenCodeTimeoutOptions | null = null,
  deepseekApiProtocol: string | null = null,
) {
  return invokeCommand<CliConfigPreview>("build_cli_config_preview", {
    endpoint,
    target,
    selectedModels,
    defaultModel,
    timeouts,
    deepseekApiProtocol,
  });
}

export function buildRemoveOpenCodeConfigPreviewApi(endpoint: SavedEndpoint) {
  return invokeCommand<CliConfigPreview>("build_remove_opencode_config_preview", { endpoint });
}

export function buildRemoveDeepSeekProviderPreviewApi(endpoint: SavedEndpoint) {
  return invokeCommand<CliConfigPreview>("build_remove_deepseek_provider_preview", { endpoint });
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
