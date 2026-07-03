import type { UiElements } from "./elements";

export function openTestSettingsDialog(elements: UiElements, successKeyword: string, testPrompt: string) {
  elements.successKeywordInput.value = successKeyword;
  elements.testPromptInput.value = testPrompt;
  elements.testSettingsPanel.classList.remove("hidden");
}

export function closeTestSettingsDialog(elements: UiElements) {
  elements.testSettingsPanel.classList.add("hidden");
}

export function resetTestSettingsDialog(elements: UiElements) {
  elements.successKeywordInput.value = "OKK";
  elements.testPromptInput.value = "You must output exactly OKK and nothing else. Do not explain. Do not add punctuation.";
}
