use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CliConfigTargetKind {
    Codex,
    Claude,
    Opencode,
    Deepseek,
}

impl CliConfigTargetKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::Opencode => "opencode",
            Self::Deepseek => "deepseek",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CliConfigPreview {
    pub endpoint_type: String,
    pub target: String,
    pub files: Vec<CliConfigPreviewFile>,
    pub warnings: Vec<CliConfigPreviewWarning>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CliConfigPreviewWarning {
    OpenCodeProviderOverwrite { provider: String },
    DeepseekProviderOverwrite { provider: String },
}

#[derive(Clone, Debug, Serialize)]
pub struct CliConfigPreviewFile {
    pub file_id: String,
    pub path: String,
    pub language: String,
    pub content: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EditedCliConfig {
    pub files: Vec<EditedCliConfigFile>,
    pub selected_models: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct OpenCodeTimeoutOptions {
    pub timeout_ms: Option<u64>,
    pub header_timeout_ms: Option<u64>,
    pub chunk_timeout_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EditedCliConfigFile {
    pub file_id: String,
    pub path: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApplyCliConfigResult {
    pub baseline_id: String,
    pub baseline_path: String,
    pub endpoint_type: String,
    pub target: String,
    pub results: Vec<CliConfigWriteResult>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CliConfigWriteResult {
    pub target: String,
    pub path: String,
    pub backup_paths: Vec<String>,
    pub action: String,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RestoreSelection {
    pub target: CliConfigTargetKind,
    pub file_id: String,
    pub path: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RestoreCliConfigResult {
    pub baseline_id: String,
    pub results: Vec<CliConfigRestoreResult>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CliConfigRestoreResult {
    pub target: String,
    pub path: String,
    pub action: String,
    pub ok: bool,
    pub error: Option<String>,
}
