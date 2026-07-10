import { byId } from "../utils/dom";

export function getElements() {
  return {
    endpointType: byId<HTMLSelectElement>("endpoint-type"),
    openCodeSdkLabel: byId<HTMLLabelElement>("opencode-sdk-label"),
    openCodeSdkPackage: byId<HTMLSelectElement>("opencode-sdk-package"),
    endpointName: byId<HTMLInputElement>("endpoint-name"),
    baseUrl: byId<HTMLInputElement>("base-url"),
    apiKey: byId<HTMLInputElement>("api-key"),
    fetchTimeout: byId<HTMLInputElement>("fetch-timeout"),
    endpointFilterText: byId<HTMLInputElement>("endpoint-filter-text"),
    endpointFilterType: byId<HTMLSelectElement>("endpoint-filter-type"),
    endpointRows: byId<HTMLTableSectionElement>("endpoint-rows"),
    fetchedModelsEl: byId<HTMLDivElement>("fetched-models"),
    testPanel: byId<HTMLElement>("test-panel"),
    testType: byId<HTMLElement>("test-type"),
    testUrl: byId<HTMLElement>("test-url"),
    testKey: byId<HTMLElement>("test-key"),
    append1mLabel: byId<HTMLLabelElement>("append-1m-label"),
    append1m: byId<HTMLInputElement>("append-1m"),
    testModelsEl: byId<HTMLDivElement>("test-models"),
    testTimeout: byId<HTMLInputElement>("test-timeout"),
    resultRows: byId<HTMLTableSectionElement>("result-rows"),
    startTest: byId<HTMLButtonElement>("start-test"),
    startRealConfigTest: byId<HTMLButtonElement>("start-real-config-test"),
    stopTest: byId<HTMLButtonElement>("stop-test"),
    applyCodex: byId<HTMLButtonElement>("apply-codex"),
    applyOpenCode: byId<HTMLButtonElement>("apply-opencode"),
    removeOpenCode: byId<HTMLButtonElement>("remove-opencode"),
    applyClaude: byId<HTMLButtonElement>("apply-claude"),
    testStatus: byId<HTMLSpanElement>("test-status"),
    testLogOutput: byId<HTMLPreElement>("test-log-output"),
    testSettingsPanel: byId<HTMLElement>("test-settings-panel"),
    successKeywordInput: byId<HTMLInputElement>("success-keyword"),
    testPromptInput: byId<HTMLTextAreaElement>("test-prompt"),
    docsPanel: byId<HTMLElement>("docs-panel"),
    toast: byId<HTMLDivElement>("toast"),
  };
}

export type UiElements = ReturnType<typeof getElements>;
