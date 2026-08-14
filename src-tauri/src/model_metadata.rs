use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
struct ModelMetadata {
    limits: BTreeMap<String, ModelLimit>,
    opencode_variants: BTreeMap<String, Value>,
    dsh_default_input: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ModelLimit {
    context: u64,
    input: Option<u64>,
}

pub(crate) fn opencode_model_variants() -> &'static BTreeMap<String, Value> {
    &metadata().opencode_variants
}

pub(crate) fn model_limit(model: &str) -> Option<Value> {
    let limit = metadata().limits.get(model)?;
    let mut value = Map::new();
    value.insert("context".to_string(), Value::from(limit.context));
    if let Some(input) = limit.input {
        value.insert("input".to_string(), Value::from(input));
    }
    Some(Value::Object(value))
}

pub(crate) fn dsh_default_input() -> &'static [String] {
    &metadata().dsh_default_input
}

fn metadata() -> &'static ModelMetadata {
    static METADATA: OnceLock<ModelMetadata> = OnceLock::new();
    METADATA.get_or_init(|| {
        serde_json::from_str(include_str!("model_metadata.json"))
            .expect("embedded model metadata must be valid JSON")
    })
}

#[cfg(test)]
mod tests {
    use super::{dsh_default_input, model_limit, opencode_model_variants};

    #[test]
    fn embedded_metadata_contains_deepseek_v4_capabilities() {
        assert_eq!(
            model_limit("deepseek-v4-flash"),
            Some(serde_json::json!({ "context": 1_000_000 }))
        );
        assert_eq!(dsh_default_input(), ["text".to_string()]);
        assert!(opencode_model_variants().get("deepseek-v4-flash").is_some());
    }
}
