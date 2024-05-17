use anyhow::Context;
use axum::http::Method;
use object_store::signer::Signer;
use std::sync::Arc;
use url::Url;

use app_config::AppConfig;

const BUCKET: &str = "media";

pub fn init_storage(conf: &AppConfig) -> Result<Arc<FileStore>, anyhow::Error> {
    let sc = &conf.storage;
    let client = object_store::aws::AmazonS3Builder::new()
        .with_access_key_id(sc.access_key_id.clone())
        .with_secret_access_key(sc.secret_access_key.clone())
        .with_region(sc.region.clone())
        .with_endpoint(sc.endpoint.clone())
        .with_allow_http(sc.allow_http)
        .with_bucket_name(BUCKET)
        .build()
        .context("Failed to build S3 client")?;
    Ok(Arc::new(FileStore { client }))
}

#[derive(Debug)]
pub struct FileStore {
    client: object_store::aws::AmazonS3,
}

impl FileStore {
    pub async fn get_upload_url(&self, path: impl AsRef<str>) -> Result<Url, anyhow::Error> {
        let client = &self.client;
        let url = client
            .signed_url(
                Method::PUT,
                &path.as_ref().into(),
                chrono::Duration::hours(1)
                    .to_std()
                    .expect("Invalid duration"),
            )
            .await
            .context("Failed to generate signed URL")?;
        Ok(url)
    }

    pub async fn sign_url(&self, path: impl AsRef<str>) -> Result<Url, anyhow::Error> {
        let client = &self.client;
        let url = client
            .signed_url(
                Method::GET,
                &path.as_ref().into(),
                chrono::Duration::days(1)
                    .to_std()
                    .expect("Invalid duration"),
            )
            .await
            .context("Failed to generate signed URL")?;
        Ok(url)
    }
}
