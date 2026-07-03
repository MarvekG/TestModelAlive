import { invokeCommand } from "./tauri";
import type { TestMessage, TestSettings } from "../types";

export function loadTestSettingsApi() {
  return invokeCommand<TestSettings>("load_test_settings");
}

export function fetchModelsApi(request: Record<string, unknown>) {
  return invokeCommand<string[]>("fetch_models", { request });
}

export function saveTestSettingsApi(settings: TestSettings) {
  return invokeCommand("save_test_settings", { settings });
}

export function testModelsApi(request: Record<string, unknown>, onEvent: unknown) {
  return invokeCommand("test_models", { request, onEvent });
}

export function testCliWithRealConfigApi(request: Record<string, unknown>, onEvent: unknown) {
  return invokeCommand("test_cli_with_real_config", { request, onEvent });
}

export function stopTestApi() {
  return invokeCommand("stop_test");
}

export function createTestEventChannel(onMessage: (message: TestMessage) => void) {
  return onMessage;
}
