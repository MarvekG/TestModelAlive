use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Snapshot of https://models.dev/api.json prepared in Cargo's output directory
/// by build.rs. The build script uses the checked-in resource as its fallback.
const EMBEDDED_METADATA: &str = include_str!(concat!(env!("OUT_DIR"), "/models_dev_api.json"));

const MODELS_DEV_URL: &str = "https://models.dev/api.json";
const CACHE_FILE_NAME: &str = "models_dev_api.json";
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);

/// Reasoning effort levels understood by OpenCode variants and model profiles.
const EFFORT_LEVELS: [&str; 6] = ["minimal", "low", "medium", "high", "xhigh", "max"];
const OFF_LEVELS: [&str; 2] = ["none", "off"];

/// Official-provider rules mirrored from the DeepSeek Harness reference
/// implementation: a model id belongs to the first provider whose token
/// prefixes the normalized id (or its bare name after the last `/`).
const OFFICIAL_PROVIDER_RULES: [(&str, &[&str]); 11] = [
    ("deepseek", &["deepseek"]),
    ("openai", &["gpt", "o1", "o3", "o4", "codex"]),
    ("xai", &["grok", "x-ai/grok", "xai/grok"]),
    ("anthropic", &["claude"]),
    ("google", &["gemini"]),
    ("mistral", &["mistral"]),
    ("cohere", &["command"]),
    ("nvidia", &["nemotron"]),
    ("meta", &["llama"]),
    ("xiaomi", &["mimo"]),
    ("alibaba", &["qwen"]),
];

/// Capabilities of one model, shaped for the DeepSeek Harness YAML schema.
#[derive(Debug, PartialEq)]
pub(crate) struct ModelProfile {
    pub context_window: Option<u64>,
    pub max_tokens: Option<u64>,
    pub input: Vec<String>,
    pub reasoning_efforts: Vec<(String, Option<String>)>,
}

#[derive(Debug, Clone)]
struct Candidate {
    provider_id: String,
    provider_key: String,
    model_key: String,
}

struct MetadataSnapshot {
    root: Value,
    resolved: BTreeMap<String, Candidate>,
}

static METADATA: RwLock<Option<Arc<MetadataSnapshot>>> = RwLock::new(None);

pub(crate) fn model_limit(model: &str) -> Option<Value> {
    let record = resolve_record(model)?;
    complete_limit(record.get("limit")?)
}

fn complete_limit(limit: &Value) -> Option<Value> {
    let context = positive_limit(limit, "context")?;
    let output = positive_limit(limit, "output")?;
    let mut value = Map::new();
    value.insert("context".to_string(), Value::from(context));
    if let Some(input) = positive_limit(limit, "input") {
        value.insert("input".to_string(), Value::from(input));
    }
    value.insert("output".to_string(), Value::from(output));
    Some(Value::Object(value))
}

/// OpenCode reasoning variants derived from the models.dev effort options,
/// e.g. `{ "none": {}, "low": { "reasoningEffort": "low" }, ... }`.
pub(crate) fn opencode_model_variants(model: &str) -> Map<String, Value> {
    resolve_record(model)
        .map(|record| opencode_variants_for_record(&record).into_iter().collect())
        .unwrap_or_default()
}

pub(crate) fn model_profile(model: &str) -> Option<ModelProfile> {
    let record = resolve_record(model)?;
    let context_window = record
        .get("limit")
        .and_then(|limit| positive_limit(limit, "context"));
    let max_tokens = record
        .get("limit")
        .and_then(|limit| positive_limit(limit, "output"));
    let mut input: Vec<String> = record
        .get("modalities")
        .and_then(|modalities| modalities.get("input"))
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| *value == "text" || *value == "image")
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if input.is_empty() {
        input = vec!["text".to_string()];
    }
    let mut reasoning_efforts: Vec<(String, Option<String>)> = effort_values(&record)
        .into_iter()
        .map(|value| {
            if OFF_LEVELS.contains(&value.as_str()) {
                ("off".to_string(), None)
            } else {
                let wire = value.clone();
                (value, Some(wire))
            }
        })
        .collect();
    if reasoning_efforts.iter().all(|(level, _)| level == "off") {
        reasoning_efforts.clear();
    }
    let has_profile =
        context_window.is_some() || max_tokens.is_some() || !reasoning_efforts.is_empty();
    has_profile.then_some(ModelProfile {
        context_window,
        max_tokens,
        input,
        reasoning_efforts,
    })
}

/// Refresh model metadata when the model-test page is opened. Network failures
/// are non-fatal: the current in-memory snapshot or embedded fallback remains
/// available to config generation.
#[tauri::command]
pub(crate) async fn refresh_model_metadata(app: tauri::AppHandle) -> bool {
    let result = tauri::async_runtime::spawn_blocking(move || {
        let cache_path = match crate::paths::store_path(&app, CACHE_FILE_NAME) {
            Ok(path) => {
                load_cache(&path);
                Some(path)
            }
            Err(err) => {
                tracing::warn!(error = %err, "could not resolve the models.dev cache path");
                None
            }
        };
        let text = download_metadata()?;
        if replace_metadata(&text) {
            if let Some(cache_path) = cache_path {
                if let Err(err) = write_cache(&cache_path, &text) {
                    tracing::warn!(error = %err, "could not store the models.dev cache");
                }
            }
            Ok(())
        } else {
            Err("downloaded models.dev metadata had no usable model records".to_string())
        }
    })
    .await;

    match result {
        Ok(Ok(())) => {
            tracing::info!("refreshed models.dev model metadata");
            true
        }
        Ok(Err(err)) => {
            tracing::warn!(error = %err, "could not refresh models.dev metadata; using fallback");
            current_snapshot();
            false
        }
        Err(err) => {
            tracing::warn!(error = %err, "models.dev refresh task failed; using fallback");
            current_snapshot();
            false
        }
    }
}

fn load_cache(path: &Path) -> bool {
    fs::read_to_string(path)
        .ok()
        .is_some_and(|text| replace_metadata(&text))
}

fn write_cache(path: &Path, text: &str) -> std::io::Result<()> {
    if fs::read_to_string(path).is_ok_and(|existing| existing == text) {
        return Ok(());
    }
    fs::write(path, text)
}

fn replace_metadata(text: &str) -> bool {
    match parse_snapshot(text) {
        Some(snapshot) => {
            store(snapshot);
            true
        }
        None => false,
    }
}

fn store(snapshot: MetadataSnapshot) {
    if let Ok(mut guard) = METADATA.write() {
        *guard = Some(Arc::new(snapshot));
    }
}

fn current_snapshot() -> Arc<MetadataSnapshot> {
    if let Some(snapshot) = METADATA.read().ok().and_then(|guard| guard.clone()) {
        return snapshot;
    }
    let snapshot = Arc::new(parse_snapshot(EMBEDDED_METADATA).unwrap_or_else(empty_snapshot));
    if let Ok(mut guard) = METADATA.write() {
        *guard = Some(snapshot.clone());
    }
    snapshot
}

fn empty_snapshot() -> MetadataSnapshot {
    MetadataSnapshot {
        root: serde_json::json!({}),
        resolved: BTreeMap::new(),
    }
}

fn parse_snapshot(text: &str) -> Option<MetadataSnapshot> {
    let root: Value = serde_json::from_str(text).ok()?;
    if !root.is_object() {
        return None;
    }
    let resolved = build_resolved_index(&root);
    if resolved.is_empty() {
        return None;
    }
    Some(MetadataSnapshot { root, resolved })
}

fn download_metadata() -> Result<String, String> {
    let agent = reqwest::blocking::Client::builder()
        .timeout(DOWNLOAD_TIMEOUT)
        .build()
        .map_err(|error| error.to_string())?;
    let response = agent
        .get(MODELS_DEV_URL)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| error.to_string())?;
    response.text().map_err(|error| error.to_string())
}

/// Index every model id to its best provider record, mirroring the reference
/// implementation: prefer the official provider, then a unique match, then the
/// smallest-capacity record as deterministic default.
fn build_resolved_index(root: &Value) -> BTreeMap<String, Candidate> {
    let mut candidates: BTreeMap<String, Vec<Candidate>> = BTreeMap::new();
    for (provider_key, provider) in root.as_object().into_iter().flatten() {
        let Some(models) = provider.get("models").and_then(Value::as_object) else {
            continue;
        };
        let provider_id = provider_identifier(provider, provider_key);
        for (model_key, model) in models {
            if model.is_object() {
                candidates
                    .entry(model_key.clone())
                    .or_default()
                    .push(Candidate {
                        provider_id: provider_id.to_string(),
                        provider_key: provider_key.clone(),
                        model_key: model_key.clone(),
                    });
            }
        }
    }
    candidates
        .into_iter()
        .filter_map(|(id, candidates)| {
            select_candidate(root, &id, &candidates).map(|candidate| (id, candidate))
        })
        .collect()
}

fn select_candidate(root: &Value, id: &str, candidates: &[Candidate]) -> Option<Candidate> {
    let official = official_provider_for_model(id);
    if let Some(official) = official {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate.provider_id == official)
        {
            return Some(candidate.clone());
        }
        if let Some((provider_key, model_key)) = lookup_in_provider(root, official, id) {
            return Some(Candidate {
                provider_id: official.to_string(),
                provider_key,
                model_key,
            });
        }
    }
    match candidates {
        [] => None,
        [only] => Some(only.clone()),
        many => many
            .iter()
            .reduce(|left, right| {
                if default_candidate_ordering(root, left, right) != std::cmp::Ordering::Greater {
                    left
                } else {
                    right
                }
            })
            .cloned(),
    }
}

/// Deterministic default ranking: smallest context first (missing last), then
/// smallest output, then provider id.
fn default_candidate_ordering(
    root: &Value,
    left: &Candidate,
    right: &Candidate,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    for field in ["context", "output"] {
        let left_value = candidate_limit(root, left, field);
        let right_value = candidate_limit(root, right, field);
        match (left_value, right_value) {
            (None, None) => continue,
            (None, Some(_)) => return Ordering::Greater,
            (Some(_), None) => return Ordering::Less,
            (Some(left), Some(right)) => {
                if left != right {
                    return left.cmp(&right);
                }
            }
        }
    }
    left.provider_id.cmp(&right.provider_id)
}

fn candidate_limit(root: &Value, candidate: &Candidate, field: &str) -> Option<u64> {
    record_in(root, candidate)?
        .get("limit")
        .and_then(|limit| positive_limit(limit, field))
}

/// Case-insensitive lookup of a model id inside one provider, trying the full
/// id and its bare name after the last `/`.
fn lookup_in_provider(root: &Value, provider_id: &str, id: &str) -> Option<(String, String)> {
    for (provider_key, provider) in root.as_object()?.iter() {
        if provider_identifier(provider, provider_key) != provider_id {
            continue;
        }
        let models = provider.get("models")?.as_object()?;
        for variant in id_variants(id) {
            if let Some((model_key, _)) = models
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(&variant))
            {
                return Some((provider_key.clone(), model_key.clone()));
            }
        }
        return None;
    }
    None
}

fn id_variants(id: &str) -> Vec<String> {
    let mut variants = vec![id.to_string()];
    if let Some((_, bare)) = id.rsplit_once('/') {
        variants.push(bare.to_string());
    }
    variants
}

fn provider_identifier<'a>(provider: &'a Value, provider_key: &'a str) -> &'a str {
    match provider.get("id").and_then(Value::as_str) {
        Some(id) if !id.is_empty() => id,
        _ => provider_key,
    }
}

fn record_in<'a>(root: &'a Value, candidate: &Candidate) -> Option<&'a Value> {
    root.get(&candidate.provider_key)?
        .get("models")?
        .get(&candidate.model_key)
}

fn resolve_record(model: &str) -> Option<Value> {
    let snapshot = current_snapshot();
    let mut candidates = [Some(model), model.rsplit_once('/').map(|(_, bare)| bare)];
    let candidate = candidates.iter_mut().find_map(|id| {
        let id = id.take()?;
        snapshot.resolved.get(id).cloned().or_else(|| {
            let lowered = id.to_ascii_lowercase();
            snapshot
                .resolved
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(&lowered))
                .map(|(_, candidate)| candidate.clone())
        })
    })?;
    record_in(&snapshot.root, &candidate).cloned()
}

fn official_provider_for_model(id: &str) -> Option<&'static str> {
    let normalized = id.to_ascii_lowercase();
    let bare = normalized.rsplit('/').next().unwrap_or(&normalized);
    OFFICIAL_PROVIDER_RULES
        .iter()
        .find(|(_, tokens)| {
            tokens
                .iter()
                .any(|token| matches_token(&normalized, token) || matches_token(bare, token))
        })
        .map(|(provider, _)| *provider)
}

fn matches_token(id: &str, token: &str) -> bool {
    id == token
        || id.starts_with(&format!("{token}-"))
        || id.starts_with(&format!("{token}/"))
        || id.starts_with(&format!("{token}."))
}

fn positive_limit(limit: &Value, field: &str) -> Option<u64> {
    limit
        .get(field)
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
}

/// First `effort`-typed reasoning option, filtered to known levels.
fn effort_values(record: &Value) -> Vec<String> {
    record
        .get("reasoning_options")
        .and_then(Value::as_array)
        .and_then(|options| {
            options
                .iter()
                .find(|option| option.get("type").and_then(Value::as_str) == Some("effort"))
        })
        .and_then(|option| option.get("values"))
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| EFFORT_LEVELS.contains(value) || OFF_LEVELS.contains(value))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn opencode_variants_for_record(record: &Value) -> Vec<(String, Value)> {
    effort_values(record)
        .into_iter()
        .map(|value| {
            if OFF_LEVELS.contains(&value.as_str()) {
                ("none".to_string(), serde_json::json!({}))
            } else {
                (
                    value.clone(),
                    serde_json::json!({ "reasoningEffort": value }),
                )
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        complete_limit, model_limit, model_profile, opencode_model_variants, parse_snapshot,
    };
    use serde_json::json;

    #[test]
    fn official_records_win_for_known_models() {
        assert_eq!(
            model_limit("deepseek-v4-flash"),
            Some(json!({ "context": 1_000_000, "output": 384_000 }))
        );
        assert_eq!(
            model_limit("claude-opus-4-6"),
            Some(json!({ "context": 1_000_000, "output": 128_000 }))
        );
        assert_eq!(
            model_limit("gpt-5.4"),
            Some(json!({ "context": 1_050_000, "input": 922_000, "output": 128_000 }))
        );
    }

    #[test]
    fn unknown_models_have_no_metadata() {
        assert_eq!(model_limit("custom-local-model"), None);
        assert_eq!(model_profile("custom-local-model"), None);
        assert!(opencode_model_variants("custom-local-model").is_empty());
    }

    #[test]
    fn incomplete_limits_are_not_emitted() {
        assert_eq!(
            complete_limit(&json!({ "context": 0, "output": 4096 })),
            None
        );
        assert_eq!(complete_limit(&json!({ "context": 128000 })), None);
    }

    #[test]
    fn invalid_snapshots_are_rejected() {
        assert!(parse_snapshot("{}").is_none());
        assert!(parse_snapshot(r#"{"error":"unavailable"}"#).is_none());
    }

    #[test]
    fn variants_follow_the_advertised_effort_levels() {
        assert_eq!(
            opencode_model_variants("gpt-5.4"),
            json!({
                "none": {},
                "low": { "reasoningEffort": "low" },
                "medium": { "reasoningEffort": "medium" },
                "high": { "reasoningEffort": "high" },
                "xhigh": { "reasoningEffort": "xhigh" }
            })
            .as_object()
            .unwrap()
            .clone()
        );
        assert_eq!(
            opencode_model_variants("deepseek-v4-flash"),
            json!({
                "low": { "reasoningEffort": "low" },
                "high": { "reasoningEffort": "high" },
                "max": { "reasoningEffort": "max" }
            })
            .as_object()
            .unwrap()
            .clone()
        );
        assert!(opencode_model_variants("minimax-m3").is_empty());
    }

    #[test]
    fn profiles_carry_capacity_input_and_reasoning() {
        let profile = model_profile("deepseek-v4-flash").expect("profile should exist");
        assert_eq!(profile.context_window, Some(1_000_000));
        assert_eq!(profile.max_tokens, Some(384_000));
        assert_eq!(profile.input, vec!["text".to_string()]);
        assert_eq!(
            profile.reasoning_efforts,
            vec![
                ("low".to_string(), Some("low".to_string())),
                ("high".to_string(), Some("high".to_string())),
                ("max".to_string(), Some("max".to_string()))
            ]
        );

        let image_profile = model_profile("claude-opus-4-6").expect("profile should exist");
        assert_eq!(
            image_profile.input,
            vec!["text".to_string(), "image".to_string()]
        );

        let toggle_only = model_profile("minimax-m3").expect("profile should exist");
        assert!(toggle_only.reasoning_efforts.is_empty());
    }

    #[test]
    fn qualified_ids_resolve_to_the_bare_model_name() {
        assert_eq!(
            model_limit("any-gateway/deepseek-v4-flash"),
            Some(json!({ "context": 1_000_000, "output": 384_000 }))
        );
    }
}
