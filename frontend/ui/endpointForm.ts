import type { EndpointType } from "../types";
import type { UiElements } from "./elements";

export async function readEndpointForm(elements: UiElements, title: string, showAlert: (title: string, message: string) => Promise<void>, t: (key: string) => string) {
  const request = {
    name: elements.endpointName.value.trim(),
    type: elements.endpointType.value as EndpointType,
    base_url: elements.baseUrl.value.trim().replace(/\/+$/, ""),
    api_key: elements.apiKey.value.trim(),
    timeout: Number(elements.fetchTimeout.value || 30),
  };
  if (!request.name) {
    await showAlert(title, t("missingEndpointName"));
    return null;
  }
  if (!/^[A-Za-z0-9]+$/.test(request.name)) {
    await showAlert(title, t("invalidEndpointName"));
    return null;
  }
  if (!request.base_url) {
    await showAlert(title, t("missingEndpointUrl"));
    return null;
  }
  if (!request.api_key) {
    await showAlert(title, t("missingApiKey"));
    return null;
  }
  return request;
}

export function clearEndpointForm(elements: UiElements) {
  elements.endpointName.value = "";
  elements.baseUrl.value = "";
  elements.apiKey.value = "";
}
