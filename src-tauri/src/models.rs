use serde::{Deserialize, Serialize};

pub const DEFAULT_SUCCESS_KEYWORD: &str = "OKK";
pub const DEFAULT_PROMPT: &str =
    "You must output exactly OKK and nothing else. Do not explain. Do not add punctuation.";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SavedEndpoint {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub endpoint_type: String,
    pub base_url: String,
    pub api_key: String,
    pub models: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddEndpointRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub endpoint_type: String,
    pub base_url: String,
    pub api_key: String,
    pub models: Vec<String>,
    pub overwrite: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct FetchModelsRequest {
    #[serde(rename = "type")]
    pub endpoint_type: String,
    pub base_url: String,
    pub api_key: String,
    pub timeout: u64,
}

#[derive(Debug, Deserialize)]
pub struct TestModelsRequest {
    pub endpoint: SavedEndpoint,
    pub models: Vec<String>,
    pub timeout: u64,
    pub append_1m: bool,
    pub prompt: String,
    pub success_keyword: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TestSettings {
    pub prompt: String,
    pub success_keyword: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TestResult {
    pub model: String,
    pub status: String,
    pub seconds: f64,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct LogEvent {
    pub message: String,
    pub stream: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind")]
pub enum TestMessage {
    #[serde(rename = "log")]
    Log { message: String, stream: bool },
    #[serde(rename = "result")]
    Result { result: TestResult },
    #[serde(rename = "finished")]
    Finished,
}
