import { invokeCommand } from "./tauri";
import type { SavedEndpoint } from "../types";

export function loadEndpointsApi() {
  return invokeCommand<SavedEndpoint[]>("load_endpoints");
}

export function deleteEndpointApi(endpointId: string) {
  return invokeCommand("delete_endpoint", { endpointId });
}

export function addEndpointApi(request: Record<string, unknown>) {
  return invokeCommand<SavedEndpoint>("add_endpoint", { request });
}
