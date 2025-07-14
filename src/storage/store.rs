//! Abstraction for object storage.
//! Mostly uses the S3 terminology because that's what I started with.
//!
use anyhow::Context;
use axum::body::Bytes;
use azure_storage::{
    shared_access_signature::service_sas::BlobSasPermissions, CloudLocation, StorageCredentials,
};
use azure_storage_blobs::prelude::{BlobServiceClient, ClientBuilder, ContainerClient};
use futures::stream::StreamExt as _;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

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
            let account = sc.azure_storage_account.clone();
            let loc = match &sc.azure_storage_endpoint {
                None => CloudLocation::Public { account },
                Some(endpoint) => CloudLocation::Custom {
                    account,
                    uri: endpoint.clone(),
                },
            };
            ClientBuilder::with_location(loc, creds).blob_service_client()
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
    pub fn to_name<'a>(&self, conf: &'a StorageConfig) -> &'a String {
        match self {
            Bucket::Media => &conf.container_media,
        }
    }
}

pub type SignedUrl = String;
pub type FileKey = String;

#[derive(Debug)]
pub struct UploadUrl {
    pub bucket: String,
    pub key: FileKey,
    pub url: SignedUrl,
    pub method: String,
    pub headers: HashMap<String, String>,
}

#[derive(Debug)]
pub struct FileStore {
    client: BlobServiceClient,
    pub conf: StorageConfig,
}

impl FileStore {
    pub async fn list_containers(&self) -> Result<Vec<String>, anyhow::Error> {
        let mut r = self.client.list_containers().into_stream();
        let mut containers: Vec<String> = Vec::new();
        while let Some(page) = r.next().await {
            let page = page.context("Failed to query containers")?;
            for container in page.containers {
                containers.push(container.name);
            }
        }

        Ok(containers)
    }

    pub async fn list_files(&self, bucket: &Bucket) -> Result<HashSet<FileKey>, anyhow::Error> {
        let bucket = bucket.to_name(&self.conf).to_owned();
        let container_client = self.client.container_client(bucket);
        let mut file_stream = container_client.list_blobs().into_stream();

        let mut files: HashSet<FileKey> = HashSet::new();
        while let Some(page) = file_stream.next().await {
            let page = page.context("Failed to list files")?;
            for blob in page.blobs.blobs() {
                files.insert(blob.name.clone());
            }
        }

        Ok(files)
    }

    pub async fn delete_file(&self, bucket: &Bucket, key: &str) -> Result<(), anyhow::Error> {
        let bucket = bucket.to_name(&self.conf).to_owned();
        let container_client = self.client.container_client(&bucket);
        let blob_client = container_client.blob_client(key);
        blob_client
            .delete()
            .await
            .with_context(|| format!("Failed to delete file: {bucket}/{key}"))?;

        Ok(())
    }

    pub async fn get_upload_url(
        &self,
        bucket: Bucket,
        key: FileKey,
    ) -> Result<UploadUrl, anyhow::Error> {
        let bucket = bucket.to_name(&self.conf).to_owned();
        if self.conf.emulator {
            let url = Route::MediaUploadProxyPut(Some(&MediaUploadProxyParams {
                bucket: bucket.clone(),
                key: key.clone(),
            }))
            .as_path();
            return Ok(UploadUrl {
                method: "PUT".into(),
                url: url.into(),
                headers: Default::default(),
                bucket,
                key,
            });
        }

        let client = &self.client;
        let blob_client = client
            .container_client(bucket.clone())
            .blob_client(key.clone());

        let expiry = time::OffsetDateTime::now_utc() + time::Duration::hours(12);
        let perms = BlobSasPermissions {
            create: true,
            ..Default::default()
        };

        let sas = blob_client
            .shared_access_signature(perms, expiry)
            .await
            .context("Failed to generate SAS")?;
        let url = blob_client
            .generate_signed_blob_url(&sas)
            .context("Failed to generate signed URL")?;

        let headers = HashMap::from([("x-ms-blob-type".to_string(), "BlockBlob".to_string())]);
        Ok(UploadUrl {
            bucket,
            key,
            headers,
            url: url.to_string(),
            method: "PUT".to_string(),
        })
    }

    pub async fn upload(
        &self,
        bucket: String,
        key: FileKey,
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

        // Use a semi-constant expiry to leverage browser caches.
        let start = time::OffsetDateTime::now_utc()
            .replace_day(1)
            .unwrap()
            .replace_time(time::Time::from_hms(0, 0, 0).unwrap());
        let expiry = start + time::Duration::days(30);

        let perms = BlobSasPermissions {
            read: true,
            ..Default::default()
        };
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
