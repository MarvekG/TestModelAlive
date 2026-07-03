import type { Language } from "./i18n";
import type { SavedEndpoint, TestResult } from "./types";

export function createInitialState() {
  return {
    endpoints: [] as SavedEndpoint[],
    fetchedModels: [] as string[],
    fetchedSelection: new Set<string>(),
    selectedEndpointId: "",
    checkedEndpointIds: new Set<string>(),
    testEndpoint: null as SavedEndpoint | null,
    testSelection: new Set<string>(),
    testResults: [] as TestResult[],
    testRunning: false,
    testLogChunks: [] as string[],
    testPrompt: "You must output exactly OKK and nothing else. Do not explain. Do not add punctuation.",
    successKeyword: "OKK",
    language: (localStorage.getItem("language") === "en" ? "en" : "zh") as Language,
  };
}
