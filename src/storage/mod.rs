use anyhow::Context;
use axum::http::Method;
use object_store::signer::Signer;
use std::env::current_dir;
use std::fs::create_dir_all;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;
use url::Url;

use app_config::{AppConfig, LocalStorageConfig, LocalStorageRootUrl};

use self::routes::LocalFileStoreRoute;

pub mod routes;

pub fn init_storage(conf: &AppConfig) -> Result<Arc<FileStore>, anyhow::Error> {
    match &conf.storage {
        app_config::StorageConfig::Local(LocalStorageConfig { root_dir, root_url }) => {
            info!("Using storage: local");
            let path = current_dir().unwrap().join(root_dir);
            create_dir_all(&path).context(format!(
                "Failed to create local storage directory at {:?}",
                &path
            ))?;
            let o = object_store::local::LocalFileSystem::new_with_prefix(path)?;
            Ok(Arc::new(FileStore(FileStoreInner::Local(
                o,
                root_url.clone(),
            ))))
        }
        app_config::StorageConfig::Aws {} => {
            info!("Using storage: AWS");
            let o = object_store::aws::AmazonS3Builder::from_env().build()?;
            Ok(Arc::new(FileStore(FileStoreInner::R2(o))))
        }
    }
}

/// Unfortunately can't just have a `dyn object_store::ObjectStore`, because that
/// doesn't let us use the [object_store::signer::Signer] trait :/, since only some
/// object stores implement it.
pub struct FileStore(FileStoreInner);

impl FileStore {
    pub async fn get_upload_url(
        &self,
        path: impl AsRef<str>,
        expires_in: Duration,
    ) -> Result<String, anyhow::Error> {
        match &self.0 {
            FileStoreInner::Local(_, root_url) => Ok(LocalFileStoreRoute::url_for(root_url, path)),
            FileStoreInner::R2(store) => {
                let url = store
                    .signed_url(Method::PUT, &path.as_ref().into(), expires_in)
                    .await
                    .context("Failed to generate signed URL")?;
                // Convert to string because it's not convenient to return a URL for the local driver above.
                Ok(url.to_string())
            }
        }
    }
}

enum FileStoreInner {
    Local(object_store::local::LocalFileSystem, LocalStorageRootUrl),
    R2(object_store::aws::AmazonS3),
}
