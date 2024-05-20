use anyhow::Context;
use aws_sdk_s3::presigning::PresigningConfig;
use std::{sync::Arc, time::Duration};

use app_config::{AppConfig, StorageConfig};

pub async fn init_storage(conf: &AppConfig) -> Result<Arc<FileStore>, anyhow::Error> {
    let sc = &conf.storage;

    // let region_provider =
    //     aws_config::meta::region::RegionProviderChain::default_provider().or_else("us-east-1");
    // FIXME
    // let mut aws_conf = aws_config::from_env().region(region_provider);
    let mut aws_conf = aws_config::from_env();
    // let mut aws_conf = aws_config::defaults(aws_config::BehaviorVersion::latest());
    aws_conf = aws_conf.endpoint_url("http://localhost:localstack.cloud:4566");
    let aws_conf = aws_conf.load().await;

    let client = aws_sdk_s3::Client::new(&aws_conf);

    // XXX temp
    let buckets = client.list_buckets().send().await.context("Oops")?;
    println!("buckets: {:?}", buckets.buckets());

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
            Bucket::Media => &conf.bucket_media,
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
    client: aws_sdk_s3::Client,
    conf: StorageConfig,
}

impl FileStore {
    pub async fn get_upload_url(
        &self,
        bucket: Bucket,
        key: String,
    ) -> Result<UploadUrl, anyhow::Error> {
        let client = &self.client;
        let bucket = bucket.to_name(&self.conf);
        // XXX temp
        println!("bucket {}", bucket);
        let req = client
            .put_object()
            .bucket(bucket)
            .key(&key)
            .presigned(
                PresigningConfig::expires_in(Duration::from_secs(300))
                    .expect("Should be a valid duration"),
            )
            .await
            .context("Failed to generate signed URL")?;
        let url = req.uri().to_owned();
        let method = req.method().to_owned();
        let req2 = req.make_http_02x_request("");
        println!("uri: {} - headers: {:?}", req2.uri(), req2.headers());
        Ok(UploadUrl {
            bucket: bucket.clone(),
            key,
            url,
            method,
        })
    }

    pub async fn sign_url(
        &self,
        bucket: impl AsRef<str>,
        key: impl AsRef<str>,
    ) -> Result<SignedUrl, anyhow::Error> {
        let client = &self.client;
        let req = client
            .get_object()
            .bucket(bucket.as_ref())
            .key(key.as_ref())
            .presigned(
                PresigningConfig::expires_in(chrono::Duration::days(1).to_std().unwrap()).unwrap(),
            )
            .await
            .context("Failed to generate signed URL")?;
        let url = req.uri().to_owned();
        Ok(url)
    }
}
