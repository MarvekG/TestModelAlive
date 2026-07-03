export type EndpointType = "codex" | "claude";
export type CliConfigTargetKind = "codex" | "claude" | "opencode";

export interface SavedEndpoint {
  id: string;
  type: EndpointType;
  base_url: string;
  api_key: string;
  models: string[];
}

export interface TestResult {
  model: string;
  status: string;
  seconds: number;
  detail: string;
}

export interface TestMessage {
  kind: "log" | "result" | "finished";
  message?: string;
  stream?: boolean;
  result?: TestResult;
}

export interface TestSettings {
  prompt: string;
  success_keyword: string;
}

export interface CliConfigPreview {
  endpoint_type: string;
  target: CliConfigTargetKind;
  files: CliConfigPreviewFile[];
}

export interface CliConfigPreviewFile {
  file_id: string;
  path: string;
  language: string;
  content: string;
}

export interface EditedCliConfig {
  selected_models: string[];
  files: { file_id: string; path: string; content: string }[];
}

export interface ApplyCliConfigResult {
  baseline_id: string;
  baseline_path: string;
  endpoint_type: string;
  target: CliConfigTargetKind;
  results: CliConfigWriteResult[];
}

export interface CliConfigWriteResult {
  target: string;
  path: string;
  backup_paths: string[];
  action: string;
  ok: boolean;
  error?: string | null;
}

export interface CliConfigBaselineItem {
  target: CliConfigTargetKind;
  file_id: string;
  path: string;
  existed_before: boolean;
  created_at: string;
}

export interface RestoreSelection {
  target: CliConfigTargetKind;
  file_id: string;
  path: string;
}

export interface RestoreCliConfigResult {
  baseline_id: string;
  results: { target: string; path: string; action: string; ok: boolean; error?: string | null }[];
}
