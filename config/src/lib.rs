use std::env::current_dir;

use anyhow::Context;
use config::{Config, Environment};
use serde::Deserialize;
use thiserror::Error;
use tracing::{info, warn};

pub fn load_env() -> Result<(), anyhow::Error> {
    let cwd = current_dir().context("Failed to access current directory")?;
    let dotenv_path = cwd.join(".env");
    if dotenv_path.exists() {
        dotenv::from_path(&dotenv_path)?;
        let p = dotenv_path.to_string_lossy();
        info!("Loaded environment from: {p}");
    } else {
        let p = dotenv_path.to_string_lossy();
        warn!("No environment file found at: {p}");
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_file: String,
    pub storage: StorageConfig,
}

#[derive(Clone, Debug)]
pub struct LocalStorageRootUrl(pub String);

#[derive(Clone, Debug)]
pub struct LocalStorageConfig {
    pub root_dir: String,
    pub root_url: LocalStorageRootUrl,
}

#[derive(Clone, Debug)]
pub enum StorageConfig {
    Local(LocalStorageConfig),
    /// Rest of the values are loaded from [object_store::aws::AmazonS3Builder::from_env]
    Aws,
}

impl AppConfig {
    pub fn from_env() -> Result<AppConfig, ConfigError> {
        let conf_builder = Config::builder()
            .add_source(Environment::with_prefix("app").try_parsing(true))
            .build()
            .expect("Failed to setup config builder.");

        let conf_pre = conf_builder.clone().try_deserialize::<AppConfigPre>()?;

        let storage: StorageConfig = match conf_pre.storage {
            StorageType::Local => {
                let storage_conf = conf_builder
                    .clone()
                    .try_deserialize::<LocalStorageConfigEnv>()?;
                StorageConfig::Local(LocalStorageConfig {
                    root_dir: storage_conf.storage_local_path,
                    root_url: LocalStorageRootUrl(storage_conf.storage_local_url),
                })
            }
            StorageType::Aws => StorageConfig::Aws {},
        };

        let conf = AppConfig {
            database_file: conf_pre.database_file,
            storage: storage,
        };
        Ok(conf)
    }

    pub fn database_url(&self) -> String {
        format!("sqlite://{}?mode=rwc", self.database_file)
    }
}

#[derive(Error, Debug)]
#[error("Invalid config:\n{0:#?}")]
pub struct ConfigError(#[from] config::ConfigError);

// Looks like there's no way to do discriminated unions with config-rs?
#[derive(Clone, Debug, Deserialize)]
struct AppConfigPre {
    pub database_file: String,
    pub storage: StorageType,
}

#[derive(Clone, Debug, Deserialize)]
enum StorageType {
    Local,
    Aws,
}

#[derive(Clone, Debug, Deserialize)]
struct LocalStorageConfigEnv {
    /// Like Django's [MEDIA_ROOT](https://docs.djangoproject.com/en/5.0/ref/settings/#media-root).
    pub storage_local_path: String,
    /// Like Django's [MEDIA_URL](https://docs.djangoproject.com/en/5.0/ref/settings/#media-url)
    pub storage_local_url: String,
}
