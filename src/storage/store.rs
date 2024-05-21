//! Abstraction for object storage.
//! Mostly uses the S3 terminology because that's what I started with.
//!
use anyhow::Context as _;
use axum::body::Bytes;
use azure_storage::{shared_access_signature::service_sas::BlobSasPermissions, StorageCredentials};
use azure_storage_blobs::prelude::{BlobServiceClient, ClientBuilder, ContainerClient};
use std::{sync::Arc, time::Duration};

use app_config::{AppConfig, StorageConfig};

use crate::Route;

use super::routes::MediaUploadProxyParams;

pub async fn init_storage(conf: &AppConfig) -> Result<Arc<FileStore>, anyhow::Error> {
    let sc = &conf.storage;

    let client: BlobServiceClient = match sc.emulator {
        true => ClientBuilder::emulator().blob_service_client(),
        false => {
            let creds = StorageCredentials::access_key(
                sc.azure_storage_account.clone(),
                sc.azure_storage_access_key.clone(),
            );
            BlobServiceClient::builder(sc.azure_storage_account.clone(), creds)
                .blob_service_client()
        }
    };

    Ok(Arc::new(FileStore {
        client,
        conf: sc.clone(),
    }))
}

pub enum Bucket {
    Media,
}

impl Bucket {
    fn to_name<'a>(&self, conf: &'a StorageConfig) -> &'a String {
        match self {
            Bucket::Media => &conf.container_media,
        }
    }
}

pub type SignedUrl = String;

pub struct UploadUrl {
    pub bucket: String,
    pub key: String,
    pub url: SignedUrl,
    pub method: String,
}

#[derive(Debug)]
pub struct FileStore {
    client: BlobServiceClient,
    conf: StorageConfig,
}

impl FileStore {
    pub async fn get_upload_url(
        &self,
        bucket: Bucket,
        key: String,
    ) -> Result<UploadUrl, anyhow::Error> {
        let bucket = bucket.to_name(&self.conf).to_owned();
        if self.conf.emulator {
            let url = Route::MediaUploadProxyPut(Some(MediaUploadProxyParams {
                bucket: bucket.clone(),
                key: key.clone(),
            }))
            .as_path();
            return Ok(UploadUrl {
                method: "PUT".into(),
                url: url.into(),
                bucket,
                key,
            });
        }

        let client = &self.client;
        let blob_client = client
            .container_client(bucket.clone())
            .blob_client(key.clone());
        let perms = BlobSasPermissions {
            create: true,
            ..Default::default()
        };
        let expiry = time::OffsetDateTime::now_utc() + time::Duration::hours(1);
        let sas = blob_client
            .shared_access_signature(perms, expiry)
            .await
            .context("Failed to generate SAS")?;
        let url = blob_client
            .generate_signed_blob_url(&sas)
            .context("Failed to generate signed URL")?;
        Ok(UploadUrl {
            bucket,
            key,
            url: url.to_string(),
            method: "PUT".to_string(),
        })
    }

    pub async fn upload(
        &self,
        bucket: String,
        key: String,
        body: Bytes,
    ) -> Result<(), UploadError> {
        if !self.conf.emulator {
            return Err(UploadError::NotSupported);
        }

        let client = &self.client;
        let container_client = client.container_client(bucket);
        self.ensure_container_exists(&container_client)
            .await
            .context("Failed to ensure container exists")?;

        let blob_client = container_client.blob_client(key);
        blob_client
            .put_block_blob(body)
            .await
            .context("Failed to upload blob")?;
        Ok(())
    }

    pub async fn sign_url(
        &self,
        bucket: impl AsRef<str>,
        key: impl AsRef<str>,
    ) -> Result<SignedUrl, anyhow::Error> {
        let client = &self.client;
        let blob_client = client
            .container_client(bucket.as_ref())
            .blob_client(key.as_ref());
        let perms = BlobSasPermissions {
            read: true,
            ..Default::default()
        };
        let expiry = time::OffsetDateTime::now_utc() + time::Duration::days(30);
        let sas = blob_client
            .shared_access_signature(perms, expiry)
            .await
            .context("Failed to generate SAS")?;
        let url = blob_client
            .generate_signed_blob_url(&sas)
            .context("Failed to generate signed URL")?;
        Ok(url.to_string())
    }

    async fn ensure_container_exists(
        &self,
        container_client: &ContainerClient,
    ) -> Result<(), azure_storage::Error> {
        match container_client.create().await {
            Err(err) => match err.kind() {
                azure_storage::ErrorKind::HttpResponse {
                    error_code: Some(ref code),
                    ..
                } => {
                    if code == "ContainerAlreadyExists" {
                        Ok(())
                    } else {
                        Err(err)
                    }
                }
                _ => Err(err),
            },
            Ok(_) => Ok(()),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum UploadError {
    #[error("{0}")]
    Other(#[from] anyhow::Error),
    #[error("Direct upload is not supported")]
    NotSupported,
}
