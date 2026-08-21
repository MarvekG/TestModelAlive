use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const MODELS_DEV_URL: &str = "https://models.dev/api.json";
const RESOURCE_DIR: &str = "resources";
const RESOURCE_FILE: &str = "models_dev_api.json";
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);

fn main() {
    refresh_models_dev_metadata();
    tauri_build::build();
}

/// Download a fresh copy of the models.dev metadata at build time. When the
/// download fails (offline, proxy issues), keep the previously downloaded file
/// as fallback so builds stay reproducible and work without network access.
fn refresh_models_dev_metadata() {
    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by cargo"),
    );
    let fallback = manifest_dir.join(RESOURCE_DIR).join(RESOURCE_FILE);
    let destination = PathBuf::from(
        std::env::var("OUT_DIR").expect("OUT_DIR must be set by cargo for build scripts"),
    )
    .join(RESOURCE_FILE);
    println!(
        "cargo:rerun-if-changed={}",
        fallback.display().to_string().replace('\\', "/")
    );

    match download_models_dev_metadata() {
        Ok(body) if metadata_is_valid(&body) => {
            if let Err(error) = write_if_changed(&destination, &body) {
                println!("cargo:warning=failed to store models.dev metadata: {error}");
                ensure_output_exists(&destination, &fallback);
            }
        }
        Ok(_) => {
            println!("cargo:warning=models.dev metadata download had no usable model records");
            ensure_output_exists(&destination, &fallback);
        }
        Err(error) => {
            if destination.exists() {
                println!(
                    "cargo:warning=could not refresh models.dev metadata ({error}); using the existing build snapshot at {}",
                    destination.display()
                );
            } else {
                ensure_output_exists(&destination, &fallback);
                println!(
                    "cargo:warning=could not download models.dev metadata ({error}); using the checked-in fallback"
                );
            }
        }
    }
}

fn metadata_is_valid(body: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value.as_object().map(|providers| {
                providers.values().any(|provider| {
                    provider
                        .get("models")
                        .and_then(serde_json::Value::as_object)
                        .is_some_and(|models| !models.is_empty())
                })
            })
        })
        .unwrap_or(false)
}

fn ensure_output_exists(destination: &Path, fallback: &Path) {
    if destination.exists() {
        return;
    }
    let body = fs::read(fallback).unwrap_or_else(|error| {
        println!(
            "cargo:warning=failed to read models.dev fallback at {} ({error}); embedding an empty placeholder",
            fallback.display()
        );
        b"{}".to_vec()
    });
    if let Err(error) = write_if_changed(destination, &body) {
        panic!("failed to create models.dev build snapshot: {error}");
    }
}

fn write_if_changed(destination: &Path, body: &[u8]) -> std::io::Result<()> {
    if fs::read(destination).is_ok_and(|existing| existing == body) {
        return Ok(());
    }
    fs::write(destination, body)
}

fn download_models_dev_metadata() -> Result<Vec<u8>, String> {
    let agent = reqwest::blocking::Client::builder()
        .timeout(DOWNLOAD_TIMEOUT)
        .build()
        .map_err(|error| error.to_string())?;
    let response = agent
        .get(MODELS_DEV_URL)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| error.to_string())?;
    response
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(|error| error.to_string())
}
