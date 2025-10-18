use std::collections::HashMap;

use anyhow::Context;

type ManifestData = HashMap<String, String>;

#[derive(Debug)]
pub struct AssetManifest {
    url_base: String,
    manifest: ManifestData,
}

impl AssetManifest {
    pub async fn load(url_base: String, manifest_path: &str) -> Result<Self, anyhow::Error> {
        let manifest = load_manifest(manifest_path).await?;
        Ok(Self { url_base, manifest })
    }

    pub fn get_href(&self, asset_key: &str) -> Option<String> {
        self.manifest
            .get(asset_key)
            .map(|value| format!("{}/{}", self.url_base, value))
    }
}

async fn load_manifest(path: &str) -> Result<ManifestData, anyhow::Error> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read manifest file at {path}."))?;
    let manifest: ManifestData = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to deserialize manifest:\n{raw}"))?;
    Ok(manifest)
}
